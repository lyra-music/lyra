use image::{self, imageops::FilterType, DynamicImage, GenericImageView};
use kmeans_colors::{get_kmeans_hamerly, Sort};
use palette::{rgb::channels::Rgba, FromColor, IntoColor, Lab, Pixel, Srgb, Srgba};
use rayon::prelude::*;

pub fn get_dominant_palette_from_image(
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
        .par_iter()
        .filter(|x| x.alpha == 255)
        .map(|x| x.into_format::<_, f32>().into_color())
        .collect::<Vec<_>>();

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
        .max_by(|k1, k2| k1.score.total_cmp(&k2.score))
        .expect("expected `RUNS` to be greater or equal to 1");

    let mut res = Lab::sort_indexed_colors(&result.centroids, &result.indices);
    res.sort_unstable_by(|a, b| (b.percentage).total_cmp(&a.percentage));
    let rgb = res
        .par_iter()
        .map(|x| {
            Srgb::from_color(x.centroid)
                .into_format::<u8>()
                .into_u32::<Rgba>()
        })
        .collect::<Vec<_>>();
    rgb
}

pub fn limit_image_file_size(image: DynamicImage, limit: u64) -> DynamicImage {
    let (x, y) = image.dimensions();
    let bytes_per_pixel = image.color().bytes_per_pixel() as u32;

    let max_image_size = (x * y * bytes_per_pixel) as u64;
    if max_image_size <= limit {
        return image;
    }

    let x_to_y = x as f64 / y as f64;

    let new_y = ((limit as f64) / (bytes_per_pixel as f64 * x_to_y)).sqrt();
    let new_x = new_y * x_to_y;

    image.resize(new_x as u32, new_y as u32, FilterType::Lanczos3)
}

#[cfg(test)]
mod test {
    use super::limit_image_file_size;

    #[test]
    fn test_limit_image_byte_size() {
        let image = image::open("../assets/lyra2-X.png").unwrap_or_else(|e| panic!("{e:#?}"));
        let new_image = limit_image_file_size(image, 8 * 2_u64.pow(20));
        new_image
            .save("../assets/lyra2-XD.png")
            .unwrap_or_else(|e| panic!("{e:#?}"));
    }
}
