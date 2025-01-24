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
    DeserialiseBodyFromHttp(#[from] super::core::DeserialiseBodyFromHttpError),
    NewNowPlayingMessage(#[from] NewNowPlayingMessageError),
    NewNowPlayingData(#[from] NewNowPlayingDataError),
}

#[derive(Error, Debug)]
#[error("constructing a new now playing data failed: {:?}", .0)]
pub enum NewNowPlayingDataError {
    Cache(#[from] super::Cache),
    GetDominantPaletteFromUrl(#[from] std::sync::Arc<GetDominantPaletteFromUrlError>),
}

#[derive(Error, Debug)]
#[error("building the now playing message embed failed: {:?}", .0)]
pub enum BuildNowPlayingEmbedError {
    ImageSourceUrl(#[from] twilight_util::builder::embed::image_source::ImageSourceUrlError),
    TimestampParse(#[from] twilight_model::util::datetime::TimestampParseError),
}

#[derive(Error, Debug)]
#[error("generating now playing embed failed: {:?}", .0)]
pub enum NewNowPlayingMessageError {
    TwilightHttp(#[from] twilight_http::Error),
    DeserialiseBody(#[from] twilight_http::response::DeserializeBodyError),
    DeserialiseBodyFromHttp(#[from] super::core::DeserialiseBodyFromHttpError),
    BuildNowPlayingEmbed(#[from] BuildNowPlayingEmbedError),
}

#[derive(Error, Debug)]
#[error("updating now playing message failed: {:?}", .0)]
pub enum UpdateNowPlayingMessageError {
    BuildNowPlayingEmbed(#[from] BuildNowPlayingEmbedError),
    DeserialiseBodyFromHttp(#[from] super::core::DeserialiseBodyFromHttpError),
    TwilightHttp(#[from] twilight_http::Error),
}

#[derive(Error, Debug)]
#[error("getting dominant palette from url failed: {:?}", .0)]
pub enum GetDominantPaletteFromUrlError {
    Reqwest(#[from] reqwest::Error),
    Image(#[from] lyra_ext::ImageError),
}

pub type ProcessResult = Result<(), ProcessError>;
