use thiserror::Error;

#[derive(Error, Debug)]
#[error("starting bot failed: {:?}", .0)]
pub enum StartError {
    StartRecommended(#[from] twilight_gateway::stream::StartRecommendedError),
    Sqlx(#[from] sqlx::Error),
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
    Http(#[from] twilight_http::Error),
    NodeError(#[from] twilight_lavalink::node::NodeError),
    Send(#[from] tokio::sync::watch::error::SendError<bool>),
    WaitForShutdown(#[from] WaitForShutdownError),
    DeserializeBodyFromHttp(#[from] super::core::DeserializeBodyFromHttpError),
}

#[derive(Error, Debug)]
pub enum WaitForShutdownError {
    #[error("unable to register handler: {:?}", .0)]
    Io(#[from] std::io::Error),
}
