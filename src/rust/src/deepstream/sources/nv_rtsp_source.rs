use gst::prelude::*;
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use deepstream_sys::nvbufsurface::NvBufSurface;

use crate::core::pipeline::Pipeline;
use crate::core::source_bins::RtspBin;
use crate::cuda::image::CudaImage;
use crate::errors::{Error, GstMissingElementError};

pub fn pull_cuda_image(appsink: &gst_app::AppSink, channels: usize) -> Option<CudaImage> {
    let sample = appsink.pull_sample().unwrap();

    let buffer = sample.buffer().unwrap();
    let map = buffer.map_readable().unwrap();

    let surface = unsafe { &*(map.as_slice().as_ptr() as *const NvBufSurface) };
    let surf0_params = unsafe {
        &std::slice::from_raw_parts_mut(surface.surface_list, surface.num_filled as usize)[0]
    };

    let width = surf0_params.width as usize;
    let height = surf0_params.height as usize;
    let pitch = surf0_params.pitch as usize;

    unsafe {
        let image = CudaImage::copy_from_cuda_ptr(
            surf0_params.data_ptr as cust_raw::CUdeviceptr,
            width,
            height,
            pitch,
            channels,
            surface.gpu_id as i32,
        );

        Some(image)
    }
}

#[pyclass]
pub struct NvRtspSource {
    pipeline: Pipeline,
    rtspbin: RtspBin,
    last_cuda_image: Arc<Mutex<Option<CudaImage>>>,
}

#[pymethods]
impl NvRtspSource {
    #[new]
    pub fn new(uri: &str, username: Option<&str>, password: Option<&str>) -> Result<Self, Error> {
        let pipeline = Pipeline::new(uri);

        // crate pieline elements
        let rtspbin = RtspBin::new(uri, username, password)?;
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
            .field("format", "BGR")
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
                    *img = pull_cuda_image(appsink, 3);

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        pipeline.set_state(gst::State::Playing)?;

        Ok(Self {
            pipeline,
            rtspbin,
            last_cuda_image,
        })
    }

    fn read(&mut self) -> Option<CudaImage> {
        self.last_cuda_image.lock().unwrap().take()
    }

    fn is_reconnecting(&self) -> bool {
        self.rtspbin.is_reconnecting()
    }
}

impl Drop for NvRtspSource {
    fn drop(&mut self) {
        self.pipeline
            .set_state(gst::State::Null)
            .expect("Cant set pipeline state to null");
    }
}
