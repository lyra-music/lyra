use thiserror::Error;

#[derive(Error, Debug)]
#[error("creating a followup failed: {}", .0)]
pub enum FollowupError {
    TwilightHttp(#[from] twilight_http::Error),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
}

pub type RespondResult<T> = Result<T, twilight_http::Error>;
pub type FollowupResult<T> = Result<T, FollowupError>;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum DeserialiseBodyFromHttpError {
    TwilightHttp(#[from] twilight_http::Error),
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum RegisterGlobalCommandsError {
    TwilightHttp(#[from] twilight_http::Error),
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
}
