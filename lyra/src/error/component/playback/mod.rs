use thiserror::Error;

pub use play_pause::Error as PlayPauseError;

#[derive(Error, Debug)]
#[error("handling `VoiceStateUpdate` failed: {:?}", .0)]
pub enum HandleVoiceStateUpdateError {
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    TwilightHttp(#[from] twilight_http::Error),
    SetPauseWith(#[from] crate::error::command::require::SetPauseWithError),
}

pub mod back {
    use thiserror::Error;

    use crate::error::command::check::UsersTrackError;

    #[derive(Error, Debug)]
    #[error(transparent)]
    pub enum BackError {
        NotInVoice(#[from] crate::error::NotInVoice),
        Unsuppressed(#[from] crate::error::command::require::UnsuppressedError),
        InVoiceWithoutUser(#[from] crate::error::InVoiceWithoutUser),
        UsersTrack(#[from] UsersTrackError),
        Respond(#[from] crate::error::core::RespondError),
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
    }
}

pub mod skip {
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error(transparent)]
    pub enum SkipError {
        NotPlaying(#[from] crate::error::NotPlaying),
        NotInVoice(#[from] crate::error::NotInVoice),
        Unsuppressed(#[from] crate::error::command::require::UnsuppressedError),
        InVoiceWithoutUser(#[from] crate::error::InVoiceWithoutUser),
        UsersTrackError(#[from] crate::error::command::check::UsersTrackError),
        Respond(#[from] crate::error::core::RespondError),
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
    }
}

pub mod play_pause {
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error(transparent)]
    pub enum Error {
        NotInVoice(#[from] crate::error::NotInVoice),
        Unsuppressed(#[from] crate::error::command::require::UnsuppressedError),
        InVoiceWithoutUser(#[from] crate::error::InVoiceWithoutUser),
        UserOnlyIn(#[from] crate::error::command::check::UserOnlyInError),
        NotPlaying(#[from] crate::error::NotPlaying),
        UsersTrack(#[from] crate::error::command::check::UsersTrackError),
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
        Respond(#[from] crate::error::core::RespondError),
        SetPauseWith(#[from] crate::error::command::require::SetPauseWithError),
    }
}
