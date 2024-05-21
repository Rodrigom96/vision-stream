use cust_raw as cuda;
use pyo3::prelude::*;

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
    device: i32,
}

impl CudaImage {
    pub unsafe fn copy_from_cuda_ptr(
        src_data_ptr: cuda::CUdeviceptr,
        width: usize,
        height: usize,
        pitch: usize,
        channels: usize,
        device: i32,
    ) -> Self {
        let mut data_ptr = cuda::CUdeviceptr::default();

        let size = width * height * channels;

        unsafe {
            assert_eq!(
                cuda::cuMemAlloc_v2(&mut data_ptr as *mut cuda::CUdeviceptr, size),
                cuda::cudaError_enum::CUDA_SUCCESS
            );

            let copy_size = width * channels;
            for i in 0..height {
                let src_offset = (i * pitch) as u64;
                let dst_offset = (i * width * channels) as u64;
                assert_eq!(
                    cuda::cuMemcpyDtoD_v2(data_ptr + dst_offset, src_data_ptr + src_offset, copy_size),
                    cuda::cudaError_enum::CUDA_SUCCESS
                );
            }
        }

        Self {
            width,
            height,
            channels,
            data_ptr,
            device,
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
