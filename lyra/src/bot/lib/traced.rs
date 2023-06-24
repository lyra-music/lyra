use std::future::Future;

use anyhow::Result;

pub fn tokio_spawn(
    fut: impl Future<Output = Result<()>> + Send + 'static,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(why) = fut.await {
            tracing::error!("task error: {why:#?}");
        }
    })
}
