use image::{DynamicImage, ImageResult};
use kmeans_colors::Sort;
use palette::{FromColor, IntoColor, Lab, Srgb, Srgba, cast::from_component_slice};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub type DominantColour = Srgb<u8>;
pub type DominantPalette = Vec<DominantColour>;

pub trait Get {
    fn get_dominant_palette(&self, palette_size: usize) -> DominantPalette;
}

impl Get for DynamicImage {
    fn get_dominant_palette(&self, palette_size: usize) -> DominantPalette {
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

        let result = (0..RUNS)
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
            .expect("number of runs must be non-zero");

        let mut res = Lab::sort_indexed_colors(&result.centroids, &result.indices);
        res.sort_unstable_by(|a, b| (b.percentage).total_cmp(&a.percentage));
        res.par_iter()
            .map(|x| Srgb::from_color(x.centroid).into_format())
            .collect::<Vec<_>>()
    }
}

/// # Errors
/// When loading the images from bytes failed
pub fn from_bytes(bytes: &[u8], palette_size: usize) -> ImageResult<DominantPalette> {
    let image = image::load_from_memory(bytes)?;

    Ok(image.get_dominant_palette(palette_size))
}

#[must_use]
pub fn normalise(dominant_palette: DominantPalette) -> Vec<u32> {
    dominant_palette
        .into_iter()
        .map(|c| c.into_u32::<palette::rgb::channels::Rgba>() >> 8)
        .collect()
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::image::dominant_palette::Get;

    const TEST_RESOURCES_PATH: &str = "src/image/test";

    #[rstest]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/dominant_palette_1.jpg"),
        1,
        &[(101, 100, 134).into()]
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/dominant_palette_1.jpg"),
        2,
        &[(63, 51, 99).into(), (149, 168, 180).into()]
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/dominant_palette_2.jpg"),
        3,
        &[(126, 92, 92).into(), (57, 57, 67).into(), (188, 155, 133).into()]
    )]
    #[case(
        const_str::concat!(TEST_RESOURCES_PATH, "/dominant_palette_2.jpg"),
        4,
        &[(55, 54, 65).into(), (114, 92, 93).into(), (189, 158, 136).into(), (155, 91, 90).into()]
    )]
    fn dominant_palette(
        #[case] input_path: &str,
        #[case] input_palette_size: usize,
        #[case] expected: &[palette::rgb::Srgb<u8>],
    ) {
        let image = image::open(input_path).unwrap_or_else(|e| panic!("{e:#?}"));
        assert_eq!(image.get_dominant_palette(input_palette_size), expected);
    }
}
