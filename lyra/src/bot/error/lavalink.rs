use thiserror::Error;

#[derive(Error, Debug)]
#[error("processing lavalink event failed: {:?}", .0)]
pub enum ProcessError {
    Client(#[from] twilight_lavalink::client::ClientError),
    NodeSender(#[from] twilight_lavalink::node::NodeSenderError),
}

pub type ProcessResult = Result<(), ProcessError>;
