use gst::prelude::*;

use crate::core::gst_common::add_bin_ghost_pad;
use crate::errors::{Error, GstMissingElementError};

fn pad_add_handler(src: &gst::Element, src_pad: &gst::Pad, sink: &gst::Element) {
    log::info!(
        "Received new pad {} from {} to sink {}",
        src_pad.name(),
        src.name(),
        sink.name()
    );

    let sink_pad = sink
        .static_pad("sink")
        .expect("Failed to get static sink pad from convert");
    if sink_pad.is_linked() {
        log::warn!("{} already linked. Ignoring.", sink.name());
        return;
    }
    let new_pad_caps = match src_pad.current_caps() {
        Some(cap) => cap,
        None => src_pad.query_caps(None),
    };
    let new_pad_struct = new_pad_caps
        .structure(0)
        .expect("Failed to get first structure of caps.");
    let new_pad_type = new_pad_struct.name();
    log::debug!("Received pad type {} from {}", new_pad_type, src.name());

    let res = src_pad.link(&sink_pad);
    if res.is_err() {
        log::error!("Type is {} but link failed.", new_pad_type);
    } else {
        log::debug!("Link succeeded (type {}).", new_pad_type);
    }
}

pub struct RtspBin {
    pub bin: gst::Bin,
}

impl RtspBin {
    pub fn new(uri: &str, username: Option<&str>, password: Option<&str>) -> Result<Self, Error> {
        let bin = gst::Bin::new();

        let rtspsrc = gst::ElementFactory::make("rtspsrc")
            .build()
            .map_err(|_| GstMissingElementError("rtspsrc"))?;
        let decodebin = gst::ElementFactory::make("decodebin")
            .build()
            .map_err(|_| GstMissingElementError("decodebin"))?;
        let queue = gst::ElementFactory::make("queue")
            .build()
            .map_err(|_| GstMissingElementError("queue"))?;

        // config rtsp src
        rtspsrc.set_property("location", &uri);
        rtspsrc.set_property("latency", 100_u32);
        rtspsrc.set_property("drop-on-latency", true);
        if let Some(username) = username {
            rtspsrc.set_property("user-id", username);
        }
        if let Some(password) = password {
            rtspsrc.set_property("user-pw", password);
        }

        // add elements to bin
        bin.add_many(&[&rtspsrc, &decodebin, &queue])?;

        // add bin sink ghostpad
        add_bin_ghost_pad(&bin, &queue, "src")?;

        rtspsrc.connect("select-stream", false, move |args| {
            let caps = args[2].get::<gst::Caps>().unwrap();
            let caps_struct = caps.structure(0).expect("Failed to get structure of caps.");
            let media: String = caps_struct
                .get("media")
                .expect("error on get struct \"media\"");

            let is_video = media == "video";
            if !is_video {
                return Some(false.to_value());
            }

            Some(true.to_value())
        });

        let decodebin_week = decodebin.downgrade();
        rtspsrc.connect_pad_added(move |src, src_pad| {
            let decodebin = decodebin_week.upgrade().unwrap();
            pad_add_handler(src, src_pad, &decodebin);
        });
        let queue_week = queue.downgrade();
        decodebin.connect_pad_added(move |src, src_pad| {
            let queue = queue_week.upgrade().unwrap();
            pad_add_handler(src, src_pad, &queue);
        });

        Ok(Self { bin })
    }
}
