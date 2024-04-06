use cust_raw::{cuMemcpyDtoD_v2, cudaError_enum, CUdeviceptr};
use gst::prelude::*;
use pyo3::prelude::*;
use pyo3_tch::PyTensor;
use std::sync::{Arc, Mutex};

use deepstream_sys::nvbufsurface::NvBufSurface;

use crate::core::source_bins::RtspBin;
use crate::deepstream::nv_image::NvImage;
use crate::errors::{Error, GstMissingElementError};

pub fn pull_nv_image(appsink: &gst_app::AppSink) -> Option<NvImage> {
    let sample = appsink.pull_sample().unwrap();

    let buffer = sample.buffer().unwrap();
    let map = buffer.map_readable().unwrap();

    // get nvbufsurface
    let surf0_params = unsafe {
        let surface = &*(map.as_slice().as_ptr() as *const NvBufSurface);
        &std::slice::from_raw_parts_mut(surface.surface_list, surface.num_filled as usize)[0]
    };

    // create empty torch tensor
    let tensor = tch::Tensor::empty(
        &[surf0_params.height as i64, surf0_params.width as i64, 4_i64],
        (tch::Kind::Uint8, tch::Device::Cuda(0)),
    );

    // copy nvbufsurface image into torch tensor
    unsafe {
        let cu_mem_copy_result = cuMemcpyDtoD_v2(
            tensor.data_ptr() as CUdeviceptr,
            surf0_params.data_ptr as CUdeviceptr,
            surf0_params.data_size.try_into().unwrap(),
        );
        assert_eq!(cu_mem_copy_result, cudaError_enum::CUDA_SUCCESS);
    }

    Some(NvImage::new(tensor.slice(-1, 0, 3, 1)))
}

#[pyclass]
pub struct NvRtspSource {
    pipeline: gst::Pipeline,
    last_nv_image: Arc<Mutex<Option<NvImage>>>,
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

        let last_nv_image = Arc::new(Mutex::new(None));
        let last_nv_image_clone = Arc::clone(&last_nv_image);
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let mut img = last_nv_image_clone.lock().unwrap();
                    *img = pull_nv_image(appsink);

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        pipeline.set_state(gst::State::Playing)?;

        Ok(Self {
            pipeline,
            last_nv_image,
        })
    }

    fn read(&mut self) -> PyResult<Option<PyTensor>> {
        let nv_img = self.last_nv_image.lock().unwrap().take();
        match nv_img {
            Some(nv_img) => {
                let tensor = nv_img.to_pytorch()?;
                Ok(Some(tensor))
            }
            None => Ok(None),
        }
    }
}

impl Drop for NvRtspSource {
    fn drop(&mut self) {
        self.pipeline
            .set_state(gst::State::Null)
            .expect("Cant set pipeline state to null");
    }
}
