mod bot;

#[tokio::main]
async fn main() {
    color_eyre::install().ok();
    dotenvy::dotenv().ok();
    if let Err(why) = bot::run().await {
        tracing::error!("unhandled error: {why:#?}");
    }
}
