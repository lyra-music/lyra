mod bot;

#[tokio::main]
#[tracing::instrument]
async fn main() {
    let _ = bot::run().await;
}
