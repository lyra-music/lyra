use super::{error::manager::StartError, manager::BotManager};
use crate::bot::core::model::Config;

pub async fn run() -> Result<(), StartError> {
    tracing_subscriber::fmt().compact().init();

    let config = Config::from_env();
    let mut bot_manager = BotManager::new(config);

    bot_manager.start().await?;
    Ok(())
}
