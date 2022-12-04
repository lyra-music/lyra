use pyo3::prelude::*;

use image;
use kmeans_colors::{get_kmeans_hamerly, Sort};
use palette::{rgb::channels::Rgba, FromColor, IntoColor, Lab, Pixel, Srgb, Srgba};

pub fn _get_dominant_palette_from_image(
    image: &image::DynamicImage,
    palette_size: usize,
) -> Vec<u32> {
    const MAX_ITERATIONS: usize = usize::MAX;
    const RESIZE: u32 = 1 << 7;
    const RANDOM_SEED: u64 = 0;
    const RUNS: u8 = 1 << 4;
    const COVERAGE: f32 = 1.;

    let img = image
        .resize(RESIZE, RESIZE, image::imageops::FilterType::Nearest)
        .to_rgba8();
    let img_vec = img.into_raw();

    let lab = Srgba::from_raw_slice(&img_vec)
        .iter()
        .filter(|x| x.alpha == 255)
        .map(|x| x.into_format::<_, f32>().into_color())
        .collect::<Vec<Lab>>();

    let result = (0..RUNS)
        .map(|i| {
            get_kmeans_hamerly(
                palette_size,
                MAX_ITERATIONS,
                COVERAGE,
                false,
                &lab,
                RANDOM_SEED + i as u64,
            )
        })
        .max_by(|k1, k2| k1.score.partial_cmp(&k2.score).unwrap())
        .unwrap();

    let mut res = Lab::sort_indexed_colors(&result.centroids, &result.indices);
    res.sort_unstable_by(|a, b| (b.percentage).partial_cmp(&a.percentage).unwrap());
    let rgb = res
        .iter()
        .map(|x| {
            Srgb::from_color(x.centroid)
                .into_format::<u8>()
                .into_u32::<Rgba>()
        })
        .collect::<Vec<u32>>();
    rgb
}

#[pyfunction]
fn get_dominant_palette_from_image(img_path: String, palette_size: usize) -> PyResult<Vec<u32>> {
    let img = image::open(&img_path).unwrap();
    Ok(_get_dominant_palette_from_image(&img, palette_size))
}

#[pymodule]
fn lyra(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_dominant_palette_from_image, m)?)?;
    Ok(())
}
