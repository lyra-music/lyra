use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum BackError {
    NotInVoice(#[from] crate::error::NotInVoice),
    Unsuppressed(#[from] crate::error::command::require::UnsuppressedError),
    InVoiceWithoutUser(#[from] crate::error::InVoiceWithoutUser),
    UserOnlyIn(#[from] crate::error::command::check::UserOnlyInError),
    Respond(#[from] crate::error::core::RespondError),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
}
