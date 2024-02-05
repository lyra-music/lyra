use thiserror::Error;
use twilight_model::id::{marker::MessageMarker, Id};

#[derive(Error, Debug)]
#[error(transparent)]
pub enum PromptForConfirmationError {
    StandbyCanceled(#[from] twilight_standby::future::Canceled),
    Respond(#[from] super::RespondError),
    Confirmation(#[from] ConfirmationError),
}

#[derive(Error, Debug)]
pub enum ConfirmationError {
    #[error("confirmation cancelled")]
    Cancelled,
    #[error("confirmation timed out")]
    TimedOut,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualGetUsersVoiceChannelError {
    Cache(#[from] crate::bot::error::Cache),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualUserAllowedError {
    AccessCalculatorBuild(#[from] super::check::AccessCalculatorBuildError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualImplConnectToError {
    CheckUserAllowed(#[from] ResidualUserAllowedError),
    Cache(#[from] crate::bot::error::Cache),
    GatewaySend(#[from] twilight_gateway::error::SendError),
    TwilightHttp(#[from] twilight_http::Error),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualConnectToNewError {
    ImplConnectTo(#[from] ResidualImplConnectToError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualImplAutoJoinError {
    GetUsersVoiceChannel(#[from] ResidualGetUsersVoiceChannelError),
    ConnectToNew(#[from] ResidualConnectToNewError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum AutoJoinAttemptError {
    ImplAutoJoin(#[from] ResidualImplAutoJoinError),
    HandleResponse(#[from] crate::bot::error::component::connection::join::HandleResponseError),
    Failed(crate::bot::error::AutoJoinAttemptFailed),
}

impl AutoJoinAttemptError {
    const fn from_get_users_voice_channel(
        error: crate::bot::error::component::connection::join::GetUsersVoiceChannelError,
    ) -> Self {
        match error {
            crate::bot::error::component::connection::join::GetUsersVoiceChannelError::UserNotInVoice(e) => {
                Self::Failed(crate::bot::error::AutoJoinAttemptFailed::UserNotInVoice(e))
            },
            crate::bot::error::component::connection::join::GetUsersVoiceChannelError::Cache(e) => {
                Self::ImplAutoJoin(ResidualImplAutoJoinError::GetUsersVoiceChannel(ResidualGetUsersVoiceChannelError::Cache(e)))
            },
        }
    }

    fn from_check_user_allowed(error: super::check::UserAllowedError) -> Self {
        match error {
            super::check::UserAllowedError::UserNotAllowed(e) => {
                Self::Failed(crate::bot::error::AutoJoinAttemptFailed::UserNotAllowed(e))
            }
            super::check::UserAllowedError::AccessCalculatorBuild(e) => Self::ImplAutoJoin(
                ResidualImplAutoJoinError::ConnectToNew(ResidualConnectToNewError::ImplConnectTo(
                    ResidualImplConnectToError::CheckUserAllowed(
                        ResidualUserAllowedError::AccessCalculatorBuild(e),
                    ),
                )),
            ),
        }
    }

    fn from_impl_connect_to(
        error: crate::bot::error::component::connection::join::ImplConnectToError,
    ) -> Self {
        match error {
            crate::bot::error::component::connection::join::ImplConnectToError::Forbidden(e) => {
                Self::Failed(crate::bot::error::AutoJoinAttemptFailed::Forbidden(e))
            },
            crate::bot::error::component::connection::join::ImplConnectToError::Cache(e) => {
                Self::ImplAutoJoin(ResidualImplAutoJoinError::ConnectToNew(ResidualConnectToNewError::ImplConnectTo(ResidualImplConnectToError::Cache(e))))
            },
            crate::bot::error::component::connection::join::ImplConnectToError::GatewaySend(e) => {
                Self::ImplAutoJoin(ResidualImplAutoJoinError::ConnectToNew(ResidualConnectToNewError::ImplConnectTo(ResidualImplConnectToError::GatewaySend(e))))
            },
            crate::bot::error::component::connection::join::ImplConnectToError::TwilightHttp(e) => {
                Self::ImplAutoJoin(ResidualImplAutoJoinError::ConnectToNew(ResidualConnectToNewError::ImplConnectTo(ResidualImplConnectToError::TwilightHttp(e))))
            },
            crate::bot::error::component::connection::join::ImplConnectToError::CheckUserAllowed(e) => {
                Self::from_check_user_allowed(e)
            },
        }
    }

    fn from_connect_to_new(
        error: crate::bot::error::component::connection::join::ConnectToNewError,
    ) -> Self {
        match error {
            crate::bot::error::component::connection::join::ConnectToNewError::UserNotStageManager(e) => {
                Self::Failed(crate::bot::error::AutoJoinAttemptFailed::UserNotStageManager(e))
            },
            crate::bot::error::component::connection::join::ConnectToNewError::ImplConnectTo(e) => {
                Self::from_impl_connect_to(e)
            },
        }
    }

    fn from_impl_auto_join(
        error: crate::bot::error::component::connection::join::ImplAutoJoinError,
    ) -> Self {
        match error {
            crate::bot::error::component::connection::join::ImplAutoJoinError::GetUsersVoiceChannel(e) => {
                Self::from_get_users_voice_channel(e)
            },
            crate::bot::error::component::connection::join::ImplAutoJoinError::ConnectToNew(e) => {
                Self::from_connect_to_new(e)
            },
        }
    }
}

impl crate::bot::error::component::connection::join::AutoJoinError {
    pub fn unflatten_into_auto_join_attempt(self) -> AutoJoinAttemptError {
        match self {
            Self::ImplAutoJoin(e) => AutoJoinAttemptError::from_impl_auto_join(e),
            Self::HandleResponse(e) => AutoJoinAttemptError::HandleResponse(e),
        }
    }
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum AutoJoinOrCheckInVoiceWithUserError {
    InVoiceWithoutUser(#[from] crate::bot::error::InVoiceWithoutUser),
    CheckNotSuppressed(#[from] super::check::NotSuppressedError),
    AutoJoinAttempt(#[from] AutoJoinAttemptError),
    HandleSuppressedAutoJoin(#[from] HandleSuppressedAutoJoinError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum HandleSuppressedAutoJoinError {
    DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
    FollowUp(#[from] super::FollowupError),
    AutoJoinSuppressed(#[from] AutoJoinSuppressedError),
}

#[derive(Error, Debug)]
pub enum AutoJoinSuppressedError {
    #[error("bot is server muted")]
    Muted,
    #[error("bot has still not become a speaker in stage")]
    StillNotSpeaker { last_followup_id: Id<MessageMarker> },
}
