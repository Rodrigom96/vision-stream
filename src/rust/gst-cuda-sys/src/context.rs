#[derive(Debug)]
pub struct GstCudaContextPrivate {
    pub context: cust_raw::CUcontext,
    pub device: cust_raw::CUdevice,
    pub device_id: libc::c_int,
    pub tex_align: libc::c_int,
    pub accessible_peer: *mut libc::c_void,
}

#[derive(Debug)]
pub struct GstCudaContext {
    pub object: gst_sys::GstObject,
    pub r#priv: *mut GstCudaContextPrivate,
}
