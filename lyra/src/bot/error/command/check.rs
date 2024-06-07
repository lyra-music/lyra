use thiserror::Error;

use crate::bot::error::{
    self, Cache as CacheError, InVoiceWithoutUser as InVoiceWithoutUserError, NotInVoice,
    NotUsersTrack as NotUsersTrackError, QueueNotSeekable as QueueNotSeekableError,
};

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
pub enum InVoiceWithSomeoneElseError {
    Cache(#[from] CacheError),
    InVoiceWithoutSomeoneElse(#[from] error::InVoiceWithoutSomeoneElse),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum UserOnlyInError {
    Cache(#[from] CacheError),
    InVoiceWithSomeoneElse(#[from] crate::bot::error::InVoiceWithSomeoneElse),
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
pub enum NotSuppressedError {
    Cache(#[from] CacheError),
    Suppressed(#[from] error::Suppressed),
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
    InVoiceWithSomeoneElse(#[from] crate::bot::error::InVoiceWithSomeoneElse),
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

impl crate::bot::error::EPrint for PollResolvableError {
    fn eprint(&self) -> String {
        match self {
            Self::InVoiceWithSomeoneElse(e) => e.eprint(),
            Self::QueueNotSeekable(e) => e.eprint(),
            Self::NotUsersTrack(e) => e.eprint(),
        }
    }
}

#[derive(Error, Debug)]
#[error("another poll is ongoing in the same guild")]
pub struct AnotherPollOngoingError {
    pub message: crate::bot::command::util::MessageLinkComponent,
    pub alternate_vote: Option<AlternateVoteResponse>,
}

#[derive(Debug)]
pub enum AlternateVoteResponse {
    Casted,
    DjCasted,
    CastDenied,
    CastedAlready(crate::bot::command::poll::Vote),
}

#[derive(Error, Debug)]
#[error("poll was voided")]
pub struct PollVoidedError(pub crate::bot::command::poll::VoidingEvent);

impl crate::bot::error::EPrint for PollVoidedError {
    fn eprint(&self) -> String {
        match self.0 {
            crate::bot::command::poll::VoidingEvent::QueueClear => {
                String::from("the queue had been cleared")
            }
            crate::bot::command::poll::VoidingEvent::QueueRepeat => {
                String::from("the queue had been set to repeat in another manner")
            }
        }
    }
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum HandlePollError {
    AnotherPollOngoing(#[from] AnotherPollOngoingError),
    StartPoll(#[from] super::poll::StartPollError),
    EventSend(#[from] tokio::sync::broadcast::error::SendError<crate::bot::lavalink::Event>),
    DeserializeBodyFromHttp(#[from] crate::bot::error::core::DeserializeBodyFromHttpError),
    PollLoss(#[from] PollLossError),
    PollVoided(#[from] PollVoidedError),
    EventRecv(#[from] tokio::sync::broadcast::error::RecvError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum SendSupersededWinNoticeError {
    DeserializeBodyFromHttp(#[from] crate::bot::error::core::DeserializeBodyFromHttpError),
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
    NotSuppressed(#[from] NotSuppressedError),
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
