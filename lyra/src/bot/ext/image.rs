use image::{self, imageops::FilterType, DynamicImage, GenericImageView};
use kmeans_colors::Sort;
use palette::{cast::from_component_slice, FromColor, IntoColor, Lab, Srgb, Srgba};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

pub trait DominantPalette {
    fn dominant_palette(&self, palette_size: usize) -> Vec<Srgb<u8>>;
}

impl DominantPalette for DynamicImage {
    fn dominant_palette(&self, palette_size: usize) -> Vec<Srgb<u8>> {
        const MAX_ITERATIONS: usize = usize::MAX;
        const RESIZE: u32 = 1 << 7;
        const RANDOM_SEED: u64 = 0;
        const RUNS: u8 = 1 << 4;
        const COVERAGE: f32 = 1.;

        let img = self
            .resize(RESIZE, RESIZE, image::imageops::FilterType::Nearest)
            .to_rgba8();
        let img_vec = img.into_raw();

        let lab = from_component_slice::<Srgba<u8>>(&img_vec)
            .par_iter()
            .filter(|x| x.alpha == 255)
            .map(|x| x.into_format::<_, f32>().into_color())
            .collect::<Vec<Lab>>();

        // SAFETY: `0..RUNS` is a non-empty iterator,
        //         so unwrapping `.max_by(...)` is safe
        let result = unsafe {
            (0..RUNS)
                .map(|i| {
                    kmeans_colors::get_kmeans_hamerly(
                        palette_size,
                        MAX_ITERATIONS,
                        COVERAGE,
                        false,
                        &lab,
                        RANDOM_SEED + u64::from(i),
                    )
                })
                .max_by(|k1, k2| k1.score.total_cmp(&k2.score))
                .unwrap_unchecked()
        };

        let mut res = Lab::sort_indexed_colors(&result.centroids, &result.indices);
        res.sort_unstable_by(|a, b| (b.percentage).total_cmp(&a.percentage));
        let rgb = res
            .par_iter()
            .map(|x| Srgb::from_color(x.centroid).into_format())
            .collect::<Vec<_>>();
        rgb
    }
}

pub struct LimitImageFileSizeResponse {
    image: DynamicImage,
    kind: LimitImageFileSizeResponseKind,
}

impl LimitImageFileSizeResponse {
    const fn new(image: DynamicImage, kind: LimitImageFileSizeResponseKind) -> Self {
        Self { image, kind }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LimitImageFileSizeResponseKind {
    Noop,
    Resized,
}

pub trait LimitFileSize {
    fn limit_file_size(self, limit: u32) -> LimitImageFileSizeResponse;
}

impl LimitFileSize for DynamicImage {
    fn limit_file_size(self, limit: u32) -> LimitImageFileSizeResponse {
        let (x, y) = self.dimensions();
        let bytes_per_pixel = u32::from(self.color().bytes_per_pixel());

        let max_image_size = x * y * bytes_per_pixel;
        if max_image_size <= limit {
            return LimitImageFileSizeResponse::new(self, LimitImageFileSizeResponseKind::Noop);
        }

        let x_to_y = f64::from(x) / f64::from(y);

        let new_y = (f64::from(limit) / (f64::from(bytes_per_pixel) * x_to_y)).sqrt();
        let new_x = new_y * x_to_y;

        let image = self.resize(new_x as u32, new_y as u32, FilterType::Lanczos3);
        LimitImageFileSizeResponse::new(image, LimitImageFileSizeResponseKind::Resized)
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::{DominantPalette, LimitFileSize, LimitImageFileSizeResponseKind};

    const TEST_RESOURCES_PATH: &str = "src/bot/ext/test";

    #[rstest]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/limit_file_size_1.png"),
        2_u32.pow(20),
        LimitImageFileSizeResponseKind::Resized
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/limit_file_size_2.png"),
        2_u32.pow(20),
        LimitImageFileSizeResponseKind::Resized
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/limit_file_size_1.png"),
        10 * 2_u32.pow(20),
        LimitImageFileSizeResponseKind::Noop
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/limit_file_size_2.png"),
        10 * 2_u32.pow(20),
        LimitImageFileSizeResponseKind::Resized
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/limit_file_size_1.png"),
        50 * 2_u32.pow(20),
        LimitImageFileSizeResponseKind::Noop
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/limit_file_size_2.png"),
        50 * 2_u32.pow(20),
        LimitImageFileSizeResponseKind::Noop
    )]
    fn limit_file_size(
        #[case] input_path: &str,
        #[case] input_limit: u32,
        #[case] expected_response_kind: LimitImageFileSizeResponseKind,
    ) {
        let image = image::open(input_path).unwrap_or_else(|e| panic!("{e:#?}"));
        let response = image.limit_file_size(input_limit);
        assert_eq!(response.kind, expected_response_kind);
    }

    #[rstest]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/dominant_palette_1.jpg"),
        1,
        &[(101, 100, 134).into()]
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/dominant_palette_1.jpg"),
        2,
        &[(93, 108, 132).into(), (131, 43, 145).into()]
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/dominant_palette_2.jpg"),
        3,
        &[(63, 60, 69).into(), (134, 94, 94).into(), (188, 157, 135).into()]
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/dominant_palette_2.jpg"),
        4,
        &[(98, 75, 83).into(), (149, 107, 101).into(), (192, 162, 138).into(), (47, 53, 62).into()]
    )]
    fn dominant_palette(
        #[case] input_path: &str,
        #[case] input_palette_size: usize,
        #[case] expected: &[palette::rgb::Srgb<u8>],
    ) {
        let image = image::open(input_path).unwrap_or_else(|e| panic!("{e:#?}"));
        assert_eq!(image.dominant_palette(input_palette_size), expected);
    }
}
