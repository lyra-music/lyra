#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ShuffleError {
    NotInVoice(#[from] crate::error::NotInVoice),
    InVoiceWithoutUser(#[from] crate::error::InVoiceWithoutUser),
    UserOnlyIn(#[from] crate::error::command::check::UserOnlyInError),
    UpdateNowPlayingMessage(#[from] crate::error::lavalink::UpdateNowPlayingMessageError),
    Respond(#[from] crate::error::core::RespondError),
}
