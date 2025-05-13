use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum GenerateEmbedError {
    ImageSourceUrl(#[from] twilight_util::builder::embed::image_source::ImageSourceUrlError),
    EmbedValidation(#[from] twilight_validate::embed::EmbedValidationError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UpdateEmbedError {
    Http(#[from] twilight_http::Error),
    EmbedValidation(#[from] twilight_validate::embed::EmbedValidationError),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    Followup(#[from] crate::error::core::FollowupError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum StartPollError {
    GenerateEmbed(#[from] GenerateEmbedError),
    Cache(#[from] crate::error::Cache),
    TwilightHttp(#[from] twilight_http::Error),
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
    WaitForVotes(#[from] WaitForVotesError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum WaitForVotesError {
    TwilightHttp(#[from] twilight_http::Error),
    UpdateEmbed(#[from] UpdateEmbedError),
    EventRecv(#[from] tokio::sync::broadcast::error::RecvError),
}
