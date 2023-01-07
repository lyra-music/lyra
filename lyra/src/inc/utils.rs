use image::{self, GenericImageView};

pub fn limit_image_byte_size_impl(
    img_bytes: &[u8],
    limit: u32,
) -> Result<Vec<u8>, image::ImageError> {
    if (img_bytes.len() as u32) < limit {
        return Ok(img_bytes.to_owned());
    };
    let resize = ((img_bytes.len() as u32 / limit) as f32).sqrt();
    let img = image::load_from_memory(img_bytes)?;
    let (x, y) = img.dimensions();
    let ceil_fn = |z: u32| -> u32 { (z as f32 / resize).ceil() as u32 };
    let img = img.resize(
        ceil_fn(x),
        ceil_fn(y),
        image::imageops::FilterType::Lanczos3,
    );

    Ok(img.into_bytes())
}
