use std::sync::Arc;

use crate::bot::interactions;
use crate::bot::lib::models::LyraConfig;

use super::lib::logging;
use super::manager::LyraBotManager;

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = LyraConfig::from_env();
    let mut bot_manager = LyraBotManager::new(config);
    let bot = Arc::new(bot_manager.build_bot().await?);

    logging::spawn(interactions::server::start());
    bot.register_app_commands().await?;

    bot_manager.handle_events(Arc::clone(&bot)).await?;

    Ok(())
}
