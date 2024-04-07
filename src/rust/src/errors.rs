use core::fmt;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

#[derive(Debug)]
pub struct GstMissingElementError(pub &'static str);

impl fmt::Display for GstMissingElementError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Missing element {}", self.0)
    }
}

#[derive(Debug)]
pub enum Error {
    GlibBool(glib::BoolError),
    GstMissingElement(GstMissingElementError),
    GstPipelineStateChange(gst::StateChangeError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::GlibBool(e) => write!(f, "GlibBoolError: {}", e.message),
            Error::GstMissingElement(e) => write!(f, "GstMissingElementError: {e:?}"),
            Error::GstPipelineStateChange(e) => {
                write!(f, "GstPipelineStateChangeError: {e:?}")
            }
        }
    }
}

impl From<glib::BoolError> for Error {
    fn from(err: glib::BoolError) -> Self {
        Self::GlibBool(err)
    }
}

impl From<GstMissingElementError> for Error {
    fn from(err: GstMissingElementError) -> Self {
        Self::GstMissingElement(err)
    }
}

impl From<gst::StateChangeError> for Error {
    fn from(err: gst::StateChangeError) -> Self {
        Self::GstPipelineStateChange(err)
    }
}

impl std::convert::From<Error> for PyErr {
    fn from(err: Error) -> PyErr {
        PyException::new_err(err.to_string())
    }
}
