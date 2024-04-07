use pyo3::prelude::*;

#[pyclass]
#[derive(Clone, Debug)]
pub struct Image {
    pub width: i32,
    pub height: i32,
    pub channels: i32,
    pub data: Vec<u8>,
}

#[pymethods]
impl Image {
    fn to_numpy<'py>(&self, py: Python<'py>) -> &'py numpy::PyArray3<u8> {
        let arr = numpy::PyArray::from_vec(py, self.data.clone());
        let arr = arr
            .reshape([
                self.height as usize,
                self.width as usize,
                self.channels as usize,
            ])
            .unwrap();
        arr
    }
}
