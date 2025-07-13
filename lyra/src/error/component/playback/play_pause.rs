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
