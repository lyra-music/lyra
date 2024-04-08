use std::{error::Error, fmt::Debug, future::Future};

use tokio::task::JoinHandle;
use tracing::Instrument;

pub fn tokio_spawn<E, F>(fut: F) -> JoinHandle<()>
where
    E: Error + Debug,
    F: Future<Output = Result<(), E>> + Send + 'static,
{
    tokio::spawn(
        async move {
            if let Err(error) = fut.await {
                tracing::error!(%error);
            }
        }
        .instrument(tracing::trace_span!("task")),
    )
}
