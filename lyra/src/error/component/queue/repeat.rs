#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    Respond(#[from] crate::error::core::RespondError),
    UnrecognisedConnection(#[from] crate::error::UnrecognisedConnection),
    UpdateNowPlayingMessage(#[from] crate::error::lavalink::UpdateNowPlayingMessageError),
}
