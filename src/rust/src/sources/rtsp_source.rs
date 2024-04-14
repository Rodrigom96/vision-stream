use gst::prelude::*;
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use crate::core::appsink::pull_appsink_image;
use crate::core::pipeline::Pipeline;
use crate::core::source_bins::RtspBin;
use crate::errors::{Error, GstMissingElementError};
use crate::image::Image;

#[pyclass]
pub struct RtspSource {
    pipeline: Pipeline,
    rtspbin: RtspBin,
    last_image: Arc<Mutex<Option<Image>>>,
}

#[pymethods]
impl RtspSource {
    #[new]
    pub fn new(uri: &str, username: Option<&str>, password: Option<&str>) -> Result<Self, Error> {
        let pipeline = Pipeline::new(uri);

        // crate pieline elements
        let rtspbin = RtspBin::new(uri, username, password)?;
        let videoconvert = match gst::ElementFactory::make("nvvideoconvert").build() {
            Ok(e) => e,
            Err(_) => gst::ElementFactory::make("videoconvert")
                .build()
                .map_err(|_| GstMissingElementError("videoconvert"))?,
        };
        let capsfilter = gst::ElementFactory::make("capsfilter")
            .build()
            .map_err(|_| GstMissingElementError("capsfilter"))?;
        let appsink = gst_app::AppSink::builder()
            .max_buffers(1)
            .drop(true)
            .sync(false)
            .build();

        // config capsfilter
        let caps = gst::Caps::builder("video/x-raw")
            .field("format", "BGR")
            .build();
        capsfilter.set_property("caps", &caps);

        // add and link elements
        pipeline.add(&rtspbin.bin)?;
        pipeline.add_many([&videoconvert, &capsfilter, appsink.upcast_ref()])?;
        rtspbin.bin.link(&videoconvert)?;
        videoconvert.link(&capsfilter)?;
        capsfilter.link(&appsink)?;

        let last_image = Arc::new(Mutex::new(None));
        let last_image_clone = Arc::clone(&last_image);
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let mut img = last_image_clone.lock().unwrap();
                    *img = pull_appsink_image(appsink);

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        pipeline.set_state(gst::State::Playing)?;

        Ok(Self {
            pipeline,
            rtspbin,
            last_image,
        })
    }

    fn read(&mut self) -> Option<Image> {
        self.last_image.lock().unwrap().take()
    }

    fn is_reconnecting(&self) -> bool {
        self.rtspbin.is_reconnecting()
    }
}

impl Drop for RtspSource {
    fn drop(&mut self) {
        self.pipeline
            .set_state(gst::State::Null)
            .expect("Cant set pipeline state to null");
    }
}
