pub mod check;
pub mod declare;
pub mod poll;
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
    CheckNotSuppressed(#[from] check::NotSuppressedError),
    CheckUsersTrack(#[from] check::UsersTrackError),
    UserNotDj(#[from] super::UserNotDj),
    InVoiceWithSomeoneElse(#[from] check::InVoiceWithSomeoneElseError),
    PositionOutOfRange(#[from] super::PositionOutOfRange),
    CheckRun(#[from] check::RunError),
    Respond(#[from] RespondError),
    Followup(#[from] FollowupError),
    PromptForConfirmation(#[from] util::PromptForConfirmationError),
    Join(#[from] super::component::connection::join::ResidualError),
    Leave(#[from] super::component::connection::leave::ResidualError),
    WithAdvanceLockAndStopped(
        #[from] super::component::queue::remove::WithAdvanceLockAndStoppedError,
    ),
    Play(#[from] super::component::queue::play::Error),
    DeserializeBodyFromHttp(#[from] super::core::DeserializeBodyFromHttpError),
    RemoveTracks(#[from] super::component::queue::RemoveTracksError),
}

pub enum FlattenedError<'a> {
    UserNotAccessManager(&'a super::UserNotAccessManager),
    Sqlx(&'a sqlx::Error),
    TaskJoin(&'a tokio::task::JoinError),
    EmbedValidation(&'a twilight_validate::embed::EmbedValidationError),
    NotInVoice(&'a super::NotInVoice),
    InVoiceWithoutUser(&'a super::InVoiceWithoutUser),
    QueueEmpty(&'a super::QueueEmpty),
    PositionOutOfRange(&'a super::PositionOutOfRange),
    Cache(&'a super::Cache),
    Suppressed(&'a super::Suppressed),
    NotUsersTrack(&'a super::NotUsersTrack),
    UserNotDj(&'a super::UserNotDj),
    InVoiceWithoutSomeoneElse(&'a super::InVoiceWithoutSomeoneElse),
    NotPlaying(&'a super::NotPlaying),
    Paused(&'a super::Paused),
    Stopped(&'a super::Stopped),
    InVoiceWithSomeoneElse(&'a super::InVoiceWithSomeoneElse),
    QueueNotSeekable(&'a super::QueueNotSeekable),
    AnotherPollOngoing(&'a check::AnotherPollOngoingError),
    TwilightHttp(&'a twilight_http::Error),
    DeserializeBody(&'a twilight_http::response::DeserializeBodyError),
    EventSend(&'a tokio::sync::broadcast::error::SendError<crate::bot::lavalink::Event>),
    EventRecv(&'a tokio::sync::broadcast::error::RecvError),
    ImageSourceUrl(&'a twilight_util::builder::embed::image_source::ImageSourceUrlError),
    MessageValidation(&'a twilight_validate::message::MessageValidationError),
    PollLoss(&'a check::PollLossError),
    PollVoided(&'a check::PollVoidedError),
    StandbyCanceled(&'a twilight_standby::future::Canceled),
    Confirmation(&'a util::ConfirmationError),
    GatewaySend(&'a twilight_gateway::error::ChannelError),
    AutoJoinSuppressed(&'a util::AutoJoinSuppressedError),
    AutoJoinAttemptFailed(&'a super::AutoJoinAttemptFailed),
    Lavalink(&'a lavalink_rs::error::LavalinkError),
}

pub use FlattenedError as Fe;

impl<'a> Fe<'a> {
    const fn from_core_followup_error(error: &'a super::core::FollowupError) -> Fe<'a> {
        match error {
            super::core::FollowupError::TwilightHttp(e) => Self::TwilightHttp(e),
            super::core::FollowupError::MessageValidation(e) => Self::MessageValidation(e),
        }
    }

    const fn from_check_not_suppressed_error(error: &'a check::NotSuppressedError) -> Fe<'a> {
        match error {
            check::NotSuppressedError::Cache(e) => Self::Cache(e),
            check::NotSuppressedError::Suppressed(e) => Self::Suppressed(e),
        }
    }

    const fn from_deserialize_body_from_http_error(
        error: &'a super::core::DeserializeBodyFromHttpError,
    ) -> Fe<'a> {
        match error {
            super::core::DeserializeBodyFromHttpError::TwilightHttp(e) => Self::TwilightHttp(e),
            super::core::DeserializeBodyFromHttpError::DeserializeBody(e) => {
                Self::DeserializeBody(e)
            }
        }
    }

    const fn from_users_track_error(error: &'a check::UsersTrackError) -> Fe<'a> {
        match error {
            check::UsersTrackError::Cache(e) => Self::Cache(e),
            check::UsersTrackError::NotUsersTrack(e) => Self::NotUsersTrack(e),
        }
    }

    const fn from_in_voice_with_someone_else_error(
        error: &'a check::InVoiceWithSomeoneElseError,
    ) -> Self {
        match error {
            check::InVoiceWithSomeoneElseError::Cache(e) => Self::Cache(e),
            check::InVoiceWithSomeoneElseError::InVoiceWithoutSomeoneElse(e) => {
                Self::InVoiceWithoutSomeoneElse(e)
            }
        }
    }

    const fn from_run(error: &'a check::RunError) -> Fe<'a> {
        match error {
            check::RunError::NotInVoice(e) => Self::NotInVoice(e),
            check::RunError::QueueEmpty(e) => Self::QueueEmpty(e),
            check::RunError::NotPlaying(e) => Self::NotPlaying(e),
            check::RunError::InVoiceWithoutUser(e) => Self::InVoiceWithoutUser(e),
            check::RunError::Cache(e) => Self::Cache(e),
            check::RunError::Paused(e) => Self::Paused(e),
            check::RunError::Stopped(e) => Self::Stopped(e),
            check::RunError::NotSuppressed(e) => Self::from_check_not_suppressed_error(e),
            check::RunError::HandleInVoiceWithSomeoneElse(e) => {
                Self::from_handle_in_voice_with_someone_else_error(e)
            }
        }
    }

    const fn from_vote_resolvable(error: &'a check::PollResolvableError) -> Fe<'a> {
        match error {
            check::PollResolvableError::InVoiceWithSomeoneElse(e) => {
                Self::InVoiceWithSomeoneElse(e)
            }
            check::PollResolvableError::QueueNotSeekable(e) => Self::QueueNotSeekable(e),
            check::PollResolvableError::NotUsersTrack(e) => Self::NotUsersTrack(e),
        }
    }

    const fn from_update_embed(error: &'a poll::UpdateEmbedError) -> Fe<'a> {
        match error {
            poll::UpdateEmbedError::Http(e) => Self::TwilightHttp(e),
            poll::UpdateEmbedError::EmbedValidation(e) => Self::EmbedValidation(e),
            poll::UpdateEmbedError::MessageValidation(e) => Self::MessageValidation(e),
            poll::UpdateEmbedError::Followup(e) => Self::from_core_followup_error(e),
        }
    }

    const fn from_generate_embed(error: &'a poll::GenerateEmbedError) -> Fe<'a> {
        match error {
            poll::GenerateEmbedError::ImageSourceUrl(e) => Self::ImageSourceUrl(e),
            poll::GenerateEmbedError::EmbedValidation(e) => Self::EmbedValidation(e),
        }
    }

    const fn from_wait_for_votes(error: &'a poll::WaitForVotesError) -> Fe<'a> {
        match error {
            poll::WaitForVotesError::TwilightHttp(e) => Self::TwilightHttp(e),
            poll::WaitForVotesError::EventRecv(e) => Self::EventRecv(e),
            poll::WaitForVotesError::DeserializeBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
            poll::WaitForVotesError::UpdateEmbed(e) => Self::from_update_embed(e),
        }
    }

    const fn from_start_poll(error: &'a poll::StartPollError) -> Fe<'a> {
        match error {
            poll::StartPollError::Cache(e) => Self::Cache(e),
            poll::StartPollError::DeserializeBody(e) => Self::DeserializeBody(e),
            poll::StartPollError::Respond(e) => Self::from_respond(e),
            poll::StartPollError::GenerateEmbed(e) => Self::from_generate_embed(e),
            poll::StartPollError::WaitForVotes(e) => Self::from_wait_for_votes(e),
        }
    }

    const fn from_handle_poll(error: &'a check::HandlePollError) -> Fe<'a> {
        match error {
            check::HandlePollError::AnotherPollOngoing(e) => Self::AnotherPollOngoing(e),
            check::HandlePollError::EventSend(e) => Self::EventSend(e),
            check::HandlePollError::PollLoss(e) => Self::PollLoss(e),
            check::HandlePollError::PollVoided(e) => Self::PollVoided(e),
            check::HandlePollError::EventRecv(e) => Self::EventRecv(e),
            check::HandlePollError::StartPoll(e) => Self::from_start_poll(e),
            check::HandlePollError::DeserializeBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
        }
    }

    const fn from_handle_in_voice_with_someone_else_error(
        error: &'a check::HandleInVoiceWithSomeoneElseError,
    ) -> Fe<'a> {
        match error {
            check::HandleInVoiceWithSomeoneElseError::PollResolvable(e) => {
                Self::from_vote_resolvable(e)
            }
            check::HandleInVoiceWithSomeoneElseError::HandlePollError(e) => {
                Self::from_handle_poll(e)
            }
        }
    }

    const fn from_respond(error: &'a RespondError) -> Fe<'a> {
        match error {
            RespondError::TwilightHttp(e) => Self::TwilightHttp(e),
            RespondError::DeserializeBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
        }
    }

    const fn from_followup(error: &'a FollowupError) -> Fe<'a> {
        match error {
            FollowupError::DeserializeBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
            FollowupError::Followup(e) => Self::from_core_followup_error(e),
        }
    }

    const fn from_prompt_for_confirmation(error: &'a util::PromptForConfirmationError) -> Fe<'a> {
        match error {
            util::PromptForConfirmationError::StandbyCanceled(e) => Self::StandbyCanceled(e),
            util::PromptForConfirmationError::Confirmation(e) => Self::Confirmation(e),
            util::PromptForConfirmationError::Respond(e) => Self::from_respond(e),
        }
    }

    const fn from_access_calculator_build(error: &'a check::AccessCalculatorBuildError) -> Fe<'a> {
        match error {
            check::AccessCalculatorBuildError::Sqlx(e) => Self::Sqlx(e),
            check::AccessCalculatorBuildError::TaskJoin(e) => Self::TaskJoin(e),
        }
    }

    const fn from_check_user_only_in(error: &'a check::UserOnlyInError) -> Fe<'a> {
        match error {
            check::UserOnlyInError::Cache(e) => Self::Cache(e),
            check::UserOnlyInError::InVoiceWithSomeoneElse(e) => Self::InVoiceWithSomeoneElse(e),
        }
    }

    const fn from_check_user_allowed_residual(
        error: &'a super::component::connection::join::ResidualUserAllowedError,
    ) -> Fe<'a> {
        match error {
            super::component::connection::join::ResidualUserAllowedError::AccessCalculatorBuild(
                e,
            ) => Self::from_access_calculator_build(e),
        }
    }

    const fn from_impl_connect_to_residual(
        error: &'a super::component::connection::join::ResidualImplConnectToError,
    ) -> Fe<'a> {
        match error {
            super::component::connection::join::ResidualImplConnectToError::Cache(e) => {
                Self::Cache(e)
            }
            super::component::connection::join::ResidualImplConnectToError::GatewaySend(e) => {
                Self::GatewaySend(e)
            }
            super::component::connection::join::ResidualImplConnectToError::TwilightHttp(e) => {
                Self::TwilightHttp(e)
            }
            super::component::connection::join::ResidualImplConnectToError::Lavalink(e) => {
                Self::Lavalink(e)
            }
            super::component::connection::join::ResidualImplConnectToError::CheckUserAllowed(e) => {
                Self::from_check_user_allowed_residual(e)
            }
        }
    }

    const fn from_connect_to_residual(
        error: &'a super::component::connection::join::ResidualConnectToError,
    ) -> Fe<'a> {
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
    ) -> Fe<'a> {
        match error {
            super::component::connection::join::ResidualGetUsersVoiceChannelError::Cache(e) => {
                Self::Cache(e)
            }
        }
    }

    const fn from_impl_join_residual(
        error: &'a super::component::connection::join::ResidualImplJoinError,
    ) -> Fe<'a> {
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
    ) -> Fe<'a> {
        match error {
            super::component::connection::join::HandleResponseError::Cache(e) => Self::Cache(e),
            super::component::connection::join::HandleResponseError::DeserializeBody(e) => {
                Self::DeserializeBody(e)
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
    ) -> Fe<'a> {
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
    ) -> Fe<'a> {
        match error {
            super::component::connection::leave::PreDisconnectCleanupError::EventSend(e) => {
                Self::EventSend(e)
            }
            super::component::connection::leave::PreDisconnectCleanupError::Lavalink(e) => {
                Self::Lavalink(e)
            }
        }
    }

    const fn from_leave_residual(
        error: &'a super::component::connection::leave::ResidualError,
    ) -> Fe<'a> {
        match error {
            super::component::connection::leave::ResidualError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            super::component::connection::leave::ResidualError::GatewaySend(e) => {
                Self::GatewaySend(e)
            }
            super::component::connection::leave::ResidualError::CheckUserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            super::component::connection::leave::ResidualError::PreDisconnectCleanupError(e) => {
                Self::from_pre_disconnect_cleanup(e)
            }
        }
    }

    const fn from_with_advance_lock_and_stopped(
        error: &'a super::component::queue::remove::WithAdvanceLockAndStoppedError,
    ) -> Fe<'a> {
        match error {
            super::component::queue::remove::WithAdvanceLockAndStoppedError::Lavalink(e) => {
                Self::Lavalink(e)
            }
        }
    }

    const fn from_handle_suppressed_auto_join(
        error: &'a util::HandleSuppressedAutoJoinError,
    ) -> Fe<'a> {
        match error {
            util::HandleSuppressedAutoJoinError::DeserializeBody(e) => Self::DeserializeBody(e),
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
            util::ResidualGetUsersVoiceChannelError::Cache(e) => Self::Cache(e),
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
            util::ResidualImplConnectToError::Lavalink(e) => Self::Lavalink(e),
            util::ResidualImplConnectToError::Cache(e) => Self::Cache(e),
            util::ResidualImplConnectToError::GatewaySend(e) => Self::GatewaySend(e),
            util::ResidualImplConnectToError::TwilightHttp(e) => Self::TwilightHttp(e),
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

    const fn from_auto_join_attempt(error: &'a util::AutoJoinAttemptError) -> Fe<'a> {
        match error {
            util::AutoJoinAttemptError::Failed(e) => Self::AutoJoinAttemptFailed(e),
            util::AutoJoinAttemptError::ImplAutoJoin(e) => Self::from_impl_auto_join_residual(e),
            util::AutoJoinAttemptError::HandleResponse(e) => Self::from_handle_response(e),
        }
    }

    const fn from_auto_join_or_check_in_voice_with_user(
        error: &'a util::AutoJoinOrCheckInVoiceWithUserError,
    ) -> Fe<'a> {
        match error {
            util::AutoJoinOrCheckInVoiceWithUserError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            util::AutoJoinOrCheckInVoiceWithUserError::CheckNotSuppressed(e) => {
                Self::from_check_not_suppressed_error(e)
            }
            util::AutoJoinOrCheckInVoiceWithUserError::HandleSuppressedAutoJoin(e) => {
                Self::from_handle_suppressed_auto_join(e)
            }
            util::AutoJoinOrCheckInVoiceWithUserError::AutoJoinAttempt(e) => {
                Self::from_auto_join_attempt(e)
            }
        }
    }

    const fn from_play(error: &'a super::component::queue::play::Error) -> Fe<'a> {
        match error {
            super::component::queue::play::Error::Lavalink(e) => Self::Lavalink(e),
            super::component::queue::play::Error::CheckNotSuppressed(e) => {
                Self::from_check_not_suppressed_error(e)
            }
            super::component::queue::play::Error::Respond(e) => Self::from_respond(e),
            super::component::queue::play::Error::Followup(e) => Self::from_followup(e),
            super::component::queue::play::Error::AutoJoinOrCheckInVoiceWithUser(e) => {
                Self::from_auto_join_or_check_in_voice_with_user(e)
            }
        }
    }

    const fn from_remove_tracks(error: &'a super::component::queue::RemoveTracksError) -> Fe<'a> {
        match error {
            super::component::queue::RemoveTracksError::TryWithAdvanceLock(e) => {
                Self::from_with_advance_lock_and_stopped(e)
            }
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
            Self::UserNotAccessManager(e) => Fe::UserNotAccessManager(e),
            Self::Sqlx(e) => Fe::Sqlx(e),
            Self::TaskJoin(e) => Fe::TaskJoin(e),
            Self::EmbedValidation(e) => Fe::EmbedValidation(e),
            Self::NotInVoice(e) => Fe::NotInVoice(e),
            Self::InVoiceWithoutUser(e) => Fe::InVoiceWithoutUser(e),
            Self::QueueEmpty(e) => Fe::QueueEmpty(e),
            Self::PositionOutOfRange(e) => Fe::PositionOutOfRange(e),
            Self::UserNotDj(e) => Fe::UserNotDj(e),
            Self::CheckNotSuppressed(e) => Fe::from_check_not_suppressed_error(e),
            Self::CheckUsersTrack(e) => Fe::from_users_track_error(e),
            Self::InVoiceWithSomeoneElse(e) => Fe::from_in_voice_with_someone_else_error(e),
            Self::CheckRun(e) => Fe::from_run(e),
            Self::Respond(e) => Fe::from_respond(e),
            Self::Followup(e) => Fe::from_followup(e),
            Self::PromptForConfirmation(e) => Fe::from_prompt_for_confirmation(e),
            Self::Join(e) => Fe::from_join_residual(e),
            Self::Leave(e) => Fe::from_leave_residual(e),
            Self::WithAdvanceLockAndStopped(e) => Fe::from_with_advance_lock_and_stopped(e),
            Self::Play(e) => Fe::from_play(e),
            Self::DeserializeBodyFromHttp(e) => Fe::from_deserialize_body_from_http_error(e),
            Self::RemoveTracks(e) => Fe::from_remove_tracks(e),
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
}

pub type AutocompleteResult = core::result::Result<(), AutocompleteError>;
