pub mod play;
pub mod repeat;
pub mod shuffle;

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum RemoveTracksError {
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    DeserialiseBodyFromHttp(#[from] crate::error::core::DeserialiseBodyFromHttpError),
    UpdateNowPlayingMessage(#[from] crate::error::lavalink::UpdateNowPlayingMessageError),
    Respond(#[from] crate::error::core::RespondError),
    RespondOrFollowup(#[from] crate::error::core::RespondOrFollowupError),
}
