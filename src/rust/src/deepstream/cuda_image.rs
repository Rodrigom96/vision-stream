use cust_raw as cuda;
use pyo3::prelude::*;

use deepstream_sys::nvbufsurface::NvBufSurface;

#[pyclass]
#[derive(Debug)]
pub struct CudaImage {
    #[pyo3(get)]
    width: usize,
    #[pyo3(get)]
    height: usize,
    #[pyo3(get)]
    channels: usize,
    #[pyo3(get)]
    data_ptr: cuda::CUdeviceptr,
    #[pyo3(get)]
    device: usize,
}

impl CudaImage {
    pub fn copy_from(surface: &NvBufSurface) -> Self {
        let mut data_ptr = cuda::CUdeviceptr::default();

        let surf0_params = unsafe {
            &std::slice::from_raw_parts_mut(surface.surface_list, surface.num_filled as usize)[0]
        };

        unsafe {
            let result = cuda::cuMemAlloc_v2(
                &mut data_ptr as *mut cuda::CUdeviceptr,
                surf0_params.data_size.try_into().unwrap(),
            );
            assert_eq!(result, cuda::cudaError_enum::CUDA_SUCCESS);

            let result = cuda::cuMemcpyDtoD_v2(
                data_ptr,
                surf0_params.data_ptr as cuda::CUdeviceptr,
                surf0_params.data_size.try_into().unwrap(),
            );
            assert_eq!(result, cuda::cudaError_enum::CUDA_SUCCESS);
        }

        Self {
            width: surf0_params.width as usize,
            height: surf0_params.height as usize,
            channels: 4,
            data_ptr,
            device: surface.gpu_id as usize,
        }
    }
}

#[pymethods]
impl CudaImage {
    #[getter]
    fn shape(&self) -> (usize, usize, usize) {
        return (self.height, self.width, self.channels);
    }

    fn copy_to(&self, data_ptr: cuda::CUdeviceptr) {
        unsafe {
            let result = cuda::cuMemcpyDtoD_v2(
                data_ptr,
                self.data_ptr,
                self.width * self.height * self.channels,
            );
            assert_eq!(result, cuda::cudaError_enum::CUDA_SUCCESS);
        }
    }
}

impl Drop for CudaImage {
    fn drop(&mut self) {
        unsafe {
            let result = cuda::cuMemFree_v2(self.data_ptr);
            assert_eq!(result, cuda::cudaError_enum::CUDA_SUCCESS);
        }
    }
}
