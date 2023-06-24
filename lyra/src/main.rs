mod bot;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    if let Err(why) = bot::run().await {
        tracing::error!("unhandled error: {why:#?}")
    }
}
