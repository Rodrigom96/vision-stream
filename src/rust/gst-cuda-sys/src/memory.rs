use glib_sys::gpointer;

#[repr(C)]
#[derive(Debug)]
pub struct GstCudaAllocationParams {
    pub parent: gst_sys::GstAllocationParams,
    pub info: gst_video_sys::GstVideoInfo,
}

#[repr(C)]
#[derive(Debug)]
pub struct GstCudaMemory {
    pub mem: gst_sys::GstMemory,
    pub context: *mut crate::context::GstCudaContext,
    pub data: cust_raw::CUdeviceptr,
    pub alloc_params: GstCudaAllocationParams,
    /* offset and stride of CUDA device memory */
    pub offset: [libc::c_uint; gst_video_sys::GST_VIDEO_MAX_PLANES as usize],
    pub stride: libc::c_int,

    /* allocated CUDA Host memory */
    pub map_alloc_data: glib_sys::gpointer,

    /* aligned CUDA Host memory */
    pub align_data: *mut libc::c_uchar,

    /* pointing align_data if the memory is mapped */
    pub map_data: gpointer,

    pub map_count: libc::c_int,
    pub lock: glib_sys::GMutex,
}
