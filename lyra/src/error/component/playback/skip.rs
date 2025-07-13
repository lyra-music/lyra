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
