pub mod play {
    #[derive(thiserror::Error, Debug)]
    #[error("loading track error: {}", .0)]
    pub enum LoadTrackProcessError {
        Http(#[from] http::Error),
        Hyper(#[from] hyper::Error),
        SerdeJson(#[from] serde_json::Error),
    }

    #[derive(thiserror::Error, Debug)]
    #[error("loading many tracks failed: {:?}", .0)]
    pub enum LoadTrackProcessManyError {
        Process(#[from] LoadTrackProcessError),
        Query(#[from] QueryError),
        UnknownLoadType(#[from] UnknownLoadTypeError),
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
    #[error("unknown load type: {:?}", .0)]
    pub struct UnknownLoadTypeError(pub twilight_lavalink::http::LoadType);

    #[derive(thiserror::Error, Debug)]
    #[error("playing failed: {:?}", .0)]
    pub enum Error {
        NodeSender(#[from] twilight_lavalink::node::NodeSenderError),
        Client(#[from] twilight_lavalink::client::ClientError),
        CheckNotSuppressed(#[from] crate::bot::error::command::check::NotSuppressedError),
        Respond(#[from] crate::bot::error::command::RespondError),
        Followup(#[from] crate::bot::error::command::FollowupError),
        AutoJoinOrCheckInVoiceWithUser(
            #[from] crate::bot::error::command::util::AutoJoinOrCheckInVoiceWithUserError,
        ),
        LoadTrackProcess(#[from] LoadTrackProcessError),
        UnknownLoadType(#[from] UnknownLoadTypeError),
    }
}

pub mod remove {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum WithAdvanceLockAndStoppedError {
        #[error(transparent)]
        Client(#[from] twilight_lavalink::client::ClientError),
        #[error(transparent)]
        NodeSender(#[from] twilight_lavalink::node::NodeSenderError),
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
