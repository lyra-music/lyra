pub mod join {

    use crate::bot::error::command::check;

    #[derive(thiserror::Error, Debug)]
    #[error("deleting empty voice notice failed: {:?}", .0)]
    pub enum DeleteEmptyVoiceNoticeError {
        Http(#[from] twilight_http::Error),
        StandbyDropped(#[from] twilight_standby::future::Canceled),
        DeserializeBodyFromHttp(#[from] crate::bot::error::core::DeserializeBodyFromHttpError),
    }

    #[derive(thiserror::Error, Debug)]
    #[error(transparent)]
    pub enum GetUsersVoiceChannelError {
        UserNotInVoice(#[from] crate::bot::error::UserNotInVoice),
        Cache(#[from] crate::bot::error::Cache),
    }

    #[derive(thiserror::Error, Debug)]
    #[error(transparent)]
    pub enum ImplConnectToError {
        Forbidden(#[from] crate::bot::error::ConnectionForbidden),
        CheckUserAllowed(#[from] check::UserAllowedError),
        Cache(#[from] crate::bot::error::Cache),
        GatewaySend(#[from] twilight_gateway::error::ChannelError),
        TwilightHttp(#[from] twilight_http::Error),
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
    }

    #[derive(thiserror::Error, Debug)]
    #[error(transparent)]
    pub enum ConnectToError {
        UserNotStageManager(#[from] crate::bot::error::UserNotStageManager),
        InVoiceAlready(#[from] crate::bot::error::InVoiceAlready),
        CheckUserOnlyIn(#[from] check::UserOnlyInError),
        ImplConnectTo(#[from] ImplConnectToError),
    }

    #[derive(thiserror::Error, Debug)]
    #[error(transparent)]
    pub enum ConnectToNewError {
        UserNotStageManager(#[from] crate::bot::error::UserNotStageManager),
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
        Cache(#[from] crate::bot::error::Cache),
        Respond(#[from] crate::bot::error::command::RespondError),
        DeserializeBody(#[from] twilight_http::response::DeserializeBodyError),
        Followup(#[from] crate::bot::error::command::FollowupError),
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
        Cache(#[from] crate::bot::error::Cache),
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
        Cache(#[from] crate::bot::error::Cache),
        GatewaySend(#[from] twilight_gateway::error::ChannelError),
        TwilightHttp(#[from] twilight_http::Error),
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
    }

    #[derive(thiserror::Error, Debug)]
    #[error(transparent)]
    pub enum ResidualUserAllowedError {
        AccessCalculatorBuild(#[from] check::AccessCalculatorBuildError),
    }

    pub enum PartiallyFlattenedError {
        UserNotInVoice(crate::bot::error::UserNotInVoice),
        UserNotStageManager(crate::bot::error::UserNotStageManager),
        UserNotAllowed(crate::bot::error::UserNotAllowed),
        InVoiceAlready(crate::bot::error::InVoiceAlready),
        Forbidden(crate::bot::error::ConnectionForbidden),
        Other(ResidualError),
    }

    pub use PartiallyFlattenedError as Pfe;

    impl PartiallyFlattenedError {
        fn from_handle_response(error: HandleResponseError) -> Self {
            match error {
                HandleResponseError::Cache(e) => {
                    Self::Other(ResidualError::HandleResponse(HandleResponseError::Cache(e)))
                }
                HandleResponseError::Respond(e) => Self::Other(ResidualError::HandleResponse(
                    HandleResponseError::Respond(e),
                )),
                HandleResponseError::DeserializeBody(e) => Self::Other(
                    ResidualError::HandleResponse(HandleResponseError::DeserializeBody(e)),
                ),
                HandleResponseError::Followup(e) => Self::Other(ResidualError::HandleResponse(
                    HandleResponseError::Followup(e),
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
                ImplConnectToError::Lavalink(e) => Self::Other(ResidualError::ImplJoin(
                    ResidualImplJoinError::ConnectTo(ResidualConnectToError::ImplConnectTo(
                        ResidualImplConnectToError::Lavalink(e),
                    )),
                )),
                ImplConnectToError::CheckUserAllowed(e) => Self::from_check_user_allowed(e),
            }
        }

        fn from_connect_to(error: ConnectToError) -> Self {
            match error {
                ConnectToError::UserNotStageManager(e) => Self::UserNotStageManager(e),
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
}

pub mod leave {
    #[derive(thiserror::Error, Debug)]
    #[error(transparent)]
    pub enum PreDisconnectCleanupError {
        EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::bot::lavalink::Event>),
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
    }

    #[derive(thiserror::Error, Debug)]
    #[error("leaving voice failed: {}", .0)]
    pub enum Error {
        NotInVoice(#[from] crate::bot::error::NotInVoice),
        InVoiceWithoutUser(#[from] crate::bot::error::InVoiceWithoutUser),
        CheckUserOnlyIn(#[from] crate::bot::error::command::check::UserOnlyInError),
        PreDisconnectCleanup(#[from] PreDisconnectCleanupError),
        GatewaySend(#[from] twilight_gateway::error::ChannelError),
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
                Self::PreDisconnectCleanup(e) => {
                    NotInVoiceMatchedError::Other(ResidualError::PreDisconnectCleanupError(e))
                }
                Self::GatewaySend(e) => {
                    NotInVoiceMatchedError::Other(ResidualError::GatewaySend(e))
                }
            }
        }
    }

    pub enum NotInVoiceMatchedError {
        NotInVoice(crate::bot::error::NotInVoice),
        Other(ResidualError),
    }

    #[derive(thiserror::Error, Debug)]
    #[error("leaving voice failed: {}", .0)]
    pub enum ResidualError {
        InVoiceWithoutUser(#[from] crate::bot::error::InVoiceWithoutUser),
        CheckUserOnlyIn(#[from] crate::bot::error::command::check::UserOnlyInError),
        PreDisconnectCleanupError(#[from] PreDisconnectCleanupError),
        GatewaySend(#[from] twilight_gateway::error::ChannelError),
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
#[error("starting inactivity timeout failed: {:?}", .0)]
pub enum StartInactivityTimeoutError {
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::bot::lavalink::Event>),
    GatewaySend(#[from] twilight_gateway::error::ChannelError),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    Http(#[from] twilight_http::Error),
    PreDisconnectCleanup(#[from] leave::PreDisconnectCleanupError),
}

#[derive(Error, Debug)]
#[error("handling `VoiceStateUpdate` failed: {:?}", .0)]
pub enum HandleVoiceStateUpdateError {
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::bot::lavalink::Event>),
    Http(#[from] twilight_http::Error),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    MatchStateChannelID(#[from] MatchStateChannelIdError),
    PreDisconnectCleanup(#[from] leave::PreDisconnectCleanupError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum MatchStateChannelIdError {
    Http(#[from] twilight_http::Error),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
    Cache(#[from] crate::bot::error::Cache),
}
