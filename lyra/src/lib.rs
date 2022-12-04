use inc::*;
use pyo3::prelude::*;

mod inc;

#[pyfunction]
fn get_dominant_palette_from_image(img_path: String, palette_size: usize) -> PyResult<Vec<u32>> {
    let img = image::open(&img_path).unwrap();
    Ok(domcols::get_dominant_palette_from_image_impl(
        &img,
        palette_size,
    ))
}

#[pymodule]
fn lyra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_dominant_palette_from_image, m)?)?;
    Ok(())
}
