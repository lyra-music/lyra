use std::sync::Arc;

// use tracing::Level;

use super::manager::BotManager;
use crate::bot::lib::models::LyraConfig;

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        // .with_max_level(Level::DEBUG)
        .init();

    let config = LyraConfig::from_env();
    let bot_manager = BotManager::new(config);
    let bot = Arc::new(bot_manager.build_bot().await?);

    bot.register_app_commands().await?;

    tokio::try_join!(
        bot_manager.handle_gateway_events(bot.clone()),
        bot_manager.handle_lavalink_events(bot.clone()),
        bot_manager.handle_shutdown(bot),
    )?;

    tracing::info!("shut down gracefully");
    Ok(())
}
