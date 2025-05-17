use crate::error::command::check;

#[derive(thiserror::Error, Debug)]
#[error("deleting empty voice notice failed: {:?}", .0)]
pub enum DeleteEmptyVoiceNoticeError {
    Http(#[from] twilight_http::Error),
    StandbyDropped(#[from] twilight_standby::future::Canceled),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum GetUsersVoiceChannelError {
    UserNotInVoice(#[from] crate::error::UserNotInVoice),
    Cache(#[from] crate::error::Cache),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ImplConnectToError {
    Forbidden(#[from] crate::error::ConnectionForbidden),
    CheckUserAllowed(#[from] check::UserAllowedError),
    Cache(#[from] crate::error::Cache),
    GatewaySend(#[from] twilight_gateway::error::ChannelError),
    TwilightHttp(#[from] twilight_http::Error),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    UnrecognisedConnection(#[from] crate::error::UnrecognisedConnection),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ConnectToError {
    UserNotStageModerator(#[from] crate::error::UserNotStageModerator),
    InVoiceAlready(#[from] crate::error::InVoiceAlready),
    CheckUserOnlyIn(#[from] check::UserOnlyInError),
    ImplConnectTo(#[from] ImplConnectToError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ConnectToNewError {
    UserNotStageModerator(#[from] crate::error::UserNotStageModerator),
    ImplConnectTo(#[from] ImplConnectToError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ImplJoinError {
    GetUsersVoiceChannel(#[from] GetUsersVoiceChannelError),
    ConnectTo(#[from] ConnectToError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ImplAutoJoinError {
    GetUsersVoiceChannel(#[from] GetUsersVoiceChannelError),
    ConnectToNew(#[from] ConnectToNewError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum HandleResponseError {
    Cache(#[from] crate::error::Cache),
    Respond(#[from] crate::error::core::RespondError),
    RespondOrFollowup(#[from] crate::error::core::RespondOrFollowupError),
    DeserialiseBodyFromHttp(#[from] crate::error::core::DeserialiseBodyFromHttpError),
}

#[derive(thiserror::Error, Debug)]
#[error("joining voice failed: {:?}", .0)]
pub enum AutoJoinError {
    ImplAutoJoin(#[from] ImplAutoJoinError),
    HandleResponse(#[from] HandleResponseError),
}

#[derive(thiserror::Error, Debug)]
#[error("joining voice failed: {:?}", .0)]
pub enum Error {
    ImplJoin(#[from] ImplJoinError),
    HandleResponse(#[from] HandleResponseError),
}

impl Error {
    pub fn flatten_partially_into(self) -> PartiallyFlattenedError {
        match self {
            Self::HandleResponse(e) => PartiallyFlattenedError::from_handle_response(e),
            Self::ImplJoin(e) => PartiallyFlattenedError::from_impl_join(e),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("joining voice failed: {:?}", .0)]
pub enum ResidualError {
    ImplJoin(#[from] ResidualImplJoinError),
    HandleResponse(#[from] HandleResponseError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualImplJoinError {
    GetUsersVoiceChannel(#[from] ResidualGetUsersVoiceChannelError),
    ConnectTo(#[from] ResidualConnectToError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualGetUsersVoiceChannelError {
    Cache(#[from] crate::error::Cache),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualConnectToError {
    CheckUserOnlyIn(#[from] check::UserOnlyInError),
    ImplConnectTo(#[from] ResidualImplConnectToError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualImplConnectToError {
    CheckUserAllowed(#[from] ResidualUserAllowedError),
    Cache(#[from] crate::error::Cache),
    GatewaySend(#[from] twilight_gateway::error::ChannelError),
    TwilightHttp(#[from] twilight_http::Error),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    UnrecognisedConnection(#[from] crate::error::UnrecognisedConnection),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ResidualUserAllowedError {
    AccessCalculatorBuild(#[from] check::AccessCalculatorBuildError),
}

pub enum PartiallyFlattenedError {
    UserNotInVoice(crate::error::UserNotInVoice),
    UserNotStageModerator(crate::error::UserNotStageModerator),
    UserNotAllowed(crate::error::UserNotAllowed),
    InVoiceAlready(crate::error::InVoiceAlready),
    Forbidden(crate::error::ConnectionForbidden),
    Other(ResidualError),
}

pub use PartiallyFlattenedError as Pfe;

impl PartiallyFlattenedError {
    fn from_handle_response(error: HandleResponseError) -> Self {
        match error {
            HandleResponseError::Cache(e) => {
                Self::Other(ResidualError::HandleResponse(HandleResponseError::Cache(e)))
            }
            HandleResponseError::RespondOrFollowup(e) => Self::Other(
                ResidualError::HandleResponse(HandleResponseError::RespondOrFollowup(e)),
            ),
            HandleResponseError::DeserialiseBodyFromHttp(e) => Self::Other(
                ResidualError::HandleResponse(HandleResponseError::DeserialiseBodyFromHttp(e)),
            ),
            HandleResponseError::Respond(e) => Self::Other(ResidualError::HandleResponse(
                HandleResponseError::Respond(e),
            )),
        }
    }

    fn from_impl_join(error: ImplJoinError) -> Self {
        match error {
            ImplJoinError::GetUsersVoiceChannel(e) => Self::from_get_users_voice_channel(e),
            ImplJoinError::ConnectTo(e) => Self::from_connect_to(e),
        }
    }

    const fn from_get_users_voice_channel(error: GetUsersVoiceChannelError) -> Self {
        match error {
            GetUsersVoiceChannelError::UserNotInVoice(e) => Self::UserNotInVoice(e),
            GetUsersVoiceChannelError::Cache(e) => Self::Other(ResidualError::ImplJoin(
                ResidualImplJoinError::GetUsersVoiceChannel(
                    ResidualGetUsersVoiceChannelError::Cache(e),
                ),
            )),
        }
    }

    fn from_impl_connect_to(error: ImplConnectToError) -> Self {
        match error {
            ImplConnectToError::Forbidden(e) => Self::Forbidden(e),
            ImplConnectToError::Cache(e) => {
                Self::Other(ResidualError::ImplJoin(ResidualImplJoinError::ConnectTo(
                    ResidualConnectToError::ImplConnectTo(ResidualImplConnectToError::Cache(e)),
                )))
            }
            ImplConnectToError::GatewaySend(e) => Self::Other(ResidualError::ImplJoin(
                ResidualImplJoinError::ConnectTo(ResidualConnectToError::ImplConnectTo(
                    ResidualImplConnectToError::GatewaySend(e),
                )),
            )),
            ImplConnectToError::TwilightHttp(e) => Self::Other(ResidualError::ImplJoin(
                ResidualImplJoinError::ConnectTo(ResidualConnectToError::ImplConnectTo(
                    ResidualImplConnectToError::TwilightHttp(e),
                )),
            )),
            ImplConnectToError::Lavalink(e) => {
                Self::Other(ResidualError::ImplJoin(ResidualImplJoinError::ConnectTo(
                    ResidualConnectToError::ImplConnectTo(ResidualImplConnectToError::Lavalink(e)),
                )))
            }
            ImplConnectToError::UnrecognisedConnection(e) => Self::Other(ResidualError::ImplJoin(
                ResidualImplJoinError::ConnectTo(ResidualConnectToError::ImplConnectTo(
                    ResidualImplConnectToError::UnrecognisedConnection(e),
                )),
            )),
            ImplConnectToError::CheckUserAllowed(e) => Self::from_check_user_allowed(e),
        }
    }

    fn from_connect_to(error: ConnectToError) -> Self {
        match error {
            ConnectToError::UserNotStageModerator(e) => Self::UserNotStageModerator(e),
            ConnectToError::InVoiceAlready(e) => Self::InVoiceAlready(e),
            ConnectToError::CheckUserOnlyIn(e) => Self::from_check_user_only_in(e),
            ConnectToError::ImplConnectTo(e) => Self::from_impl_connect_to(e),
        }
    }

    fn from_check_user_allowed(error: check::UserAllowedError) -> Self {
        match error {
            check::UserAllowedError::AccessCalculatorBuild(e) => {
                Self::Other(ResidualError::ImplJoin(ResidualImplJoinError::ConnectTo(
                    ResidualConnectToError::ImplConnectTo(
                        ResidualImplConnectToError::CheckUserAllowed(
                            ResidualUserAllowedError::AccessCalculatorBuild(e),
                        ),
                    ),
                )))
            }
            check::UserAllowedError::UserNotAllowed(e) => Self::UserNotAllowed(e),
        }
    }

    const fn from_check_user_only_in(error: check::UserOnlyInError) -> Self {
        match error {
            check::UserOnlyInError::Cache(e) => {
                Self::Other(ResidualError::ImplJoin(ResidualImplJoinError::ConnectTo(
                    ResidualConnectToError::CheckUserOnlyIn(check::UserOnlyInError::Cache(e)),
                )))
            }
            check::UserOnlyInError::InVoiceWithSomeoneElse(e) => {
                Self::Other(ResidualError::ImplJoin(ResidualImplJoinError::ConnectTo(
                    ResidualConnectToError::CheckUserOnlyIn(
                        check::UserOnlyInError::InVoiceWithSomeoneElse(e),
                    ),
                )))
            }
        }
    }
}
