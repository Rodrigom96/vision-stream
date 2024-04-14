use std::sync::{Arc, Mutex};
use std::time::Instant;

use gst::prelude::*;

use crate::core::bin_connection_manager::BinConectionManager;
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

struct Context {
    depay: Option<gst::Element>,
    parser: Option<gst::Element>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            depay: None,
            parser: None,
        }
    }
}

pub struct RtspBin {
    pub bin: gst::Bin,
    pub connection_manager: Arc<BinConectionManager>,
}

impl RtspBin {
    pub fn new(uri: &str, username: Option<&str>, password: Option<&str>) -> Result<Self, Error> {
        let bin = gst::Bin::new();
        let connection_manager = Arc::new(BinConectionManager::new(&bin));

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
        rtspsrc.set_property("location", uri);
        rtspsrc.set_property("latency", 100_u32);
        rtspsrc.set_property("drop-on-latency", true);
        if let Some(username) = username {
            rtspsrc.set_property("user-id", username);
        }
        if let Some(password) = password {
            rtspsrc.set_property("user-pw", password);
        }

        // add elements to bin
        bin.add_many([&rtspsrc, &decodebin, &queue])?;

        // add bin sink ghostpad
        add_bin_ghost_pad(&bin, &queue, "src")?;

        let ctx = Arc::new(Mutex::new(Context::new()));

        let ctx_clone = ctx.clone();
        let bin_week = bin.downgrade();
        let decodebin_week = decodebin.downgrade();
        rtspsrc.connect("select-stream", false, move |args| {
            let caps = args[2].get::<gst::Caps>().unwrap();
            let caps_struct = caps.structure(0).expect("Failed to get structure of caps.");
            let media: String = caps_struct
                .get("media")
                .expect("error on get struct \"media\"");
            let encoding_name: String = caps_struct
                .get("encoding-name")
                .expect("error on get struct \"encoding-name\"");

            let is_video = media == "video";
            if !is_video {
                return Some(false.to_value());
            }

            // get and lock decoder
            let mut ctx = ctx_clone.lock().unwrap();

            // Create and add depay and parser if not created yet
            if ctx.depay.is_none() {
                let (depay, parser) = match encoding_name.as_str() {
                    "H264" => {
                        let depay = gst::ElementFactory::make("rtph264depay")
                            .build()
                            .expect("Cant create \"rtph264depay\" element");
                        let parser = gst::ElementFactory::make("h264parse")
                            .build()
                            .expect("Cant create \"h264parse\" element");
                        (depay, parser)
                    }
                    "H265" => {
                        let depay = gst::ElementFactory::make("rtph265depay")
                            .build()
                            .expect("Cant create \"rtph265depay\" element");
                        let parser = gst::ElementFactory::make("h265parse")
                            .build()
                            .expect("Cant create \"h265parse\" element");
                        (depay, parser)
                    }
                    _ => {
                        log::warn!("{} not supported", encoding_name);
                        return Some(false.to_value());
                    }
                };
                // add elements to bin
                bin_week
                    .upgrade()
                    .unwrap()
                    .add_many(&[&depay, &parser])
                    .expect("Cant add depay and parser");

                // link elements
                depay.link(&parser).expect("Cant link depay with parser");
                let decodebin = decodebin_week.upgrade().unwrap();
                parser
                    .link(&decodebin)
                    .expect("Cant link parser with decodebin");

                // sync elements with pipeline
                depay
                    .sync_state_with_parent()
                    .expect("Depay, Cant sync state with parent");
                parser
                    .sync_state_with_parent()
                    .expect("Parser, Cant sync state with parent");

                // store depay on decoder
                ctx.depay = Some(depay);
                ctx.parser = Some(parser);
            }

            Some(true.to_value())
        });

        let ctx_clone = ctx.clone();
        let connection_manager_clone = Arc::downgrade(&connection_manager);
        rtspsrc.connect_pad_added(move |src, src_pad| {
            let connection_manager_clone = connection_manager_clone.upgrade().unwrap();
            connection_manager_clone.setup_src_pad(src_pad);
            pad_add_handler(
                src,
                src_pad,
                ctx_clone.lock().unwrap().depay.as_ref().unwrap(),
            );
        });
        let queue_week = queue.downgrade();
        decodebin.connect_pad_added(move |src, src_pad| {
            let queue = queue_week.upgrade().unwrap();
            pad_add_handler(src, src_pad, &queue);
        });

        Ok(Self {
            bin,
            connection_manager,
        })
    }

    pub fn is_reconnecting(&self) -> bool {
        self.connection_manager.is_reconnecting()
    }
}

impl Drop for RtspBin {
    fn drop(&mut self) {
        log::debug!("Drop RtspBin");
    }
}
