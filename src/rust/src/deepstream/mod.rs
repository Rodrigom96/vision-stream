use pyo3::{prelude::*, py_run};

mod cuda_image;
mod sources;

pub fn register_deepstream_module(py: Python<'_>, parent_module: &PyModule) -> PyResult<()> {
    let m = PyModule::new(py, "deepstream")?;
    m.add_class::<cuda_image::CudaImage>()?;
    m.add_class::<sources::NvRtspSource>()?;

    py_run!(
        py,
        m,
        "import sys; sys.modules['vision_stream._lib.deepstream'] = m"
    );

    parent_module.add_submodule(m)?;
    Ok(())
}
