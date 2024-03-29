use pyo3::prelude::*;

mod core;
mod errors;
mod image;
mod sources;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

/// A Python module implemented in Rust.
#[pymodule]
fn _lib(_py: Python, m: &PyModule) -> PyResult<()> {
    pyo3_log::init();
    gst::init().expect("Error on gstreamer init");

    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    m.add_class::<sources::RtspSource>()?;
    m.add_class::<sources::TestSource>()?;
    Ok(())
}
