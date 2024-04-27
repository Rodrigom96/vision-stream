use pyo3::{prelude::*, py_run};

pub mod image;
mod sources;

pub fn register_cuda_module(py: Python<'_>, parent_module: &PyModule) -> PyResult<()> {
    let m = PyModule::new(py, "cuda")?;
    m.add_class::<image::CudaImage>()?;
    m.add_class::<sources::CudaRtspSource>()?;

    py_run!(
        py,
        m,
        "import sys; sys.modules['vision_stream._lib.cuda'] = m"
    );

    parent_module.add_submodule(m)?;
    Ok(())
}
