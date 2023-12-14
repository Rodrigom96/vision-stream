use gst::prelude::*;
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use crate::core::appsink::pull_appsink_image;
use crate::image::Image;

#[pyclass]
pub struct TestSource {
    pipeline: gst::Pipeline,
    last_image: Arc<Mutex<Option<Image>>>,
}

#[pymethods]
impl TestSource {
    #[new]
    pub fn new() -> Self {
        gst::init().unwrap();

        let pipeline = gst::Pipeline::new();

        let src = gst::ElementFactory::make("videotestsrc")
            .build()
            .expect("Fail create videotestsrc");
        let videoconvert = gst::ElementFactory::make("videoconvert")
            .build()
            .expect("Fail create videoconvert");
        let capsfilter = gst::ElementFactory::make("capsfilter")
            .build()
            .expect("Fail create capsfilter");
        let appsink = gst_app::AppSink::builder()
            .max_buffers(1)
            .drop(true)
            .build();

        let caps = gst::Caps::builder("video/x-raw")
            .field("format", "BGR")
            .build();
        capsfilter.set_property("caps", &caps);

        pipeline
            .add_many([&src, &videoconvert, &capsfilter, appsink.upcast_ref()])
            .unwrap();
        src.link(&videoconvert).unwrap();
        videoconvert.link(&capsfilter).unwrap();
        capsfilter.link(&appsink).unwrap();

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

        pipeline.set_state(gst::State::Playing).unwrap();

        Self {
            pipeline,
            last_image,
        }
    }

    fn read(&mut self) -> Option<Image> {
        self.last_image.lock().unwrap().take()
    }
}

impl Drop for TestSource {
    fn drop(&mut self) {
        self.pipeline.set_state(gst::State::Null).unwrap();
    }
}
