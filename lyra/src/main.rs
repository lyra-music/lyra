mod bot;

#[tokio::main]
#[tracing::instrument]
async fn main() {
    color_eyre::install().ok();
    dotenvy::dotenv().ok();

    let _ = bot::run().await;
}
