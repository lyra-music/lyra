use thiserror::Error;

pub use play_pause::Error as PlayPauseError;

#[derive(Error, Debug)]
#[error("handling `VoiceStateUpdate` failed: {:?}", .0)]
pub enum HandleVoiceStateUpdateError {
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    TwilightHttp(#[from] twilight_http::Error),
    SetPauseWith(#[from] crate::error::command::require::SetPauseWithError),
}

pub mod play_pause {
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error(transparent)]
    pub enum Error {
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
        Respond(#[from] crate::error::command::RespondError),
        SetPauseWith(#[from] crate::error::command::require::SetPauseWithError),
    }
}
