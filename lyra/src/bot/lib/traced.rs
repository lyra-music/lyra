use std::future::Future;

pub fn tokio_spawn(fut: impl Future<Output = anyhow::Result<()>> + Send + 'static) {
    tokio::spawn(async move {
        if let Err(why) = fut.await {
            tracing::debug!("handler error: {why:?}");
        }
    });
}
