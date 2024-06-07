pub mod command;
pub mod component;
pub mod core;
pub mod gateway;
pub mod lavalink;
pub mod runner;

pub use command::Error as CommandError;
pub use command::Result as CommandResult;

use thiserror::Error;
use twilight_mention::Mention;
use twilight_model::id::{
    marker::{ChannelMarker, UserMarker},
    Id,
};

pub trait EPrint: std::error::Error + std::fmt::Debug {
    fn eprint(&self) -> String;
}

#[derive(Error, Debug)]
#[error("missing from cache")]
pub struct Cache;

pub type CacheResult<T> = Result<T, Cache>;

#[derive(Debug, Error)]
#[error("user is not a DJ")]
pub struct UserNotDj;

#[derive(Debug, Error)]
#[error("user is not a stage manager")]
pub struct UserNotStageManager;

#[derive(Debug, Error)]
#[error("user is not an access manager")]
pub struct UserNotAccessManager;

#[derive(Debug, Error)]
#[error("user is not a playlist manager")]
pub struct UserNotPlaylistManager;

#[derive(Error, Debug)]
#[error("user is not allowed to do this")]
pub struct UserNotAllowed;

#[derive(Error, Debug)]
#[error("bot is not in voice")]
pub struct NotInVoice;

#[derive(Error, Debug)]
#[error("neither user or bot is in voice")]
pub struct UserNotInVoice;

#[derive(Error, Debug)]
#[error("insufficient permissions to connect to voice channel: {}", .0)]
pub struct ConnectionForbidden(pub Id<ChannelMarker>);

#[derive(Error, Debug)]
#[error("bot is already in voice: {}", .0)]
pub struct InVoiceAlready(pub Id<ChannelMarker>);

#[derive(Error, Debug)]
#[error("bot is already in voice which you are not in: {}", .0)]
pub struct InVoiceWithoutUser(pub Id<ChannelMarker>);

#[derive(Error, Debug)]
#[error("bot is already in voice and someone else also is: {}", .0)]
pub struct InVoiceWithSomeoneElse(pub Id<ChannelMarker>);

impl EPrint for InVoiceWithSomeoneElse {
    fn eprint(&self) -> String {
        format!(
            "There are someone else in {}; You need to be a ***DJ*** to do that.",
            self.0.mention(),
        )
    }
}

#[derive(Error, Debug)]
#[error("bot is already in voice and you are the only one there: {}", .0)]
pub struct InVoiceWithoutSomeoneElse(pub Id<ChannelMarker>);

#[derive(Error, Debug)]
#[error("autojoin attempt failed: {}", .0)]
pub enum AutoJoinAttemptFailed {
    UserNotInVoice(#[from] UserNotInVoice),
    UserNotAllowed(#[from] UserNotAllowed),
    Forbidden(#[from] ConnectionForbidden),
    UserNotStageManager(#[from] UserNotStageManager),
}

#[derive(Error, Debug)]
pub enum Suppressed {
    #[error("bot is server muted")]
    Muted,
    #[error("bot has not become a speaker in stage yet")]
    NotSpeaker,
}

#[derive(Error, Debug)]
#[error("bot is not playing anything")]
pub struct NotPlaying;

#[derive(Error, Debug)]
#[error("queue is not seekable")]
pub struct QueueNotSeekable;

impl EPrint for QueueNotSeekable {
    fn eprint(&self) -> String {
        todo!()
    }
}

#[derive(Error, Debug)]
#[error("bot is playing a track the user didn't request")]
pub struct NotUsersTrack {
    pub requester: Id<UserMarker>,
    pub position: std::num::NonZeroUsize,
    pub title: std::sync::Arc<str>,
    pub channel_id: Id<ChannelMarker>,
}

impl EPrint for NotUsersTrack {
    fn eprint(&self) -> String {
        format!(
            "`{}` (`#{}`) was requested by {} and you're not the only person in {}; You'll need to be a ***DJ*** to do that.",
            self.title,
            self.position,
            self.requester.mention(),
            self.channel_id.mention(),
        )
    }
}

#[derive(Error, Debug)]
pub enum PositionOutOfRange {
    #[error("position is out of range 1..={}: {}", .queue_len, .position)]
    OutOfRange { position: i64, queue_len: usize },
    #[error("position is not 1: {}", .0)]
    OnlyTrack(i64),
}

#[derive(Error, Debug)]
#[error("player is paused")]
pub struct Paused;

#[derive(Error, Debug)]
#[error("player is stopped")]
pub struct Stopped;

#[derive(Error, Debug)]
#[error("queue is empty")]
pub struct QueueEmpty;

#[derive(Error, Debug)]
#[error("failed to load track: {}", .0)]
pub struct LoadFailed(pub Box<str>);

#[derive(PartialEq, Eq, Error, Debug)]
#[error("invalid timestamp")]
pub struct PrettifiedTimestampParse;

#[derive(Error, Debug)]
#[error("error running the bot starter: {}", .0)]
pub enum RunError {
    ColorEyre(#[from] color_eyre::Report),
    Dotenvy(#[from] dotenvy::Error),
    StartError(#[from] runner::StartError),
}

#[derive(Error, Debug)]
#[error("not in a guild")]
pub struct NotInGuild;
