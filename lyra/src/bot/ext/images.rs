use std::ops::Deref;

use image::{self, imageops::FilterType, DynamicImage, GenericImageView};
use kmeans_colors::Sort;
use palette::{FromColor, IntoColor, Lab, Pixel, Srgb, Srgba};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

pub fn get_dominant_palette_from_image(
    image: &image::DynamicImage,
    palette_size: usize,
) -> Vec<Srgb<u8>> {
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
            kmeans_colors::get_kmeans_hamerly(
                palette_size,
                MAX_ITERATIONS,
                COVERAGE,
                false,
                &lab,
                RANDOM_SEED + i as u64,
            )
        })
        .max_by(|k1, k2| k1.score.total_cmp(&k2.score))
        .expect("`RUNS` must be greater or equal to 1");

    let mut res = Lab::sort_indexed_colors(&result.centroids, &result.indices);
    res.sort_unstable_by(|a, b| (b.percentage).total_cmp(&a.percentage));
    let rgb = res
        .par_iter()
        .map(|x| Srgb::from_color(x.centroid).into_format())
        .collect::<Vec<_>>();
    rgb
}

pub struct LimitImageFileSizeResponse {
    image: DynamicImage,
    kind: LimitImageFileSizeResponseKind,
}

impl LimitImageFileSizeResponse {
    fn new(image: DynamicImage, kind: LimitImageFileSizeResponseKind) -> Self {
        Self { image, kind }
    }
}

impl Deref for LimitImageFileSizeResponse {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

#[derive(Debug, PartialEq)]
pub enum LimitImageFileSizeResponseKind {
    Noop,
    Resized,
}

pub fn limit_image_file_size(image: DynamicImage, limit: u64) -> LimitImageFileSizeResponse {
    let (x, y) = image.dimensions();
    let bytes_per_pixel = image.color().bytes_per_pixel() as u32;

    let max_image_size = (x * y * bytes_per_pixel) as u64;
    if max_image_size <= limit {
        return LimitImageFileSizeResponse::new(image, LimitImageFileSizeResponseKind::Noop);
    }

    let x_to_y = x as f64 / y as f64;

    let new_y = ((limit as f64) / (bytes_per_pixel as f64 * x_to_y)).sqrt();
    let new_x = new_y * x_to_y;

    let image = image.resize(new_x as u32, new_y as u32, FilterType::Lanczos3);
    LimitImageFileSizeResponse::new(image, LimitImageFileSizeResponseKind::Resized)
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::{
        get_dominant_palette_from_image, limit_image_file_size, LimitImageFileSizeResponseKind,
    };

    const TEST_RESOURCES_PATH: &str = "src/bot/ext/test";

    #[rstest]
    #[case(
        format!("{}/limit_image_byte_size_1.png", TEST_RESOURCES_PATH),
        2_u64.pow(20),
        LimitImageFileSizeResponseKind::Resized
    )]
    #[case(
        format!("{}/limit_image_byte_size_2.png", TEST_RESOURCES_PATH),
        2_u64.pow(20),
        LimitImageFileSizeResponseKind::Resized
    )]
    #[case(
        format!("{}/limit_image_byte_size_1.png", TEST_RESOURCES_PATH),
        10 * 2_u64.pow(20),
        LimitImageFileSizeResponseKind::Noop
    )]
    #[case(
        format!("{}/limit_image_byte_size_2.png", TEST_RESOURCES_PATH),
        10 * 2_u64.pow(20),
        LimitImageFileSizeResponseKind::Resized
    )]
    #[case(
        format!("{}/limit_image_byte_size_1.png", TEST_RESOURCES_PATH),
        50 * 2_u64.pow(20),
        LimitImageFileSizeResponseKind::Noop
    )]
    #[case(
        format!("{}/limit_image_byte_size_2.png", TEST_RESOURCES_PATH),
        50 * 2_u64.pow(20),
        LimitImageFileSizeResponseKind::Noop
    )]
    fn test_limit_image_byte_size(
        #[case] input_path: String,
        #[case] input_limit: u64,
        #[case] expected_response_kind: LimitImageFileSizeResponseKind,
    ) {
        let image = image::open(input_path).unwrap_or_else(|e| panic!("{e:#?}"));
        let response = limit_image_file_size(image, input_limit);
        assert_eq!(response.kind, expected_response_kind)
    }

    #[rstest]
    #[case(
        format!("{}/get_dominant_palette_from_image_1.jpg", TEST_RESOURCES_PATH),
        1,
        &[(101, 100, 134).into()]
    )]
    #[case(
        format!("{}/get_dominant_palette_from_image_1.jpg", TEST_RESOURCES_PATH),
        2,
        &[(63, 47, 97).into(), (143, 163, 178).into()]
    )]
    #[case(
        format!("{}/get_dominant_palette_from_image_2.jpg", TEST_RESOURCES_PATH),
        3,
        &[(63, 59, 69).into(), (135, 96, 96).into(), (191, 158, 136).into()]
    )]
    #[case(
        format!("{}/get_dominant_palette_from_image_2.jpg", TEST_RESOURCES_PATH),
        4,
        &[(148, 106, 101).into(), (193, 161, 138).into(), (99, 74, 84).into(), (49, 53, 63).into()]
    )]
    fn test_get_dominant_palette_from_image(
        #[case] input_path: String,
        #[case] input_palette_size: usize,
        #[case] expected: &[palette::rgb::Srgb<u8>],
    ) {
        let image = image::open(input_path).unwrap_or_else(|e| panic!("{e:#?}"));
        assert_eq!(
            get_dominant_palette_from_image(&image, input_palette_size),
            expected
        )
    }
}
