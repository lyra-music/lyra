pub mod component;

use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum ProcessError {
    SetGlobalCommands(#[from] super::core::SetGlobalCommandsError),
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::lavalink::Event>),
    Http(#[from] twilight_http::Error),
    Respond(#[from] super::core::RespondError),
    RespondOrFollowup(#[from] super::core::RespondOrFollowupError),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    Sqlx(#[from] sqlx::Error),
    CoreFollowup(#[from] super::core::FollowupError),
    Cache(#[from] super::Cache),
    ConnectionHandleVoiceStateUpdate(
        #[from] super::component::connection::HandleVoiceStateUpdateError,
    ),
    PlaybackHandleVoiceStateUpdate(#[from] super::component::playback::HandleVoiceStateUpdateError),
    #[error("error executing command `/{}`: {:?}", .name, .source)]
    CommandExecute {
        name: Box<str>,
        source: super::command::declare::CommandExecuteError,
    },
    #[error("error executing controller `{}`: {:?}", .kind, .source)]
    ControllerExecute {
        kind: crate::core::statik::component::NowPlayingButtonType,
        source: component::ControllerError,
    },
    #[error("error executing autocomplete for command `/{}`: {:?}", .name, .source)]
    AutocompleteExecute {
        name: Box<str>,
        source: super::command::declare::AutocompleteExecuteError,
    },
}

pub type ProcessResult = Result<(), ProcessError>;
