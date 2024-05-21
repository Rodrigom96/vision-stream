use gst::prelude::*;
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

use crate::core::pipeline::Pipeline;
use crate::core::source_bins::RtspBin;
use crate::cuda::image::CudaImage;
use crate::errors::{Error, GstMissingElementError};

fn pull_cuda_image(appsink: &gst_app::AppSink, channels: usize) -> Option<CudaImage> {
    let sample = appsink.pull_sample().unwrap();

    let buffer = sample.buffer().unwrap();
    let gst_mem = buffer.memory(0).unwrap();
    let mem = unsafe { &*(gst_mem.as_ptr() as *const gst_cuda_sys::memory::GstCudaMemory) };
    let width = mem.alloc_params.info.width as usize;
    let height = mem.alloc_params.info.height as usize;

    unsafe {
        let gst_cuda_context = &*(*mem.context).r#priv;

        // push cuda context
        assert_eq!(
            cust_raw::cuCtxPushCurrent_v2(gst_cuda_context.context),
            cust_raw::cudaError_enum::CUDA_SUCCESS
        );

        // copy image to cuda image
        let img = CudaImage::copy_from_cuda_ptr(
            mem.data,
            width,
            height,
            width * channels,
            channels,
            gst_cuda_context.device_id,
        );

        // pop cuda context
        assert_eq!(
            cust_raw::cuCtxPopCurrent_v2(&mut std::ptr::null_mut() as *mut cust_raw::CUcontext),
            cust_raw::cudaError_enum::CUDA_SUCCESS
        );

        Some(img)
    }
}

#[pyclass]
pub struct CudaRtspSource {
    pipeline: Pipeline,
    rtspbin: RtspBin,
    last_image: Arc<Mutex<Option<CudaImage>>>,
}

#[pymethods]
impl CudaRtspSource {
    #[new]
    pub fn new(uri: &str, username: Option<&str>, password: Option<&str>) -> Result<Self, Error> {
        let pipeline = Pipeline::new(uri);

        // crate pieline elements
        let rtspbin = RtspBin::new(uri, username, password)?;
        let videoconvert = gst::ElementFactory::make("cudaconvert")
            .build()
            .map_err(|_| GstMissingElementError("cudaconvert"))?;
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
            .features(["memory:CUDAMemory"])
            .field("format", "BGRA")
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
                    *img = pull_cuda_image(appsink, 4);

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

    fn read(&mut self) -> Option<CudaImage> {
        self.last_image.lock().unwrap().take()
    }

    fn is_reconnecting(&self) -> bool {
        self.rtspbin.is_reconnecting()
    }
}

impl Drop for CudaRtspSource {
    fn drop(&mut self) {
        self.pipeline
            .set_state(gst::State::Null)
            .expect("Cant set pipeline state to null");
    }
}
