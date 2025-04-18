use thiserror::Error;
use twilight_model::id::{Id, marker::MessageMarker};

#[derive(Error, Debug)]
#[error(transparent)]
pub enum PromptForConfirmationError {
    StandbyCanceled(#[from] twilight_standby::future::Canceled),
    Respond(#[from] super::RespondError),
    ConfirmationTimedout(#[from] crate::error::ConfirmationTimedOut),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualGetUsersVoiceChannelError {
    Cache(#[from] crate::error::Cache),
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
    Cache(#[from] crate::error::Cache),
    GatewaySend(#[from] twilight_gateway::error::ChannelError),
    TwilightHttp(#[from] twilight_http::Error),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
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
    HandleResponse(#[from] crate::error::component::connection::join::HandleResponseError),
    Failed(crate::error::AutoJoinAttemptFailed),
}

impl AutoJoinAttemptError {
    const fn from_get_users_voice_channel(
        error: crate::error::component::connection::join::GetUsersVoiceChannelError,
    ) -> Self {
        match error {
            crate::error::component::connection::join::GetUsersVoiceChannelError::UserNotInVoice(e) => {
                Self::Failed(crate::error::AutoJoinAttemptFailed::UserNotInVoice(e))
            },
            crate::error::component::connection::join::GetUsersVoiceChannelError::Cache(e) => {
                Self::ImplAutoJoin(ResidualImplAutoJoinError::GetUsersVoiceChannel(ResidualGetUsersVoiceChannelError::Cache(e)))
            },
        }
    }

    fn from_check_user_allowed(error: super::check::UserAllowedError) -> Self {
        match error {
            super::check::UserAllowedError::UserNotAllowed(e) => {
                Self::Failed(crate::error::AutoJoinAttemptFailed::UserNotAllowed(e))
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
        error: crate::error::component::connection::join::ImplConnectToError,
    ) -> Self {
        match error {
            crate::error::component::connection::join::ImplConnectToError::Lavalink(e) => {
                Self::ImplAutoJoin(ResidualImplAutoJoinError::ConnectToNew(
                    ResidualConnectToNewError::ImplConnectTo(ResidualImplConnectToError::Lavalink(
                        e,
                    )),
                ))
            }
            crate::error::component::connection::join::ImplConnectToError::Forbidden(e) => {
                Self::Failed(crate::error::AutoJoinAttemptFailed::Forbidden(e))
            }
            crate::error::component::connection::join::ImplConnectToError::Cache(e) => {
                Self::ImplAutoJoin(ResidualImplAutoJoinError::ConnectToNew(
                    ResidualConnectToNewError::ImplConnectTo(ResidualImplConnectToError::Cache(e)),
                ))
            }
            crate::error::component::connection::join::ImplConnectToError::GatewaySend(e) => {
                Self::ImplAutoJoin(ResidualImplAutoJoinError::ConnectToNew(
                    ResidualConnectToNewError::ImplConnectTo(
                        ResidualImplConnectToError::GatewaySend(e),
                    ),
                ))
            }
            crate::error::component::connection::join::ImplConnectToError::TwilightHttp(e) => {
                Self::ImplAutoJoin(ResidualImplAutoJoinError::ConnectToNew(
                    ResidualConnectToNewError::ImplConnectTo(
                        ResidualImplConnectToError::TwilightHttp(e),
                    ),
                ))
            }
            crate::error::component::connection::join::ImplConnectToError::UnrecognisedConnection(_) => {
                // SAFETY: if an auto-join was performed, then the `require::in_voice(_)` call was unsuccessful,
                // which is impossible as this error will only be raised if there is an unrecognised connection found.
                unsafe { std::hint::unreachable_unchecked() }
            },
            crate::error::component::connection::join::ImplConnectToError::CheckUserAllowed(e) => {
                Self::from_check_user_allowed(e)
            }
        }
    }

    fn from_connect_to_new(
        error: crate::error::component::connection::join::ConnectToNewError,
    ) -> Self {
        match error {
            crate::error::component::connection::join::ConnectToNewError::UserNotStageManager(
                e,
            ) => Self::Failed(crate::error::AutoJoinAttemptFailed::UserNotStageManager(e)),
            crate::error::component::connection::join::ConnectToNewError::ImplConnectTo(e) => {
                Self::from_impl_connect_to(e)
            }
        }
    }

    fn from_impl_auto_join(
        error: crate::error::component::connection::join::ImplAutoJoinError,
    ) -> Self {
        match error {
            crate::error::component::connection::join::ImplAutoJoinError::GetUsersVoiceChannel(
                e,
            ) => Self::from_get_users_voice_channel(e),
            crate::error::component::connection::join::ImplAutoJoinError::ConnectToNew(e) => {
                Self::from_connect_to_new(e)
            }
        }
    }
}

impl crate::error::component::connection::join::AutoJoinError {
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
    InVoiceWithoutUser(#[from] crate::error::InVoiceWithoutUser),
    RequireUnsuppressed(#[from] super::require::UnsuppressedError),
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
