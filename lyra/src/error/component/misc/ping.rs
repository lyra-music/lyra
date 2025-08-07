use thiserror::Error;

use crate::error::core::RespondError;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum PingError {
    Respond(#[from] RespondError),
    #[error("no heartbeat has been sent")]
    #[expect(unused)]
    NoHeartbeatSent,
}

#[derive(Error, Debug)]
#[error(transparent)]
pub enum PingResidualError {
    Respond(#[from] RespondError),
}
