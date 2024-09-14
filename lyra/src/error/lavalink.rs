use thiserror::Error;

#[derive(Error, Debug)]
#[error("player does not yet exist")]
pub struct NoPlayerError;

#[derive(Error, Debug)]
#[error("processing lavalink event failed: {:?}", .0)]
pub enum ProcessError {
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    TwilightHttp(#[from] twilight_http::Error),
    Sqlx(#[from] sqlx::Error),
    DeserialiseBody(#[from] twilight_http::response::DeserializeBodyError),
    GenerateNowPlayingEmbed(#[from] GenerateNowPlayingEmbedError),
}

#[derive(Error, Debug)]
#[error("generating now playing embed failed: {:?}", .0)]
pub enum GenerateNowPlayingEmbedError {
    ImageSourceUrl(#[from] twilight_util::builder::embed::image_source::ImageSourceUrlError),
    Cache(#[from] super::Cache),
    TimestampParse(#[from] twilight_model::util::datetime::TimestampParseError),
    GetDominantPaletteFromUrl(#[from] std::sync::Arc<GetDominantPaletteFromUrlError>),
}

#[derive(Error, Debug)]
#[error("getting dominant palette from url failed: {:?}", .0)]
pub enum GetDominantPaletteFromUrlError {
    Reqwest(#[from] reqwest::Error),
    Image(#[from] lyra_ext::ImageError),
}

pub type ProcessResult = Result<(), ProcessError>;
