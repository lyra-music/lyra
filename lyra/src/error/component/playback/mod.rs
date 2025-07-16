use thiserror::Error;

pub use play_pause::Error as PlayPauseError;

#[derive(Error, Debug)]
#[error("handling `VoiceStateUpdate` failed: {:?}", .0)]
pub enum HandleVoiceStateUpdateError {
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    TwilightHttp(#[from] twilight_http::Error),
    SetPauseWith(#[from] crate::error::command::require::SetPauseWithError),
}

pub mod back;
pub mod play_pause;
pub mod skip;
