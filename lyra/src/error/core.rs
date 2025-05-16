use thiserror::Error;

#[derive(Error, Debug)]
#[error("creating a followup failed: {}", .0)]
pub enum FollowupError {
    TwilightHttp(#[from] twilight_http::Error),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum DeserialiseBodyFromHttpError {
    TwilightHttp(#[from] twilight_http::Error),
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum SetGlobalCommandsError {
    TwilightHttp(#[from] twilight_http::Error),
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
}

#[derive(Error, Debug)]
pub enum RespondError {
    #[error(transparent)]
    TwilightHttp(#[from] twilight_http::Error),
    #[error(transparent)]
    Builder(#[from] super::BuildError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum RespondOrFollowupError {
    Respond(#[from] RespondError),
    Followup(#[from] twilight_http::Error),
}
