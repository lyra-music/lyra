mod bot;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::from_path("../.env")?;
    bot::run().await
}
