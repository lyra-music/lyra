use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error(transparent)]
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::lavalink::Event>),
    #[error(transparent)]
    DeserialiseBodyFromHttp(#[from] super::core::DeserialiseBodyFromHttpError),
    #[error(transparent)]
    Http(#[from] twilight_http::Error),
    #[error(transparent)]
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    CoreFollowup(#[from] super::core::FollowupError),
    #[error(transparent)]
    Cache(#[from] super::Cache),
    #[error(transparent)]
    ConnectionHandleVoiceStateUpdate(
        #[from] super::component::connection::HandleVoiceStateUpdateError,
    ),
    #[error(transparent)]
    PlaybackHandleVoiceStateUpdate(#[from] super::component::playback::HandleVoiceStateUpdateError),
    #[error(transparent)]
    Respond(#[from] super::command::RespondError),
    #[error(transparent)]
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    #[error(transparent)]
    PlayPause(#[from] super::component::playback::PlayPauseError),
    #[error(transparent)]
    Repeat(#[from] super::component::queue::RepeatError),
    #[error("error executing command `/{}`: {:?}", .name, .source)]
    CommandExecute {
        name: Box<str>,
        source: super::command::declare::CommandExecuteError,
    },
    #[error("error executing autocomplete for command `/{}`: {:?}", .name, .source)]
    AutocompleteExecute {
        name: Box<str>,
        source: super::command::declare::AutocompleteExecuteError,
    },
}

pub type ProcessResult = Result<(), ProcessError>;
