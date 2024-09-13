use thiserror::Error;

#[derive(Error, Debug)]
#[error("handling `VoiceStateUpdate` failed: {:?}", .0)]
pub enum HandleVoiceStateUpdateError {
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    TwilightHttp(#[from] twilight_http::Error),
}
