#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum DisconnectCleanupError {
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::lavalink::Event>),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
}

#[derive(thiserror::Error, Debug)]
#[error("leaving voice failed: {}", .0)]
pub enum Error {
    NotInVoice(#[from] crate::error::NotInVoice),
    InVoiceWithoutUser(#[from] crate::error::InVoiceWithoutUser),
    CheckUserOnlyIn(#[from] crate::error::command::check::UserOnlyInError),
    DisconnectCleanup(#[from] DisconnectCleanupError),
    GatewaySend(#[from] twilight_gateway::error::ChannelError),
    UnrecognisedConnection(#[from] crate::error::UnrecognisedConnection),
}

impl Error {
    pub fn match_not_in_voice_into(self) -> NotInVoiceMatchedError {
        match self {
            Self::NotInVoice(e) => NotInVoiceMatchedError::NotInVoice(e),
            Self::InVoiceWithoutUser(e) => {
                NotInVoiceMatchedError::Other(ResidualError::InVoiceWithoutUser(e))
            }
            Self::CheckUserOnlyIn(e) => {
                NotInVoiceMatchedError::Other(ResidualError::CheckUserOnlyIn(e))
            }
            Self::DisconnectCleanup(e) => {
                NotInVoiceMatchedError::Other(ResidualError::DisconnectCleanupError(e))
            }
            Self::GatewaySend(e) => NotInVoiceMatchedError::Other(ResidualError::GatewaySend(e)),
            Self::UnrecognisedConnection(e) => {
                NotInVoiceMatchedError::Other(ResidualError::UnrecognisedConnection(e))
            }
        }
    }
}

pub enum NotInVoiceMatchedError {
    NotInVoice(crate::error::NotInVoice),
    Other(ResidualError),
}

#[derive(thiserror::Error, Debug)]
#[error("leaving voice failed: {}", .0)]
pub enum ResidualError {
    InVoiceWithoutUser(#[from] crate::error::InVoiceWithoutUser),
    CheckUserOnlyIn(#[from] crate::error::command::check::UserOnlyInError),
    DisconnectCleanupError(#[from] DisconnectCleanupError),
    GatewaySend(#[from] twilight_gateway::error::ChannelError),
    UnrecognisedConnection(#[from] crate::error::UnrecognisedConnection),
}
