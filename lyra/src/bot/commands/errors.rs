use thiserror::Error;
use twilight_model::id::{marker::ChannelMarker, Id};

#[derive(Error, Debug)]
pub enum Error {
    #[error("missing from cache")]
    Cache,
    #[error("user is not in voice")]
    UserNotInVoice,
    #[error("bot is not in channel")]
    NotInVoice,
    #[error("errors regarding the bot's voice channel connection")]
    Connection {
        channel_id: Id<ChannelMarker>,
        source: ConnectionError,
    },
    #[error("user was not allowed to do this")]
    UserNotAllowed,
    #[error("errors regarding player queues")]
    Queue(QueueError),

    #[error(transparent)]
    Other(anyhow::Error),
}

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("bot is already in voice")]
    AlreadyInVoice(#[from] AlreadyInVoiceError),
    #[error("insufficient permissions")]
    Forbidden,
}

#[derive(Error, Debug)]
pub enum AlreadyInVoiceError {
    #[error("bot is already in that voice")]
    SameVoice,
    #[error("bot is already in some other voice and someone else is also in it")]
    SomeoneElseInVoice,
}

#[derive(Error, Debug)]
pub enum QueueError {
    #[error("queue is empty")]
    Empty,
}

pub type Result<T> = std::result::Result<T, Error>;
