pub mod check;
pub mod declare;
pub mod poll;
pub mod require;
pub mod util;

#[derive(thiserror::Error, Debug)]
#[error("creating a response failed: {}", .0)]
pub enum RespondError {
    TwilightHttp(#[from] twilight_http::Error),
    DeserializeBodyFromHttp(#[from] super::core::DeserializeBodyFromHttpError),
}

#[derive(thiserror::Error, Debug)]
#[error("creating a followup failed: {}", .0)]
pub enum FollowupError {
    DeserializeBodyFromHttp(#[from] super::core::DeserializeBodyFromHttpError),
    Followup(#[from] super::core::FollowupError),
}

#[derive(thiserror::Error, Debug)]
#[error("command failed: {}", .0)]
pub enum Error {
    UserNotAccessManager(#[from] super::UserNotAccessManager),
    Sqlx(#[from] sqlx::Error),
    TaskJoin(#[from] tokio::task::JoinError),
    EmbedValidation(#[from] twilight_validate::embed::EmbedValidationError),
    NotInVoice(#[from] super::NotInVoice),
    InVoiceWithoutUser(#[from] super::InVoiceWithoutUser),
    QueueEmpty(#[from] super::QueueEmpty),
    RequireUnsuppressed(#[from] require::UnsuppressedError),
    CheckUsersTrack(#[from] check::UsersTrackError),
    UserNotDj(#[from] super::UserNotDj),
    RequireInVoiceWithSomeoneElse(#[from] require::InVoiceWithSomeoneElseError),
    PositionOutOfRange(#[from] super::PositionOutOfRange),
    CheckRun(#[from] check::RunError),
    Respond(#[from] RespondError),
    Followup(#[from] FollowupError),
    PromptForConfirmation(#[from] util::PromptForConfirmationError),
    Join(#[from] super::component::connection::join::ResidualError),
    Leave(#[from] super::component::connection::leave::ResidualError),
    Play(#[from] super::component::queue::play::Error),
    DeserializeBodyFromHttp(#[from] super::core::DeserializeBodyFromHttpError),
    RemoveTracks(#[from] super::component::queue::RemoveTracksError),
    TwilightHttp(#[from] twilight_http::Error),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    NoPlayer(#[from] super::lavalink::NoPlayerError),
    NotInGuild(#[from] super::NotInGuild),
    CheckUserOnlyIn(#[from] check::UserOnlyInError),
    Cache(#[from] super::Cache),
    HandlePoll(#[from] check::HandlePollError),
    NotPlaying(#[from] super::NotPlaying),
    Paused(#[from] super::Paused),
}

pub enum FlattenedError<'a> {
    InVoiceWithoutUser(&'a super::InVoiceWithoutUser),
    Suppressed(&'a super::Suppressed),
    NotUsersTrack(&'a super::NotUsersTrack),
    InVoiceWithoutSomeoneElse(&'a super::InVoiceWithoutSomeoneElse),
    InVoiceWithSomeoneElse(&'a super::InVoiceWithSomeoneElse),
    QueueNotSeekable(&'a super::QueueNotSeekable),
    AnotherPollOngoing(&'a check::AnotherPollOngoingError),
    PositionOutOfRange(&'a super::PositionOutOfRange),
    PollLoss(&'a check::PollLossError),
    PollVoided(&'a check::PollVoidedError),
    AutoJoinSuppressed(&'a util::AutoJoinSuppressedError),
    AutoJoinAttemptFailed(&'a super::AutoJoinAttemptFailed),
    UserNotAccessManager,
    Sqlx,
    TaskJoin,
    EmbedValidation,
    NotInVoice,
    QueueEmpty,
    Cache,
    UserNotDj,
    NotPlaying,
    Paused,
    Stopped,
    TwilightHttp,
    DeserializeBody,
    EventSend,
    EventRecv,
    ImageSourceUrl,
    MessageValidation,
    StandbyCanceled,
    ConfirmationTimedOut,
    GatewaySend,
    Lavalink,
    NoPlayer,
    NotInGuild,
}

pub use FlattenedError as Fe;

impl<'a> Fe<'a> {
    const fn from_core_followup_error(error: &'a super::core::FollowupError) -> Self {
        match error {
            super::core::FollowupError::TwilightHttp(_) => Self::TwilightHttp,
            super::core::FollowupError::MessageValidation(_) => Self::MessageValidation,
        }
    }

    const fn from_require_unsuppressed_error(error: &'a require::UnsuppressedError) -> Self {
        match error {
            require::UnsuppressedError::Cache(_) => Self::Cache,
            require::UnsuppressedError::Suppressed(e) => Self::Suppressed(e),
        }
    }

    const fn from_deserialize_body_from_http_error(
        error: &'a super::core::DeserializeBodyFromHttpError,
    ) -> Self {
        match error {
            super::core::DeserializeBodyFromHttpError::TwilightHttp(_) => Self::TwilightHttp,
            super::core::DeserializeBodyFromHttpError::DeserializeBody(_) => Self::DeserializeBody,
        }
    }

    const fn from_users_track_error(error: &'a check::UsersTrackError) -> Self {
        match error {
            check::UsersTrackError::Cache(_) => Self::Cache,
            check::UsersTrackError::NotUsersTrack(e) => Self::NotUsersTrack(e),
        }
    }

    const fn from_require_in_voice_with_someone_else_error(
        error: &'a require::InVoiceWithSomeoneElseError,
    ) -> Self {
        match error {
            require::InVoiceWithSomeoneElseError::Cache(_) => Self::Cache,
            require::InVoiceWithSomeoneElseError::InVoiceWithoutSomeoneElse(e) => {
                Self::InVoiceWithoutSomeoneElse(e)
            }
        }
    }

    const fn from_run(error: &'a check::RunError) -> Self {
        match error {
            check::RunError::NotInVoice(_) => Self::NotInVoice,
            check::RunError::QueueEmpty(_) => Self::QueueEmpty,
            check::RunError::NotPlaying(_) => Self::NotPlaying,
            check::RunError::Cache(_) => Self::Cache,
            check::RunError::Paused(_) => Self::Paused,
            check::RunError::Stopped(_) => Self::Stopped,
            check::RunError::InVoiceWithoutUser(e) => Self::InVoiceWithoutUser(e),
            check::RunError::NotSuppressed(e) => Self::from_require_unsuppressed_error(e),
            check::RunError::HandleInVoiceWithSomeoneElse(e) => {
                Self::from_handle_in_voice_with_someone_else_error(e)
            }
        }
    }

    const fn from_vote_resolvable(error: &'a check::PollResolvableError) -> Self {
        match error {
            check::PollResolvableError::InVoiceWithSomeoneElse(e) => {
                Self::InVoiceWithSomeoneElse(e)
            }
            check::PollResolvableError::QueueNotSeekable(e) => Self::QueueNotSeekable(e),
            check::PollResolvableError::NotUsersTrack(e) => Self::NotUsersTrack(e),
        }
    }

    const fn from_update_embed(error: &'a poll::UpdateEmbedError) -> Self {
        match error {
            poll::UpdateEmbedError::Http(_) => Self::TwilightHttp,
            poll::UpdateEmbedError::EmbedValidation(_) => Self::EmbedValidation,
            poll::UpdateEmbedError::MessageValidation(_) => Self::MessageValidation,
            poll::UpdateEmbedError::Followup(e) => Self::from_core_followup_error(e),
        }
    }

    const fn from_generate_embed(error: &'a poll::GenerateEmbedError) -> Self {
        match error {
            poll::GenerateEmbedError::ImageSourceUrl(_) => Self::ImageSourceUrl,
            poll::GenerateEmbedError::EmbedValidation(_) => Self::EmbedValidation,
        }
    }

    const fn from_wait_for_votes(error: &'a poll::WaitForVotesError) -> Self {
        match error {
            poll::WaitForVotesError::TwilightHttp(_) => Self::TwilightHttp,
            poll::WaitForVotesError::EventRecv(_) => Self::EventRecv,
            poll::WaitForVotesError::DeserializeBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
            poll::WaitForVotesError::UpdateEmbed(e) => Self::from_update_embed(e),
        }
    }

    const fn from_start_poll(error: &'a poll::StartPollError) -> Self {
        match error {
            poll::StartPollError::Cache(_) => Self::Cache,
            poll::StartPollError::DeserializeBody(_) => Self::DeserializeBody,
            poll::StartPollError::Respond(e) => Self::from_respond(e),
            poll::StartPollError::GenerateEmbed(e) => Self::from_generate_embed(e),
            poll::StartPollError::WaitForVotes(e) => Self::from_wait_for_votes(e),
        }
    }

    const fn from_handle_poll(error: &'a check::HandlePollError) -> Self {
        match error {
            check::HandlePollError::EventRecv(_) => Self::EventRecv,
            check::HandlePollError::EventSend(_) => Self::EventSend,
            check::HandlePollError::AnotherPollOngoing(e) => Self::AnotherPollOngoing(e),
            check::HandlePollError::PollLoss(e) => Self::PollLoss(e),
            check::HandlePollError::PollVoided(e) => Self::PollVoided(e),
            check::HandlePollError::StartPoll(e) => Self::from_start_poll(e),
            check::HandlePollError::DeserializeBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
        }
    }

    const fn from_handle_in_voice_with_someone_else_error(
        error: &'a check::HandleInVoiceWithSomeoneElseError,
    ) -> Self {
        match error {
            check::HandleInVoiceWithSomeoneElseError::PollResolvable(e) => {
                Self::from_vote_resolvable(e)
            }
            check::HandleInVoiceWithSomeoneElseError::HandlePoll(e) => Self::from_handle_poll(e),
        }
    }

    const fn from_respond(error: &'a RespondError) -> Self {
        match error {
            RespondError::TwilightHttp(_) => Self::TwilightHttp,
            RespondError::DeserializeBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
        }
    }

    const fn from_followup(error: &'a FollowupError) -> Self {
        match error {
            FollowupError::DeserializeBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
            FollowupError::Followup(e) => Self::from_core_followup_error(e),
        }
    }

    const fn from_prompt_for_confirmation(error: &'a util::PromptForConfirmationError) -> Self {
        match error {
            util::PromptForConfirmationError::StandbyCanceled(_) => Self::StandbyCanceled,
            util::PromptForConfirmationError::ConfirmationTimedout(_) => Self::ConfirmationTimedOut,
            util::PromptForConfirmationError::Respond(e) => Self::from_respond(e),
        }
    }

    const fn from_access_calculator_build(error: &'a check::AccessCalculatorBuildError) -> Self {
        match error {
            check::AccessCalculatorBuildError::Sqlx(_) => Self::Sqlx,
            check::AccessCalculatorBuildError::TaskJoin(_) => Self::TaskJoin,
        }
    }

    const fn from_check_user_only_in(error: &'a check::UserOnlyInError) -> Self {
        match error {
            check::UserOnlyInError::Cache(_) => Self::Cache,
            check::UserOnlyInError::InVoiceWithSomeoneElse(e) => Self::InVoiceWithSomeoneElse(e),
        }
    }

    const fn from_check_user_allowed_residual(
        error: &'a super::component::connection::join::ResidualUserAllowedError,
    ) -> Self {
        match error {
            super::component::connection::join::ResidualUserAllowedError::AccessCalculatorBuild(
                e,
            ) => Self::from_access_calculator_build(e),
        }
    }

    const fn from_impl_connect_to_residual(
        error: &'a super::component::connection::join::ResidualImplConnectToError,
    ) -> Self {
        match error {
            super::component::connection::join::ResidualImplConnectToError::Cache(_) => Self::Cache,
            super::component::connection::join::ResidualImplConnectToError::GatewaySend(_) => {
                Self::GatewaySend
            }
            super::component::connection::join::ResidualImplConnectToError::TwilightHttp(_) => {
                Self::TwilightHttp
            }
            super::component::connection::join::ResidualImplConnectToError::Lavalink(_) => {
                Self::Lavalink
            }
            super::component::connection::join::ResidualImplConnectToError::CheckUserAllowed(e) => {
                Self::from_check_user_allowed_residual(e)
            }
        }
    }

    const fn from_connect_to_residual(
        error: &'a super::component::connection::join::ResidualConnectToError,
    ) -> Self {
        match error {
            super::component::connection::join::ResidualConnectToError::CheckUserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            super::component::connection::join::ResidualConnectToError::ImplConnectTo(e) => {
                Self::from_impl_connect_to_residual(e)
            }
        }
    }

    const fn from_get_users_voice_channel_residual(
        error: &'a super::component::connection::join::ResidualGetUsersVoiceChannelError,
    ) -> Self {
        match error {
            super::component::connection::join::ResidualGetUsersVoiceChannelError::Cache(_) => {
                Self::Cache
            }
        }
    }

    const fn from_impl_join_residual(
        error: &'a super::component::connection::join::ResidualImplJoinError,
    ) -> Self {
        match error {
            super::component::connection::join::ResidualImplJoinError::GetUsersVoiceChannel(e) => {
                Self::from_get_users_voice_channel_residual(e)
            }
            super::component::connection::join::ResidualImplJoinError::ConnectTo(e) => {
                Self::from_connect_to_residual(e)
            }
        }
    }

    const fn from_handle_response(
        error: &'a super::component::connection::join::HandleResponseError,
    ) -> Self {
        match error {
            super::component::connection::join::HandleResponseError::Cache(_) => Self::Cache,
            super::component::connection::join::HandleResponseError::DeserializeBody(_) => {
                Self::DeserializeBody
            }
            super::component::connection::join::HandleResponseError::Respond(e) => {
                Self::from_respond(e)
            }
            super::component::connection::join::HandleResponseError::Followup(e) => {
                Self::from_followup(e)
            }
        }
    }

    const fn from_join_residual(
        error: &'a super::component::connection::join::ResidualError,
    ) -> Self {
        match error {
            super::component::connection::join::ResidualError::ImplJoin(e) => {
                Self::from_impl_join_residual(e)
            }
            super::component::connection::join::ResidualError::HandleResponse(e) => {
                Self::from_handle_response(e)
            }
        }
    }

    const fn from_pre_disconnect_cleanup(
        error: &'a super::component::connection::leave::PreDisconnectCleanupError,
    ) -> Self {
        match error {
            super::component::connection::leave::PreDisconnectCleanupError::EventSend(_) => {
                Self::EventSend
            }
            super::component::connection::leave::PreDisconnectCleanupError::Lavalink(_) => {
                Self::Lavalink
            }
        }
    }

    const fn from_leave_residual(
        error: &'a super::component::connection::leave::ResidualError,
    ) -> Self {
        match error {
            super::component::connection::leave::ResidualError::GatewaySend(_) => Self::GatewaySend,
            super::component::connection::leave::ResidualError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            super::component::connection::leave::ResidualError::CheckUserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            super::component::connection::leave::ResidualError::PreDisconnectCleanupError(e) => {
                Self::from_pre_disconnect_cleanup(e)
            }
        }
    }

    const fn from_handle_suppressed_auto_join(
        error: &'a util::HandleSuppressedAutoJoinError,
    ) -> Self {
        match error {
            util::HandleSuppressedAutoJoinError::DeserializeBody(_) => Self::DeserializeBody,
            util::HandleSuppressedAutoJoinError::FollowUp(e) => Self::from_followup(e),
            util::HandleSuppressedAutoJoinError::AutoJoinSuppressed(e) => {
                Self::AutoJoinSuppressed(e)
            }
        }
    }

    const fn from_get_users_voice_channel_residual_2(
        error: &'a util::ResidualGetUsersVoiceChannelError,
    ) -> Self {
        match error {
            util::ResidualGetUsersVoiceChannelError::Cache(_) => Self::Cache,
        }
    }

    const fn from_check_user_allowed_residual_2(error: &'a util::ResidualUserAllowedError) -> Self {
        match error {
            util::ResidualUserAllowedError::AccessCalculatorBuild(e) => {
                Self::from_access_calculator_build(e)
            }
        }
    }

    const fn from_impl_connect_to_residual_2(error: &'a util::ResidualImplConnectToError) -> Self {
        match error {
            util::ResidualImplConnectToError::Lavalink(_) => Self::Lavalink,
            util::ResidualImplConnectToError::Cache(_) => Self::Cache,
            util::ResidualImplConnectToError::GatewaySend(_) => Self::GatewaySend,
            util::ResidualImplConnectToError::TwilightHttp(_) => Self::TwilightHttp,
            util::ResidualImplConnectToError::CheckUserAllowed(e) => {
                Self::from_check_user_allowed_residual_2(e)
            }
        }
    }

    const fn from_connect_to_new_residual(error: &'a util::ResidualConnectToNewError) -> Self {
        match error {
            util::ResidualConnectToNewError::ImplConnectTo(e) => {
                Self::from_impl_connect_to_residual_2(e)
            }
        }
    }

    const fn from_impl_auto_join_residual(error: &'a util::ResidualImplAutoJoinError) -> Self {
        match error {
            util::ResidualImplAutoJoinError::GetUsersVoiceChannel(e) => {
                Self::from_get_users_voice_channel_residual_2(e)
            }
            util::ResidualImplAutoJoinError::ConnectToNew(e) => {
                Self::from_connect_to_new_residual(e)
            }
        }
    }

    const fn from_auto_join_attempt(error: &'a util::AutoJoinAttemptError) -> Self {
        match error {
            util::AutoJoinAttemptError::Failed(e) => Self::AutoJoinAttemptFailed(e),
            util::AutoJoinAttemptError::ImplAutoJoin(e) => Self::from_impl_auto_join_residual(e),
            util::AutoJoinAttemptError::HandleResponse(e) => Self::from_handle_response(e),
        }
    }

    const fn from_auto_join_or_check_in_voice_with_user(
        error: &'a util::AutoJoinOrCheckInVoiceWithUserError,
    ) -> Self {
        match error {
            util::AutoJoinOrCheckInVoiceWithUserError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            util::AutoJoinOrCheckInVoiceWithUserError::RequireUnsuppressed(e) => {
                Self::from_require_unsuppressed_error(e)
            }
            util::AutoJoinOrCheckInVoiceWithUserError::HandleSuppressedAutoJoin(e) => {
                Self::from_handle_suppressed_auto_join(e)
            }
            util::AutoJoinOrCheckInVoiceWithUserError::AutoJoinAttempt(e) => {
                Self::from_auto_join_attempt(e)
            }
        }
    }

    const fn from_play(error: &'a super::component::queue::play::Error) -> Self {
        match error {
            super::component::queue::play::Error::Lavalink(_) => Self::Lavalink,
            super::component::queue::play::Error::RequireUnsuppressed(e) => {
                Self::from_require_unsuppressed_error(e)
            }
            super::component::queue::play::Error::Respond(e) => Self::from_respond(e),
            super::component::queue::play::Error::Followup(e) => Self::from_followup(e),
            super::component::queue::play::Error::AutoJoinOrCheckInVoiceWithUser(e) => {
                Self::from_auto_join_or_check_in_voice_with_user(e)
            }
        }
    }

    const fn from_remove_tracks(error: &'a super::component::queue::RemoveTracksError) -> Self {
        match error {
            super::component::queue::RemoveTracksError::Lavalink(_) => Self::Lavalink,
            super::component::queue::RemoveTracksError::Respond(e) => Self::from_respond(e),
            super::component::queue::RemoveTracksError::Followup(e) => Self::from_followup(e),
            super::component::queue::RemoveTracksError::DeserializeBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
        }
    }
}

impl Error {
    pub const fn flatten_as(&self) -> Fe<'_> {
        match self {
            Self::UserNotAccessManager(_) => Fe::UserNotAccessManager,
            Self::Sqlx(_) => Fe::Sqlx,
            Self::TaskJoin(_) => Fe::TaskJoin,
            Self::EmbedValidation(_) => Fe::EmbedValidation,
            Self::NotInVoice(_) => Fe::NotInVoice,
            Self::QueueEmpty(_) => Fe::QueueEmpty,
            Self::UserNotDj(_) => Fe::UserNotDj,
            Self::TwilightHttp(_) => Fe::TwilightHttp,
            Self::Lavalink(_) => Fe::Lavalink,
            Self::NoPlayer(_) => Fe::NoPlayer,
            Self::NotInGuild(_) => Fe::NotInGuild,
            Self::Cache(_) => Fe::Cache,
            Self::NotPlaying(_) => Fe::NotPlaying,
            Self::Paused(_) => Fe::Paused,
            Self::RequireUnsuppressed(e) => Fe::from_require_unsuppressed_error(e),
            Self::PositionOutOfRange(e) => Fe::PositionOutOfRange(e),
            Self::InVoiceWithoutUser(e) => Fe::InVoiceWithoutUser(e),
            Self::CheckUsersTrack(e) => Fe::from_users_track_error(e),
            Self::RequireInVoiceWithSomeoneElse(e) => {
                Fe::from_require_in_voice_with_someone_else_error(e)
            }
            Self::CheckRun(e) => Fe::from_run(e),
            Self::Respond(e) => Fe::from_respond(e),
            Self::Followup(e) => Fe::from_followup(e),
            Self::PromptForConfirmation(e) => Fe::from_prompt_for_confirmation(e),
            Self::Join(e) => Fe::from_join_residual(e),
            Self::Leave(e) => Fe::from_leave_residual(e),
            Self::Play(e) => Fe::from_play(e),
            Self::DeserializeBodyFromHttp(e) => Fe::from_deserialize_body_from_http_error(e),
            Self::RemoveTracks(e) => Fe::from_remove_tracks(e),
            Self::CheckUserOnlyIn(e) => Fe::from_check_user_only_in(e),
            Self::HandlePoll(e) => Fe::from_handle_poll(e),
        }
    }
}

pub type Result = core::result::Result<(), Error>;

#[derive(thiserror::Error, Debug)]
#[error("autocomplete failed: {}", .0)]
pub enum AutocompleteError {
    LoadFailed(#[from] super::LoadFailed),
    Respond(#[from] RespondError),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    NotInGuild(#[from] super::NotInGuild),
}

pub type AutocompleteResult = core::result::Result<(), AutocompleteError>;
