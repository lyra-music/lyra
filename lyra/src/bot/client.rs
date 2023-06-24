// use tracing::Level;

use anyhow::Result;

use super::manager::BotManager;
use crate::bot::lib::models::Config;

pub async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        // .with_max_level(Level::DEBUG)
        .init();

    let config = Config::from_env();
    let bot_manager = BotManager::new(config);

    bot_manager.start().await?;

    tracing::info!("shut down gracefully");
    Ok(())
}
