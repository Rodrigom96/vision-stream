use pyo3::prelude::*;
use pyo3_tch::PyTensor;

#[derive(Debug)]
pub struct NvImage {
    tensor: tch::Tensor,
}

impl NvImage {
    pub fn new(tensor: tch::Tensor) -> Self {
        Self { tensor }
    }

    pub fn to_pytorch(self) -> PyResult<PyTensor> {
        Ok(PyTensor(self.tensor))
    }
}
