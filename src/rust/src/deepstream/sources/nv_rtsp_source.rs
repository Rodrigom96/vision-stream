use gst::prelude::*;
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use deepstream_sys::nvbufsurface::NvBufSurface;

use crate::core::source_bins::RtspBin;
use crate::deepstream::cuda_image::CudaImage;
use crate::errors::{Error, GstMissingElementError};

pub fn pull_cuda_image(appsink: &gst_app::AppSink) -> Option<CudaImage> {
    let sample = appsink.pull_sample().unwrap();

    let buffer = sample.buffer().unwrap();
    let map = buffer.map_readable().unwrap();

    let surface = unsafe { &*(map.as_slice().as_ptr() as *const NvBufSurface) };
    let cuda_image = CudaImage::copy_from(surface);

    Some(cuda_image)
}

#[pyclass]
pub struct NvRtspSource {
    pipeline: gst::Pipeline,
    last_cuda_image: Arc<Mutex<Option<CudaImage>>>,
}

#[pymethods]
impl NvRtspSource {
    #[new]
    pub fn new(uri: &str) -> Result<Self, Error> {
        let pipeline = gst::Pipeline::new();

        // crate pieline elements
        let rtspbin = RtspBin::new(uri, None, None)?;
        let videoconvert = gst::ElementFactory::make("nvvideoconvert")
            .build()
            .map_err(|_| GstMissingElementError("nvvideoconvert"))?;
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
            .field("format", "RGBA")
            .features(["memory:NVMM"])
            .build();
        capsfilter.set_property("caps", &caps);

        // add and link elements
        pipeline.add(&rtspbin.bin)?;
        pipeline.add_many([&videoconvert, &capsfilter, appsink.upcast_ref()])?;
        rtspbin.bin.link(&videoconvert)?;
        videoconvert.link(&capsfilter)?;
        capsfilter.link(&appsink)?;

        let last_cuda_image = Arc::new(Mutex::new(None));
        let last_cuda_image_clone = Arc::clone(&last_cuda_image);
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let mut img = last_cuda_image_clone.lock().unwrap();
                    *img = pull_cuda_image(appsink);

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        pipeline.set_state(gst::State::Playing)?;

        Ok(Self {
            pipeline,
            last_cuda_image,
        })
    }

    fn read(&mut self) -> Option<CudaImage> {
        self.last_cuda_image.lock().unwrap().take()
    }
}

impl Drop for NvRtspSource {
    fn drop(&mut self) {
        self.pipeline
            .set_state(gst::State::Null)
            .expect("Cant set pipeline state to null");
    }
}
