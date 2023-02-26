use std::io::{BufReader, BufWriter, Cursor};

use image::{self, io, GenericImageView};

pub fn limit_image_byte_size(img_bytes: &[u8], limit: u64) -> image::ImageResult<Vec<u8>> {
    const QUALITY: u8 = 96;

    let lim = limit as f32;
    let exact_img_size = img_bytes.len() as f32;
    if exact_img_size <= lim {
        return Ok(img_bytes.into());
    };

    let img_reader =
        io::Reader::new(BufReader::new(Cursor::new(img_bytes))).with_guessed_format()?;
    let img_fmt = image::ImageOutputFormat::Jpeg(QUALITY);
    // let img_fmt = img_reader
    //     .format()
    //     .expect("the image format is not supported");

    let img = img_reader.decode()?;
    let (x, y) = img.dimensions();
    let bytes_per_pixel = img.color().bytes_per_pixel() as u32;

    let approx_img_size = (x * y * bytes_per_pixel) as f32;
    let compression = exact_img_size / approx_img_size;

    let scale = (lim / (approx_img_size * compression)).sqrt();
    let resize_fn = |z| ((z as f32) * scale).ceil() as u32;

    let new_img = img.resize(
        resize_fn(x),
        resize_fn(y),
        image::imageops::FilterType::Lanczos3,
    );

    let mut new_img_buf = BufWriter::new(Cursor::new(Vec::new()));
    new_img.write_to(&mut new_img_buf, img_fmt)?;

    let new_img_bytes = new_img_buf
        .into_inner()
        .expect("error unwrapping the image wrapper")
        .into_inner();

    Ok(new_img_bytes)
}
