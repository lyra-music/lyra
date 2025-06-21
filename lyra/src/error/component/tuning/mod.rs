use thiserror::Error;

use crate::error::{NotInVoice, command::require::UnsuppressedError, lavalink::NoPlayerError};

#[derive(Error, Debug)]
#[error(transparent)]
pub enum RequireInVoiceUnsuppressedAndPlayerError {
    NotInVoice(#[from] NotInVoice),
    Unsuppressed(#[from] UnsuppressedError),
    NoPlayer(#[from] NoPlayerError),
}
