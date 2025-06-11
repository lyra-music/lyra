use thiserror::Error;

pub use FlattenedControllerError as Fe;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum ControllerError {
    Shuffle(#[from] crate::error::component::queue::shuffle::ShuffleError),
    Back(#[from] crate::error::component::playback::back::BackError),
    PlayPause(#[from] crate::error::component::playback::PlayPauseError),
    Skip(#[from] crate::error::component::playback::skip::SkipError),
    Repeat(#[from] crate::error::component::queue::repeat::RepeatError),
}

pub enum FlattenedControllerError<'a> {
    NotInVoice,
    NotPlaying,
    UnrecognisedConnection,
    Cache,
    TwilightHttp,
    ImageSourceUrl,
    TimestampParse,
    DeserializeBody,
    Builder,
    InVoiceWithoutUser(&'a crate::error::InVoiceWithoutUser),
    InVoiceWithSomeoneElse(&'a crate::error::InVoiceWithSomeoneElse),
    Lavalink(&'a lavalink_rs::error::LavalinkError),
    Suppressed(&'a crate::error::Suppressed),
    NotUsersTrack(&'a crate::error::NotUsersTrack),
}

impl<'a> Fe<'a> {
    const fn from_shuffle(
        error: &'a crate::error::component::queue::shuffle::ShuffleError,
    ) -> Self {
        match error {
            crate::error::component::queue::shuffle::ShuffleError::NotInVoice(_) => {
                Self::NotInVoice
            }
            crate::error::component::queue::shuffle::ShuffleError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            crate::error::component::queue::shuffle::ShuffleError::UserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            crate::error::component::queue::shuffle::ShuffleError::UpdateNowPlayingMessage(e) => {
                Self::from_update_now_playing_message(e)
            }
            crate::error::component::queue::shuffle::ShuffleError::Respond(e) => {
                Self::from_respond(e)
            }
        }
    }

    const fn from_back(error: &'a crate::error::component::playback::back::BackError) -> Self {
        match error {
            crate::error::component::playback::back::BackError::NotInVoice(_) => Self::NotInVoice,
            crate::error::component::playback::back::BackError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            crate::error::component::playback::back::BackError::Lavalink(e) => Self::Lavalink(e),
            crate::error::component::playback::back::BackError::UserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            crate::error::component::playback::back::BackError::Unsuppressed(e) => {
                Self::from_require_unsuppressed_error(e)
            }
            crate::error::component::playback::back::BackError::Respond(e) => Self::from_respond(e),
        }
    }

    const fn from_play_pause(error: &'a crate::error::component::playback::PlayPauseError) -> Self {
        match error {
            crate::error::component::playback::PlayPauseError::Lavalink(e) => Self::Lavalink(e),
            crate::error::component::playback::PlayPauseError::NotInVoice(_) => Self::NotInVoice,
            crate::error::component::playback::PlayPauseError::NotPlaying(_) => Self::NotPlaying,
            crate::error::component::playback::PlayPauseError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            crate::error::component::playback::PlayPauseError::Respond(e) => Self::from_respond(e),
            crate::error::component::playback::PlayPauseError::SetPauseWith(e) => {
                Self::from_set_pause_with(e)
            }
            crate::error::component::playback::PlayPauseError::Unsuppressed(e) => {
                Self::from_require_unsuppressed_error(e)
            }
            crate::error::component::playback::PlayPauseError::UserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
            crate::error::component::playback::PlayPauseError::UsersTrack(e) => {
                Self::from_users_track_error(e)
            }
        }
    }

    const fn from_skip(error: &'a crate::error::component::playback::skip::SkipError) -> Self {
        match error {
            crate::error::component::playback::skip::SkipError::NotPlaying(_) => Self::NotPlaying,
            crate::error::component::playback::skip::SkipError::NotInVoice(_) => Self::NotInVoice,
            crate::error::component::playback::skip::SkipError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            crate::error::component::playback::skip::SkipError::Lavalink(e) => Self::Lavalink(e),
            crate::error::component::playback::skip::SkipError::Unsuppressed(e) => {
                Self::from_require_unsuppressed_error(e)
            }
            crate::error::component::playback::skip::SkipError::UsersTrackError(e) => {
                Self::from_users_track_error(e)
            }
            crate::error::component::playback::skip::SkipError::Respond(e) => Self::from_respond(e),
        }
    }

    const fn from_repeat(error: &'a crate::error::component::queue::repeat::RepeatError) -> Self {
        match error {
            crate::error::component::queue::repeat::RepeatError::UnrecognisedConnection(_) => {
                Self::UnrecognisedConnection
            }
            crate::error::component::queue::repeat::RepeatError::NotInVoice(_) => Self::NotInVoice,
            crate::error::component::queue::repeat::RepeatError::InVoiceWithoutUser(e) => {
                Self::InVoiceWithoutUser(e)
            }
            crate::error::component::queue::repeat::RepeatError::UpdateNowPlayingMessage(e) => {
                Self::from_update_now_playing_message(e)
            }
            crate::error::component::queue::repeat::RepeatError::Respond(e) => {
                Self::from_respond(e)
            }
            crate::error::component::queue::repeat::RepeatError::UserOnlyIn(e) => {
                Self::from_check_user_only_in(e)
            }
        }
    }

    const fn from_check_user_only_in(
        error: &'a crate::error::command::check::UserOnlyInError,
    ) -> Self {
        match error {
            crate::error::command::check::UserOnlyInError::Cache(_) => Self::Cache,
            crate::error::command::check::UserOnlyInError::InVoiceWithSomeoneElse(e) => {
                Self::InVoiceWithSomeoneElse(e)
            }
        }
    }

    const fn from_update_now_playing_message(
        error: &'a crate::error::lavalink::UpdateNowPlayingMessageError,
    ) -> Self {
        match error {
            crate::error::lavalink::UpdateNowPlayingMessageError::TwilightHttp(_) => {
                Self::TwilightHttp
            }
            crate::error::lavalink::UpdateNowPlayingMessageError::BuildNowPlayingEmbed(e) => {
                Self::from_build_now_playing_embed(e)
            }
            crate::error::lavalink::UpdateNowPlayingMessageError::DeserialiseBodyFromHttp(e) => {
                Self::from_deserialize_body_from_http_error(e)
            }
            crate::error::lavalink::UpdateNowPlayingMessageError::Respond(e) => {
                Self::from_respond(e)
            }
        }
    }

    const fn from_build_now_playing_embed(
        error: &'a crate::error::lavalink::BuildNowPlayingEmbedError,
    ) -> Self {
        match error {
            crate::error::lavalink::BuildNowPlayingEmbedError::ImageSourceUrl(_) => {
                Self::ImageSourceUrl
            }
            crate::error::lavalink::BuildNowPlayingEmbedError::TimestampParse(_) => {
                Self::TimestampParse
            }
        }
    }

    const fn from_deserialize_body_from_http_error(
        error: &'a crate::error::core::DeserialiseBodyFromHttpError,
    ) -> Self {
        match error {
            crate::error::core::DeserialiseBodyFromHttpError::TwilightHttp(_) => Self::TwilightHttp,
            crate::error::core::DeserialiseBodyFromHttpError::DeserializeBody(_) => {
                Self::DeserializeBody
            }
        }
    }

    const fn from_respond(error: &'a crate::error::core::RespondError) -> Self {
        match error {
            crate::error::core::RespondError::TwilightHttp(_) => Self::TwilightHttp,
            crate::error::core::RespondError::Builder(_) => Self::Builder,
        }
    }

    const fn from_require_unsuppressed_error(
        error: &'a crate::error::command::require::UnsuppressedError,
    ) -> Self {
        match error {
            crate::error::command::require::UnsuppressedError::Cache(_) => Self::Cache,
            crate::error::command::require::UnsuppressedError::Suppressed(e) => Self::Suppressed(e),
        }
    }

    #[inline]
    const fn from_set_pause_with(
        error: &'a crate::error::command::require::SetPauseWithError,
    ) -> Self {
        Self::from_seek_to_with(error)
    }

    const fn from_seek_to_with(error: &'a crate::error::command::require::SeekToWithError) -> Self {
        match error {
            crate::error::command::require::SeekToWithError::Lavalink(e) => Self::Lavalink(e),
            crate::error::command::require::SeekToWithError::UpdateNowPlayingMessage(e) => {
                Self::from_update_now_playing_message(e)
            }
        }
    }

    const fn from_users_track_error(
        error: &'a crate::error::command::check::UsersTrackError,
    ) -> Self {
        match error {
            crate::error::command::check::UsersTrackError::Cache(_) => Self::Cache,
            crate::error::command::check::UsersTrackError::NotUsersTrack(e) => {
                Self::NotUsersTrack(e)
            }
        }
    }
}

impl ControllerError {
    #[must_use]
    pub const fn flatten_as(&self) -> Fe<'_> {
        match self {
            Self::Shuffle(e) => Fe::from_shuffle(e),
            Self::Back(e) => Fe::from_back(e),
            Self::PlayPause(e) => Fe::from_play_pause(e),
            Self::Skip(e) => Fe::from_skip(e),
            Self::Repeat(e) => Fe::from_repeat(e),
        }
    }
}
