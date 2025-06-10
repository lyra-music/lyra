pub mod check;
pub mod declare;
pub mod poll;
pub mod require;
pub mod util;

#[derive(thiserror::Error, Debug)]
#[error("creating a followup failed: {}", .0)]
pub enum FollowupError {
    DeserialiseBodyFromHttp(#[from] super::core::DeserialiseBodyFromHttpError),
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
    Leave(#[from] super::component::connection::leave::ResidualError),
    NoPlayer(#[from] super::lavalink::NoPlayerError),
    NotInGuild(#[from] super::NotInGuild),
    CheckUserOnlyIn(#[from] check::UserOnlyInError),
    Cache(#[from] super::Cache),
    NotPlaying(#[from] super::NotPlaying),
    Paused(#[from] super::Paused),
    UnrecognisedConnection(#[from] super::UnrecognisedConnection),
    NewNowPlayingData(#[from] super::lavalink::NewNowPlayingDataError),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),

    Skip(Box<super::component::playback::skip::SkipError>),
    Back(Box<super::component::playback::back::BackError>),
    Shuffle(Box<super::component::queue::shuffle::ShuffleError>),
    Play(Box<super::component::queue::play::Error>),
    RemoveTracks(Box<super::component::queue::RemoveTracksError>),
    PromptForConfirmation(Box<util::PromptForConfirmationError>),
    TwilightHttp(Box<twilight_http::Error>),
    Join(Box<super::component::connection::join::ResidualError>),
    PlayPause(Box<super::component::playback::PlayPauseError>),
    Repeat(Box<super::component::queue::repeat::RepeatError>),
    UpdateNowPlayingMessage(Box<super::lavalink::UpdateNowPlayingMessageError>),
    SeekToWith(Box<require::SeekToWithError>),
    NewNowPlayingMessage(Box<super::lavalink::NewNowPlayingMessageError>),
    Respond(Box<super::core::RespondError>),
}

macro_rules! declare_from_box_impls {
    ($($variant: ident => $error: path),+$(,)?) => {
        $(
            impl ::std::convert::From<$error> for Error {
                fn from(value: $error) -> Self {
                    Self::$variant(Box::new(value))
                }
            }
        )+
    }
}

declare_from_box_impls!(
    Play => super::component::queue::play::Error,
    RemoveTracks => super::component::queue::RemoveTracksError,
    PromptForConfirmation => util::PromptForConfirmationError,
    TwilightHttp => twilight_http::Error,
    Join => super::component::connection::join::ResidualError,
    PlayPause => super::component::playback::PlayPauseError,
    Repeat => super::component::queue::repeat::RepeatError,
    Skip => super::component::playback::skip::SkipError,
    Back => super::component::playback::back::BackError,
    Shuffle => super::component::queue::shuffle::ShuffleError,
    UpdateNowPlayingMessage => super::lavalink::UpdateNowPlayingMessageError,
    SeekToWith => require::SeekToWithError,
    NewNowPlayingMessage => super::lavalink::NewNowPlayingMessageError,
    Respond => super::core::RespondError,
);

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
    Lavalink(&'a lavalink_rs::error::LavalinkError),
    NoPlayer,
    NotInGuild,
    UnrecognisedConnection,
    TimestampParse,
    GetDominantPaletteFromUrl,
    Builder,
}

pub use FlattenedError as Fe;

impl<'a> Fe<'a> {
    const fn from_require_unsuppressed_error(error: &'a require::UnsuppressedError) -> Self {
        match error {
            require::UnsuppressedError::Cache(_) => Self::Cache,
            require::UnsuppressedError::Suppressed(e) => Self::Suppressed(e),
        }
    }

    const fn from_deserialize_body_from_http_error(
        error: &'a super::core::DeserialiseBodyFromHttpError,
    ) -> Self {
        match error {
            super::core::DeserialiseBodyFromHttpError::TwilightHttp(_) => Self::TwilightHttp,
            super::core::DeserialiseBodyFromHttpError::DeserializeBody(_) => Self::DeserializeBody,
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

    const fn from_respond(error: &'a super::core::RespondError) -> Self {
        match error {
            super::core::RespondError::TwilightHttp(_) => Self::TwilightHttp,
            super::core::RespondError::Builder(_) => Self::Builder,
        }
    }

    const fn from_respond_or_followup(error: &'a super::core::RespondOrFollowupError) -> Self {
        match error {
            super::core::RespondOrFollowupError::Respond(e) => Self::from_respond(e),
            super::core::RespondOrFollowupError::Followup(_) => Self::TwilightHttp,
        }
    }

    const fn from_prompt_for_confirmation(error: &'a util::PromptForConfirmationError) -> Self {
        match error {
            util::PromptForConfirmationError::StandbyCanceled(_) => Self::StandbyCanceled,
            util::PromptForConfirmationError::ConfirmationTimedout(_) => Self::ConfirmationTimedOut,
            util::PromptForConfirmationError::TwilightHttp(_) => Self::TwilightHttp,
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
            super::component::connection::join::ResidualImplConnectToError::Lavalink(e) => {
                Self::Lavalink(e)
            }
            super::component::connection::join::ResidualImplConnectToError::UnrecognisedConnection(_) => {
                Self::UnrecognisedConnection
            },
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
            super::component::connection::join::HandleResponseError::RespondOrFollowup(e) => {
                Self::from_respond_or_followup(e)
            }
            super::component::connection::join::HandleResponseError::Respond(e) => {
                Self::from_respond(e)
            }
            super::component::connection::join::HandleResponseError::DeserialiseBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
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
        error: &'a super::component::connection::leave::DisconnectCleanupError,
    ) -> Self {
        match error {
            super::component::connection::leave::DisconnectCleanupError::EventSend(_) => {
                Self::EventSend
            }
            super::component::connection::leave::DisconnectCleanupError::Lavalink(e) => {
                Self::Lavalink(e)
            }
        }
    }

    const fn from_leave_residual(
        error: &'a super::component::connection::leave::ResidualError,
    ) -> Self {
        match error {
            super::component::connection::leave::ResidualError::GatewaySend(_) => Self::GatewaySend,
            super::component::connection::leave::ResidualError::UnrecognisedConnection(_) => {
                Self::UnrecognisedConnection
            }
            super::component::connection::leave::ResidualError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            super::component::connection::leave::ResidualError::CheckUserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            super::component::connection::leave::ResidualError::DisconnectCleanupError(e) => {
                Self::from_pre_disconnect_cleanup(e)
            }
        }
    }

    const fn from_handle_suppressed_auto_join(
        error: &'a util::HandleSuppressedAutoJoinError,
    ) -> Self {
        match error {
            util::HandleSuppressedAutoJoinError::DeserializeBody(_) => Self::DeserializeBody,
            util::HandleSuppressedAutoJoinError::AutoJoinSuppressed(e) => {
                Self::AutoJoinSuppressed(e)
            }
            util::HandleSuppressedAutoJoinError::Respond(e) => Self::from_respond(e),
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
            util::ResidualImplConnectToError::Lavalink(e) => Self::Lavalink(e),
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
            super::component::queue::play::Error::TwilightHttp(_) => Self::TwilightHttp,
            super::component::queue::play::Error::Lavalink(e) => Self::Lavalink(e),
            super::component::queue::play::Error::RespondOrFollowup(e) => {
                Self::from_respond_or_followup(e)
            }
            super::component::queue::play::Error::HandleLoadTrackResults(e) => {
                Self::from_handle_load_track_results(e)
            }
        }
    }

    const fn from_handle_load_track_results(
        error: &'a super::component::queue::play::HandleLoadTrackResultsError,
    ) -> Self {
        match error {
            super::component::queue::play::HandleLoadTrackResultsError::Lavalink(e) => Self::Lavalink(e),
            super::component::queue::play::HandleLoadTrackResultsError::RespondOrFollowup(e) => Self::from_respond_or_followup(e),
            super::component::queue::play::HandleLoadTrackResultsError::RequireUnsuppressed(e) => Self::from_require_unsuppressed_error(e),
            super::component::queue::play::HandleLoadTrackResultsError::AutoJoinOrCheckInVoiceWithUser(e) => Self::from_auto_join_or_check_in_voice_with_user(e),
            super::component::queue::play::HandleLoadTrackResultsError::UpdateNowPlayingMessage(e) => Self::from_update_now_playing_message(e),
        }
    }

    const fn from_remove_tracks(error: &'a super::component::queue::RemoveTracksError) -> Self {
        match error {
            super::component::queue::RemoveTracksError::Lavalink(e) => Self::Lavalink(e),
            super::component::queue::RemoveTracksError::DeserialiseBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
            super::component::queue::RemoveTracksError::UpdateNowPlayingMessage(e) => {
                Self::from_update_now_playing_message(e)
            }
            super::component::queue::RemoveTracksError::Respond(e) => Self::from_respond(e),
            super::component::queue::RemoveTracksError::RespondOrFollowup(e) => {
                Self::from_respond_or_followup(e)
            }
        }
    }

    const fn from_play_pause(error: &'a super::component::playback::PlayPauseError) -> Self {
        match error {
            super::component::playback::PlayPauseError::Lavalink(e) => Self::Lavalink(e),
            super::component::playback::PlayPauseError::NotInVoice(_) => Self::NotInVoice,
            super::component::playback::PlayPauseError::NotPlaying(_) => Self::NotPlaying,
            super::component::playback::PlayPauseError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            super::component::playback::PlayPauseError::Respond(e) => Self::from_respond(e),
            super::component::playback::PlayPauseError::SetPauseWith(e) => {
                Self::from_set_pause_with(e)
            }
            super::component::playback::PlayPauseError::Unsuppressed(e) => {
                Self::from_require_unsuppressed_error(e)
            }
            super::component::playback::PlayPauseError::UserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            super::component::playback::PlayPauseError::UsersTrack(e) => {
                Self::from_users_track_error(e)
            }
        }
    }

    const fn from_back(error: &'a super::component::playback::back::BackError) -> Self {
        match error {
            super::component::playback::back::BackError::NotInVoice(_) => Self::NotInVoice,
            super::component::playback::back::BackError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            super::component::playback::back::BackError::Lavalink(e) => Self::Lavalink(e),
            super::component::playback::back::BackError::UserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            super::component::playback::back::BackError::Unsuppressed(e) => {
                Self::from_require_unsuppressed_error(e)
            }
            super::component::playback::back::BackError::Respond(e) => Self::from_respond(e),
        }
    }

    const fn from_skip(error: &'a super::component::playback::skip::SkipError) -> Self {
        match error {
            super::component::playback::skip::SkipError::NotPlaying(_) => Self::NotPlaying,
            super::component::playback::skip::SkipError::NotInVoice(_) => Self::NotInVoice,
            super::component::playback::skip::SkipError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            super::component::playback::skip::SkipError::Lavalink(e) => Self::Lavalink(e),
            super::component::playback::skip::SkipError::Unsuppressed(e) => {
                Self::from_require_unsuppressed_error(e)
            }
            super::component::playback::skip::SkipError::UsersTrackError(e) => {
                Self::from_users_track_error(e)
            }
            super::component::playback::skip::SkipError::Respond(e) => Self::from_respond(e),
        }
    }

    const fn from_shuffle(error: &'a super::component::queue::shuffle::ShuffleError) -> Self {
        match error {
            super::component::queue::shuffle::ShuffleError::NotInVoice(_) => Self::NotInVoice,
            super::component::queue::shuffle::ShuffleError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            super::component::queue::shuffle::ShuffleError::UserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            super::component::queue::shuffle::ShuffleError::UpdateNowPlayingMessage(e) => {
                Self::from_update_now_playing_message(e)
            }
            super::component::queue::shuffle::ShuffleError::Respond(e) => Self::from_respond(e),
        }
    }

    const fn from_repeat(error: &'a super::component::queue::repeat::RepeatError) -> Self {
        match error {
            super::component::queue::repeat::RepeatError::UnrecognisedConnection(_) => {
                Self::UnrecognisedConnection
            }
            super::component::queue::repeat::RepeatError::NotInVoice(_) => Self::NotInVoice,
            super::component::queue::repeat::RepeatError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            super::component::queue::repeat::RepeatError::UpdateNowPlayingMessage(e) => {
                Self::from_update_now_playing_message(e)
            }
            super::component::queue::repeat::RepeatError::Respond(e) => Self::from_respond(e),
            super::component::queue::repeat::RepeatError::UserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
        }
    }

    const fn from_update_now_playing_message(
        error: &'a super::lavalink::UpdateNowPlayingMessageError,
    ) -> Self {
        match error {
            super::lavalink::UpdateNowPlayingMessageError::BuildNowPlayingEmbed(e) => {
                Self::from_build_now_playing_embed(e)
            }
            super::lavalink::UpdateNowPlayingMessageError::DeserialiseBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
            super::lavalink::UpdateNowPlayingMessageError::Respond(e) => Self::from_respond(e),
            super::lavalink::UpdateNowPlayingMessageError::TwilightHttp(_) => Self::TwilightHttp,
        }
    }

    const fn from_build_now_playing_embed(
        error: &'a super::lavalink::BuildNowPlayingEmbedError,
    ) -> Self {
        match error {
            super::lavalink::BuildNowPlayingEmbedError::ImageSourceUrl(_) => Self::ImageSourceUrl,
            super::lavalink::BuildNowPlayingEmbedError::TimestampParse(_) => Self::TimestampParse,
        }
    }

    const fn from_seek_to_with(error: &'a require::SeekToWithError) -> Self {
        match error {
            require::SeekToWithError::Lavalink(e) => Self::Lavalink(e),
            require::SeekToWithError::UpdateNowPlayingMessage(e) => {
                Self::from_update_now_playing_message(e)
            }
        }
    }

    #[inline]
    const fn from_set_pause_with(error: &'a require::SetPauseWithError) -> Self {
        Self::from_seek_to_with(error)
    }

    const fn from_new_now_playing_data(error: &'a super::lavalink::NewNowPlayingDataError) -> Self {
        match error {
            super::lavalink::NewNowPlayingDataError::Cache(_) => Self::Cache,
            super::lavalink::NewNowPlayingDataError::GetDominantPaletteFromUrl(_) => {
                Self::GetDominantPaletteFromUrl
            }
        }
    }

    const fn from_new_now_playing_message(
        error: &'a super::lavalink::NewNowPlayingMessageError,
    ) -> Self {
        match error {
            super::lavalink::NewNowPlayingMessageError::TwilightHttp(_) => Self::TwilightHttp,
            super::lavalink::NewNowPlayingMessageError::DeserialiseBody(_) => Self::DeserializeBody,
            super::lavalink::NewNowPlayingMessageError::DeserialiseBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
            super::lavalink::NewNowPlayingMessageError::BuildNowPlayingEmbed(e) => {
                Self::from_build_now_playing_embed(e)
            }
        }
    }
}

impl Error {
    #[must_use]
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
            Self::Lavalink(e) => Fe::Lavalink(e),
            Self::NoPlayer(_) => Fe::NoPlayer,
            Self::NotInGuild(_) => Fe::NotInGuild,
            Self::Cache(_) => Fe::Cache,
            Self::NotPlaying(_) => Fe::NotPlaying,
            Self::Paused(_) => Fe::Paused,
            Self::UnrecognisedConnection(_) => Fe::UnrecognisedConnection,
            Self::PositionOutOfRange(e) => Fe::PositionOutOfRange(e),
            Self::InVoiceWithoutUser(e) => Fe::InVoiceWithoutUser(e),
            Self::RequireUnsuppressed(e) => Fe::from_require_unsuppressed_error(e),
            Self::CheckUsersTrack(e) => Fe::from_users_track_error(e),
            Self::RequireInVoiceWithSomeoneElse(e) => {
                Fe::from_require_in_voice_with_someone_else_error(e)
            }
            Self::PromptForConfirmation(e) => Fe::from_prompt_for_confirmation(e),
            Self::Join(e) => Fe::from_join_residual(e),
            Self::Leave(e) => Fe::from_leave_residual(e),
            Self::Play(e) => Fe::from_play(e),
            Self::RemoveTracks(e) => Fe::from_remove_tracks(e),
            Self::CheckUserOnlyIn(e) => Fe::from_check_user_only_in(e),
            Self::PlayPause(e) => Fe::from_play_pause(e),
            Self::Skip(e) => Fe::from_skip(e),
            Self::Back(e) => Fe::from_back(e),
            Self::Shuffle(e) => Fe::from_shuffle(e),
            Self::Repeat(e) => Fe::from_repeat(e),
            Self::UpdateNowPlayingMessage(e) => Fe::from_update_now_playing_message(e),
            Self::SeekToWith(e) => Fe::from_seek_to_with(e),
            Self::NewNowPlayingData(e) => Fe::from_new_now_playing_data(e),
            Self::NewNowPlayingMessage(e) => Fe::from_new_now_playing_message(e),
            Self::Respond(e) => Fe::from_respond(e),
        }
    }
}

pub type Result = core::result::Result<(), Error>;

#[derive(thiserror::Error, Debug)]
#[error("autocomplete failed: {}", .0)]
pub enum AutocompleteError {
    LoadFailed(#[from] super::LoadFailed),
    TwilightHttp(#[from] twilight_http::Error),
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    NotInGuild(#[from] super::NotInGuild),
}

pub type AutocompleteResult = core::result::Result<(), AutocompleteError>;
