use thiserror::Error;

#[derive(Error, Debug)]
#[error("handling confirmation error failed: {:?}", .0)]
pub enum MatchConfirmationError {
    Http(#[from] twilight_http::Error),
    Followup(#[from] super::core::FollowupError),
}

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error(transparent)]
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::bot::lavalink::Event>),
    #[error(transparent)]
    DeserializeBodyFromHttp(#[from] super::core::DeserializeBodyFromHttpError),
    #[error(transparent)]
    DeserializeBodyFromHttpArc(#[from] std::sync::Arc<super::core::DeserializeBodyFromHttpError>),
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
    HandleVoiceStateUpdate(#[from] super::component::connection::HandleVoiceStateUpdateError),
    #[error(transparent)]
    MatchConfirmation(#[from] MatchConfirmationError),
    #[error(transparent)]
    Respond(#[from] super::command::RespondError),
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
