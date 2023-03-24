use std::sync::Arc;

use super::manager::LyraBotManager;
use crate::bot::lib::models::LyraConfig;

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = LyraConfig::from_env();
    let mut bot_manager = LyraBotManager::new(config);
    let bot = Arc::new(bot_manager.build_bot().await?);

    bot.register_app_commands().await?;

    bot_manager.handle_events(bot.clone()).await?;

    Ok(())
}
