use std::sync::Arc;

use cached::proc_macro::cached;
use image::{DynamicImage, ImageError};
use inc::*;
use lyra_macros::cached_oxidized;
use pyo3::{exceptions, prelude::*, types::PyBytes};

mod inc;

#[derive(Clone)]
pub enum LyraErr {
    Image(Arc<ImageError>),
}

impl LyraErr {
    fn to_string(&self) -> String {
        match self {
            LyraErr::Image(err) => err.to_string(),
        }
    }
}

impl From<LyraErr> for PyErr {
    fn from(err: LyraErr) -> Self {
        exceptions::PyRuntimeError::new_err(err.to_string())
    }
}

impl From<ImageError> for LyraErr {
    fn from(err: ImageError) -> Self {
        Self::Image(err.into())
    }
}

#[cached_oxidized]
fn process_image_bytes(bytes: Vec<u8>) -> Result<DynamicImage, LyraErr> {
    image::load_from_memory(&bytes)
}

#[cached_oxidized]
fn process_image_path(path: String) -> Result<DynamicImage, LyraErr> {
    image::open(path)
}

#[cached_oxidized]
fn _limit_image_byte_size(img_bytes: Vec<u8>, limit: u32) -> Result<Vec<u8>, LyraErr> {
    utils::limit_image_byte_size_impl(&img_bytes, limit)
}

#[pyfunction]
fn get_dominant_palette_from_image_bytes(
    py: Python<'_>,
    img_bytes: &[u8],
    palette_size: usize,
) -> Result<Vec<u32>, LyraErr> {
    let img = process_image_bytes(img_bytes.to_owned())?;
    Ok(py.allow_threads(move || domcols::get_dominant_palette_from_image_impl(&img, palette_size)))
}

#[pyfunction]
fn get_dominant_palette_from_image_path(
    py: Python<'_>,
    img_path: &str,
    palette_size: usize,
) -> Result<Vec<u32>, LyraErr> {
    let img = process_image_path(img_path.to_owned())?;
    Ok(py.allow_threads(move || domcols::get_dominant_palette_from_image_impl(&img, palette_size)))
}

#[pyfunction]
fn limit_image_byte_size(
    py: Python<'_>,
    img_bytes: &[u8],
    limit: u32,
) -> Result<PyObject, LyraErr> {
    match _limit_image_byte_size(img_bytes.to_owned(), limit) {
        Ok(obj) => Ok(PyBytes::new(py, &obj).into()),
        Err(err) => Err(err.into()),
    }
}

#[pymodule]
fn lyra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_dominant_palette_from_image_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(get_dominant_palette_from_image_path, m)?)?;
    m.add_function(wrap_pyfunction!(limit_image_byte_size, m)?)?;
    Ok(())
}
