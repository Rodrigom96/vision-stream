use libc::{c_int, c_uchar, c_uint, c_ulong, c_void};

const MAX_PLANES: usize = 4;
const STRUCTURE_PADDING: usize = 4;

#[repr(C)]
#[derive(Debug)]
pub struct NvBufSurfacePlaneParams {
    pub num_planes: c_uint,
    pub width: [c_uint; MAX_PLANES],
    pub height: [c_uint; MAX_PLANES],
    pub pitch: [c_uint; MAX_PLANES],
    pub offset: [c_uint; MAX_PLANES],
    pub psize: [c_uint; MAX_PLANES],
    pub bytes_per_pix: [c_uint; MAX_PLANES],
    _reserved: [[c_void; STRUCTURE_PADDING]; MAX_PLANES],
}

#[repr(C)]
#[derive(Debug)]
pub struct NvBufSurfaceMappedAddr {
    pub addr: [*mut c_void; MAX_PLANES],
    pub egl_image: *mut c_void,
    _reserved: [c_void; STRUCTURE_PADDING],
}

#[repr(C)]
#[derive(Debug)]
pub struct NvBufSurfaceParams {
    pub width: c_uint,
    pub height: c_uint,
    pub pitch: c_uint,
    pub color_format: c_int,
    pub layout: c_int,
    pub buffer_desc: c_ulong,
    pub data_size: c_uint,
    pub data_ptr: *mut c_void,
    pub plane_params: NvBufSurfacePlaneParams,
    pub mapped_addr: NvBufSurfaceMappedAddr,
    _reserved: [c_void; STRUCTURE_PADDING],
}

#[repr(C)]
#[derive(Debug)]
pub struct NvBufSurface {
    pub gpu_id: c_uint,
    pub batch_size: c_uint,
    pub num_filled: c_uint,
    pub is_contiguous: bool,
    pub mem_type: c_int,
    pub surface_list: *mut NvBufSurfaceParams,
    _reserved: [c_void; STRUCTURE_PADDING],
}

extern "C" {
    pub fn NvBufSurfaceCopy(src_surf: *mut NvBufSurface, dst_surf: *mut NvBufSurface) -> c_int;

    pub fn NvBufSurfaceMemSet(
        surf: *mut NvBufSurface,
        index: c_int,
        plane: c_int,
        value: c_uchar,
    ) -> c_int;
}
