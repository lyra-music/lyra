use std::{error::Error, future::Future};

use tokio::task::JoinHandle;
use tracing::Instrument;

pub fn tokio_spawn(
    fut: impl Future<Output = Result<(), impl Error>> + Send + 'static,
) -> JoinHandle<()> {
    tokio::spawn(
        async move {
            if let Err(error) = fut.await {
                tracing::error!(%error);
            }
        }
        .instrument(tracing::trace_span!("task")),
    )
}
