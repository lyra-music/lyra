use thiserror::Error;

#[derive(Error, Debug)]
#[error("starting bot failed: {:?}", .0)]
pub enum StartError {
    StartRecommended(#[from] twilight_gateway::stream::StartRecommendedError),
    Sqlx(#[from] sqlx::Error),
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
    Http(#[from] twilight_http::Error),
    WaitUntilShutdown(#[from] WaitUntilShutdownError),
    DeserializeBodyFromHttp(#[from] super::core::DeserializeBodyFromHttpError),
    RegisterGlobalCommands(#[from] super::core::RegisterGlobalCommandsError),
}

#[derive(Error, Debug)]
pub enum WaitForSignalError {
    #[error("unable to register handler: {:?}", .0)]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum WaitUntilShutdownError {
    WaitForSignal(#[from] WaitForSignalError),
    Send(#[from] tokio::sync::watch::error::SendError<bool>),
}
