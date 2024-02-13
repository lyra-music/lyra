use thiserror::Error;

#[derive(Error, Debug)]
#[error("processing lavalink event failed: {:?}", .0)]
pub enum ProcessError {}

pub type ProcessResult = Result<(), ProcessError>;
