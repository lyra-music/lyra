use std::{fmt::Debug, future::Future};

use tokio::task::JoinHandle;

pub fn tokio_spawn<E: Debug>(
    fut: impl Future<Output = Result<(), E>> + Send + 'static,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(why) = fut.await {
            tracing::error!("task error: {why:#?}");
        }
    })
}
