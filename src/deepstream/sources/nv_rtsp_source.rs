use gst::prelude::*;
use pyo3::prelude::*;
use pyo3_tch::PyTensor;
use std::sync::{Arc, Mutex};

use crate::core::source_bins::RtspBin;
use crate::deepstream::nv_image::NvImage;
use crate::errors::{Error, GstMissingElementError};

use deepstream_sys::nvbufsurface::{
    NvBufSurface, NvBufSurfaceCopy, NvBufSurfaceMemSet, NvBufSurfaceParams,
};
use libc::c_void;

fn get_surface<'a>(buffer_map: &gst::BufferMap<'a, gst::buffer::Readable>) -> NvBufSurface {
    unsafe {
        let mut surface: NvBufSurface = std::mem::zeroed();
        NvBufSurfaceMemSet(&mut surface, -1, -1, 0);
        libc::memcpy(
            &mut surface as *mut _ as *mut c_void,
            buffer_map.as_slice().as_ptr() as *const c_void,
            buffer_map.size(),
        );
        surface
    }
}

fn struct_copy(src: &mut NvBufSurface, dst: &mut NvBufSurface) {
    dst.batch_size = src.batch_size;
    dst.num_filled = src.num_filled;
    dst.is_contiguous = src.is_contiguous;
    dst.mem_type = src.mem_type;

    unsafe {
        dst.surface_list =
            libc::malloc(std::mem::size_of::<NvBufSurfaceParams>() * src.num_filled as usize)
                as *mut NvBufSurfaceParams;
        let surface_list =
            std::slice::from_raw_parts_mut(dst.surface_list, src.num_filled as usize);
        let other_surface_list =
            std::slice::from_raw_parts(src.surface_list, src.num_filled as usize);
        for surface_ix in 0..(src.num_filled as usize) {
            surface_list[surface_ix].width = other_surface_list[surface_ix].width;
            surface_list[surface_ix].height = other_surface_list[surface_ix].height;
            surface_list[surface_ix].pitch = other_surface_list[surface_ix].pitch;
            surface_list[surface_ix].color_format = other_surface_list[surface_ix].color_format;
            surface_list[surface_ix].layout = other_surface_list[surface_ix].layout;
            surface_list[surface_ix].buffer_desc = other_surface_list[surface_ix].buffer_desc;
            surface_list[surface_ix].data_size = other_surface_list[surface_ix].data_size;

            surface_list[surface_ix].plane_params.num_planes =
                other_surface_list[surface_ix].plane_params.num_planes;
            surface_list[surface_ix].plane_params.width =
                other_surface_list[surface_ix].plane_params.width;
            surface_list[surface_ix].plane_params.height =
                other_surface_list[surface_ix].plane_params.height;
            surface_list[surface_ix].plane_params.pitch =
                other_surface_list[surface_ix].plane_params.pitch;
            surface_list[surface_ix].plane_params.offset =
                other_surface_list[surface_ix].plane_params.offset;
            surface_list[surface_ix].plane_params.psize =
                other_surface_list[surface_ix].plane_params.psize;
            surface_list[surface_ix].plane_params.bytes_per_pix =
                other_surface_list[surface_ix].plane_params.bytes_per_pix;
        }
    }
}

pub fn pull_nv_image(appsink: &gst_app::AppSink) -> Option<NvImage> {
    let sample = appsink.pull_sample().unwrap();

    let buffer = sample.buffer().unwrap();
    let map = buffer.map_readable().unwrap();
    let mut surface = get_surface(&map);

    // create tensor and link with dummy surface
    let mut torch_surface = get_surface(&map);
    struct_copy(&mut surface, &mut torch_surface);
    let mut s = unsafe {
        std::slice::from_raw_parts_mut(
            torch_surface.surface_list,
            torch_surface.num_filled as usize,
        )
    };
    let tensor = tch::Tensor::zeros(
        &[s[0].height as i64, s[0].width as i64, 4_i64],
        (tch::Kind::Uint8, tch::Device::Cuda(0)),
    );
    s[0].data_ptr = tensor.data_ptr();
    torch_surface.gpu_id = 0;

    // copy surface into torch tensor
    unsafe {
        let res = NvBufSurfaceCopy(&mut surface, &mut torch_surface);
        assert_eq!(res, 0, "NvBufSurfaceCopy fail");
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
