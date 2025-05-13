pub mod play {
    #[derive(thiserror::Error, Debug)]
    #[error("loading many tracks failed: {:?}", .0)]
    pub enum LoadTrackProcessManyError {
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
        Query(#[from] QueryError),
    }

    #[derive(thiserror::Error, Debug)]
    pub enum QueryError {
        #[error(transparent)]
        LoadFailed(#[from] crate::error::LoadFailed),
        #[error("no matches found: {}", .0)]
        NoMatches(Box<str>),
        #[error("search results found: {}", .0)]
        SearchResult(Box<str>),
    }

    #[derive(thiserror::Error, Debug)]
    #[error("playing failed: {:?}", .0)]
    pub enum Error {
        TwilightHttp(#[from] twilight_http::Error),
        RespondOrFollowup(#[from] crate::error::command::RespondOrFollowupError),
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
        HandleLoadTrackResults(#[from] HandleLoadTrackResultsError),
    }

    #[derive(thiserror::Error, Debug)]
    #[error(transparent)]
    pub enum HandleLoadTrackResultsError {
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
        RespondOrFollowup(#[from] crate::error::command::RespondOrFollowupError),
        RequireUnsuppressed(#[from] crate::error::command::require::UnsuppressedError),
        AutoJoinOrCheckInVoiceWithUser(
            #[from] crate::error::command::util::AutoJoinOrCheckInVoiceWithUserError,
        ),
        UpdateNowPlayingMessage(#[from] crate::error::lavalink::UpdateNowPlayingMessageError),
    }
}

pub mod repeat {
    #[derive(thiserror::Error, Debug)]
    #[error(transparent)]
    pub enum Error {
        UnrecognisedConnection(#[from] crate::error::UnrecognisedConnection),
        TwilightHttp(#[from] twilight_http::Error),
        UpdateNowPlayingMessage(#[from] crate::error::lavalink::UpdateNowPlayingMessageError),
    }
}

pub use repeat::Error as RepeatError;

use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum RemoveTracksError {
    Lavalink(#[from] lavalink_rs::error::LavalinkError),
    TwilightHttp(#[from] twilight_http::Error),
    Followup(#[from] crate::error::command::FollowupError),
    DeserialiseBodyFromHttp(#[from] crate::error::core::DeserialiseBodyFromHttpError),
    UpdateNowPlayingMessage(#[from] crate::error::lavalink::UpdateNowPlayingMessageError),
}
