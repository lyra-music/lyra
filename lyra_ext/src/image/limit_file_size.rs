use std::borrow::Cow;

use image::{DynamicImage, GenericImageView, imageops::FilterType};

use crate::num::cast::f64_as_u32;

pub trait LimitFileSize {
    fn limit_file_size(&self, limit: u32) -> Cow<DynamicImage>;
}

impl LimitFileSize for DynamicImage {
    fn limit_file_size(&self, limit: u32) -> Cow<DynamicImage> {
        let (x, y) = self.dimensions();
        let bytes_per_pixel = self.color().bytes_per_pixel();

        let max_image_size = x * y * u32::from(bytes_per_pixel);
        if max_image_size <= limit {
            return Cow::Borrowed(self);
        }

        let x_to_y = f64::from(x) / f64::from(y);

        let new_y = (f64::from(limit) / (f64::from(bytes_per_pixel) * x_to_y)).sqrt();
        let new_x = new_y * x_to_y;

        let image = self.resize(f64_as_u32(new_x), f64_as_u32(new_y), FilterType::Lanczos3);
        Cow::Owned(image)
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use const_str::concat as const_str_concat;
    use rstest::rstest;

    use crate::image::limit_file_size::LimitFileSize;

    const TEST_RESOURCES_PATH: &str = "src/image/test";

    #[rstest]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_1.png"),
        10 * 2_u32.pow(20),
    )]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_1.png"),
        50 * 2_u32.pow(20),
    )]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_2.png"),
        50 * 2_u32.pow(20),
    )]
    fn limit_file_size_borrowed(#[case] input_path: &str, #[case] input_limit: u32) {
        let image = image::open(input_path).unwrap_or_else(|e| panic!("{e:#?}"));
        let response = image.limit_file_size(input_limit);
        assert!(matches!(response, Cow::Borrowed(_)));
    }

    #[rstest]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_1.png"),
        2_u32.pow(20),
    )]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_2.png"),
        2_u32.pow(20),
    )]
    #[case(
        const_str_concat!(TEST_RESOURCES_PATH, "/limit_file_size_2.png"),
        10 * 2_u32.pow(20),
    )]
    fn limit_file_size_owned(#[case] input_path: &str, #[case] input_limit: u32) {
        let image = image::open(input_path).unwrap_or_else(|e| panic!("{e:#?}"));
        let response = image.limit_file_size(input_limit);
        assert!(matches!(response, Cow::Owned(_)));
    }
}
