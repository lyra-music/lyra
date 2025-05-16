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
    Respond(#[from] crate::error::core::RespondError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum StartPollError {
    GenerateEmbed(#[from] GenerateEmbedError),
    Cache(#[from] crate::error::Cache),
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
    WaitForVotes(#[from] WaitForVotesError),
    Respond(#[from] crate::error::core::RespondError),
    DeserialiseBodyFromHttp(#[from] crate::error::core::DeserialiseBodyFromHttpError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum WaitForVotesError {
    Respond(#[from] crate::error::core::RespondError),
    UpdateEmbed(#[from] UpdateEmbedError),
    EventRecv(#[from] tokio::sync::broadcast::error::RecvError),
}
