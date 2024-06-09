use image::{imageops::FilterType, DynamicImage, GenericImageView};

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
    Unchanged,
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
            return LimitImageFileSizeResponse::new(
                self,
                LimitImageFileSizeResponseKind::Unchanged,
            );
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
    use const_str::concat as const_str_concat;
    use rstest::rstest;

    use crate::image::limit_file_size::{LimitFileSize, LimitImageFileSizeResponseKind};

    const TEST_RESOURCES_PATH: &str = "src/image/test";

    #[rstest]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_1.png"),
        2_u32.pow(20),
        LimitImageFileSizeResponseKind::Resized
    )]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_2.png"),
        2_u32.pow(20),
        LimitImageFileSizeResponseKind::Resized
    )]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_1.png"),
        10 * 2_u32.pow(20),
        LimitImageFileSizeResponseKind::Unchanged
    )]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_2.png"),
        10 * 2_u32.pow(20),
        LimitImageFileSizeResponseKind::Resized
    )]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_1.png"),
        50 * 2_u32.pow(20),
        LimitImageFileSizeResponseKind::Unchanged
    )]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_2.png"),
        50 * 2_u32.pow(20),
        LimitImageFileSizeResponseKind::Unchanged
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
}
