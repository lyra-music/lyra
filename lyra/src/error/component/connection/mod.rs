pub mod join;
pub mod leave;

use thiserror::Error;

#[derive(Error, Debug)]
#[error("starting inactivity timeout failed: {:?}", .0)]
pub enum StartInactivityTimeoutError {
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::lavalink::Event>),
    GatewaySend(#[from] twilight_gateway::error::ChannelError),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    Http(#[from] twilight_http::Error),
    DisconnectCleanup(#[from] leave::DisconnectCleanupError),
    UnrecognisedConnection(#[from] crate::error::UnrecognisedConnection),
}

#[derive(Error, Debug)]
#[error("handling `VoiceStateUpdate` failed: {:?}", .0)]
pub enum HandleVoiceStateUpdateError {
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::lavalink::Event>),
    Http(#[from] twilight_http::Error),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    MatchStateChannelID(#[from] MatchStateChannelIdError),
    DisconnectCleanup(#[from] leave::DisconnectCleanupError),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    SetPauseWith(#[from] crate::error::command::require::SetPauseWithError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum MatchStateChannelIdError {
    Http(#[from] twilight_http::Error),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    Cache(#[from] crate::error::Cache),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
}
