use std::{
    fmt::Debug,
    io::{BufReader, BufWriter, Cursor},
};

use convert_case::{Case, Casing};
use image::{self, io, GenericImageView};
use twilight_model::guild::Permissions;

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

trait PrettyJoiner: Sized {
    fn pretty_join(self, sep: &str, ending_sep: &str) -> String;

    fn pretty_join_with_and(self) -> String {
        self.pretty_join(", ", " and ")
    }

    fn pretty_join_with_or(self) -> String {
        self.pretty_join(", ", " or ")
    }
}

impl PrettyJoiner for &[String] {
    fn pretty_join(self, sep: &str, ending_sep: &str) -> String {
        match self.len() {
            0 => String::new(),
            1 => self[0].to_string(),
            _ => {
                let joined = self[..self.len() - 1]
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(sep);
                format!(
                    "{}{}{}",
                    joined,
                    ending_sep,
                    self.last().expect("last element must exist")
                )
            }
        }
    }
}

pub trait BitFlagsPrettify: Debug {
    fn prettify(&self) -> String {
        format!("{:?}", self)
            .split(" | ")
            .map(|s| format!("`{}`", s.to_case(Case::Title)))
            .collect::<Vec<_>>()
            .pretty_join_with_and()
    }
}

impl BitFlagsPrettify for Permissions {}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::bot::inc::utils::PrettyJoiner;

    macro_rules! string_arr {
        ($($ty:literal),+) => {
            &[$($ty.to_string()),+]
        }
    }

    #[rstest]
    #[case(&[], "")]
    #[case(string_arr!["0"], "0")]
    #[case(string_arr!["1", "2"], "1 > 2")]
    #[case(string_arr!["3", "4", "5"], "3 + 4 > 5")]
    #[case(string_arr!["6", "7", "8", "9"], "6 + 7 + 8 > 9")]
    fn test_pretty_join(#[case] input: &[String], #[case] expected: &str) {
        assert_eq!(input.pretty_join(" + ", " > "), expected);
    }

    #[rstest]
    #[case(&[], "")]
    #[case(string_arr!["a"], "a")]
    #[case(string_arr!["b", "c"], "b and c")]
    #[case(string_arr!["d", "e", "f"], "d, e and f")]
    #[case(string_arr!["g", "h", "i", "j"], "g, h, i and j")]
    fn test_pretty_join_with_and(#[case] input: &[String], #[case] expected: &str) {
        assert_eq!(input.pretty_join_with_and(), expected);
    }

    #[rstest]
    #[case(&[], "")]
    #[case(string_arr!["k"], "k")]
    #[case(string_arr!["l", "m"], "l or m")]
    #[case(string_arr!["n", "o", "p"], "n, o or p")]
    #[case(string_arr!["q", "r", "s", "t"], "q, r, s or t")]
    fn test_pretty_join_with_or(#[case] input: &[String], #[case] expected: &str) {
        assert_eq!(input.pretty_join_with_or(), expected);
    }
}
