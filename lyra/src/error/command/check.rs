use thiserror::Error;

use crate::error::{
    self, Cache as CacheError, InVoiceWithoutUser as InVoiceWithoutUserError, NotInVoice,
    NotUsersTrack as NotUsersTrackError, PrettyErrorDisplay, PrettyInVoiceWithSomeoneElseDisplayer,
    PrettyNotUsersTrackDisplayer, PrettyQueueNotSeekableDisplayer,
    QueueNotSeekable as QueueNotSeekableError,
};

use super::require::UnsuppressedError;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum AccessCalculatorBuildError {
    Sqlx(#[from] sqlx::Error),
    TaskJoin(#[from] tokio::task::JoinError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UserAllowedError {
    AccessCalculatorBuild(#[from] AccessCalculatorBuildError),
    UserNotAllowed(#[from] error::UserNotAllowed),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UserOnlyInError {
    Cache(#[from] CacheError),
    InVoiceWithSomeoneElse(#[from] crate::error::InVoiceWithSomeoneElse),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum InVoiceWithUserError {
    NotInVoice(#[from] NotInVoice),
    InVoiceWithoutUser(#[from] InVoiceWithoutUserError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum InVoiceWithUserOnlyError {
    NotInVoice(#[from] NotInVoice),
    InVoiceWithoutUser(#[from] InVoiceWithoutUserError),
    UserOnlyIn(#[from] UserOnlyInError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum CurrentlyPlayingUsersTrackError {
    NotPlaying(#[from] error::NotPlaying),
    NotUsersTrack(#[from] NotUsersTrackError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum QueueSeekableError {
    QueueNotSeekable(#[from] QueueNotSeekableError),
    NotUsersTrack(#[from] NotUsersTrackError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UsersTrackError {
    Cache(#[from] CacheError),
    NotUsersTrack(#[from] NotUsersTrackError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum PollResolvableError {
    InVoiceWithSomeoneElse(#[from] crate::error::InVoiceWithSomeoneElse),
    QueueNotSeekable(#[from] QueueNotSeekableError),
    NotUsersTrack(#[from] NotUsersTrackError),
}

impl From<QueueSeekableError> for PollResolvableError {
    fn from(value: QueueSeekableError) -> Self {
        match value {
            QueueSeekableError::QueueNotSeekable(e) => e.into(),
            QueueSeekableError::NotUsersTrack(e) => e.into(),
        }
    }
}

impl<'a> PrettyErrorDisplay<'a> for PollResolvableError {
    type Displayer = PrettyPollResolvableErrorDisplayer<'a>;

    fn pretty_display(&'a self) -> Self::Displayer {
        match self {
            Self::InVoiceWithSomeoneElse(e) => {
                PrettyPollResolvableErrorDisplayer::InVoiceWithSomeoneElse(
                    PrettyInVoiceWithSomeoneElseDisplayer(e),
                )
            }
            Self::QueueNotSeekable(_) => PrettyPollResolvableErrorDisplayer::QueueNotSeekable(
                PrettyQueueNotSeekableDisplayer,
            ),
            Self::NotUsersTrack(e) => {
                PrettyPollResolvableErrorDisplayer::NotUsersTrack(PrettyNotUsersTrackDisplayer(e))
            }
        }
    }
}

pub enum PrettyPollResolvableErrorDisplayer<'a> {
    InVoiceWithSomeoneElse(PrettyInVoiceWithSomeoneElseDisplayer<'a>),
    NotUsersTrack(PrettyNotUsersTrackDisplayer<'a>),
    QueueNotSeekable(PrettyQueueNotSeekableDisplayer),
}

impl<'a> std::fmt::Display for PrettyPollResolvableErrorDisplayer<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrettyPollResolvableErrorDisplayer::InVoiceWithSomeoneElse(e) => e.fmt(f),
            PrettyPollResolvableErrorDisplayer::NotUsersTrack(e) => e.fmt(f),
            PrettyPollResolvableErrorDisplayer::QueueNotSeekable(e) => e.fmt(f),
        }
    }
}

#[derive(Error, Debug)]
#[error("another poll is ongoing in the same guild")]
pub struct AnotherPollOngoingError {
    pub message: crate::command::util::MessageLinkComponent,
    pub alternate_vote: Option<AlternateVoteResponse>,
}

#[derive(Debug)]
pub enum AlternateVoteResponse {
    Casted,
    DjCasted,
    CastDenied,
    CastedAlready(crate::command::poll::Vote),
}

#[derive(Error, Debug)]
#[error("poll was voided")]
pub struct PollVoidedError(pub crate::command::poll::VoidingEvent);

impl<'a> PrettyErrorDisplay<'a> for PollVoidedError {
    type Displayer = PrettyVoidedErrorDisplayer;

    fn pretty_display(&'a self) -> Self::Displayer {
        match self.0 {
            crate::command::poll::VoidingEvent::QueueClear => {
                PrettyVoidedErrorDisplayer::QueueClear
            }
            crate::command::poll::VoidingEvent::QueueRepeat => {
                PrettyVoidedErrorDisplayer::QueueRepeat
            }
        }
    }
}

pub enum PrettyVoidedErrorDisplayer {
    QueueClear,
    QueueRepeat,
}

impl std::fmt::Display for PrettyVoidedErrorDisplayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = match self {
            Self::QueueClear => "the queue had been cleared",
            Self::QueueRepeat => "the queue had been set to repeat in another manner",
        };
        f.write_str(data)
    }
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum HandlePollError {
    AnotherPollOngoing(#[from] AnotherPollOngoingError),
    StartPoll(#[from] super::poll::StartPollError),
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::lavalink::Event>),
    DeserializeBodyFromHttp(#[from] crate::error::core::DeserializeBodyFromHttpError),
    PollLoss(#[from] PollLossError),
    PollVoided(#[from] PollVoidedError),
    EventRecv(#[from] tokio::sync::broadcast::error::RecvError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum SendSupersededWinNoticeError {
    DeserializeBodyFromHttp(#[from] crate::error::core::DeserializeBodyFromHttpError),
    Http(#[from] twilight_http::Error),
    MessageValidation(#[from] twilight_validate::message::MessageValidationError),
}

#[derive(Error, Debug)]
#[error("poll timed out: {}", .source)]
pub struct PollLossError {
    pub source: PollResolvableError,
    pub kind: PollLossErrorKind,
}

#[derive(Debug)]
pub enum PollLossErrorKind {
    UnanimousLoss,
    TimedOut,
    SupersededLossViaDj,
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum HandleInVoiceWithSomeoneElseError {
    PollResolvable(#[from] PollResolvableError),
    HandlePoll(#[from] HandlePollError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum RunError {
    NotInVoice(#[from] NotInVoice),
    QueueEmpty(#[from] error::QueueEmpty),
    NotSuppressed(#[from] UnsuppressedError),
    NotPlaying(#[from] error::NotPlaying),
    InVoiceWithoutUser(#[from] InVoiceWithoutUserError),
    HandleInVoiceWithSomeoneElse(#[from] HandleInVoiceWithSomeoneElseError),
    Cache(#[from] CacheError),
    Paused(#[from] error::Paused),
    Stopped(#[from] error::Stopped),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum OnlyElsePoll {
    Cache(#[from] CacheError),
    HandlePoll(#[from] HandlePollError),
}
