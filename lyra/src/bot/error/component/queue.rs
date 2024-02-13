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
        LoadFailed(#[from] crate::bot::error::LoadFailed),
        #[error("no matches found: {}", .0)]
        NoMatches(Box<str>),
        #[error("search results found: {}", .0)]
        SearchResult(Box<str>),
    }

    #[derive(thiserror::Error, Debug)]
    #[error("playing failed: {:?}", .0)]
    pub enum Error {
        CheckNotSuppressed(#[from] crate::bot::error::command::check::NotSuppressedError),
        Respond(#[from] crate::bot::error::command::RespondError),
        Followup(#[from] crate::bot::error::command::FollowupError),
        AutoJoinOrCheckInVoiceWithUser(
            #[from] crate::bot::error::command::util::AutoJoinOrCheckInVoiceWithUserError,
        ),
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
    }
}

pub mod remove {
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error(transparent)]
    pub enum WithAdvanceLockAndStoppedError {
        Lavalink(#[from] lavalink_rs::error::LavalinkError),
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RemoveTracksError {
    #[error(transparent)]
    TryWithAdvanceLock(#[from] remove::WithAdvanceLockAndStoppedError),
    #[error(transparent)]
    Respond(#[from] crate::bot::error::command::RespondError),
    #[error(transparent)]
    Followup(#[from] crate::bot::error::command::FollowupError),
    #[error(transparent)]
    DeserializeBodyFromHttp(#[from] crate::bot::error::core::DeserializeBodyFromHttpError),
    #[error(transparent)]
    DeserializeBodyFromHttpArc(
        #[from] std::sync::Arc<crate::bot::error::core::DeserializeBodyFromHttpError>,
    ),
}
