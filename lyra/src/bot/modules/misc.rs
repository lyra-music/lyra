use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::commands::models::{Context, LyraCommand};

#[derive(CreateCommand, CommandModel)]
#[command(name = "ping", desc = "Get the bot's latency.")]
pub struct Ping;

#[async_trait]
impl LyraCommand for Ping {
    async fn callback(&self, ctx: Context) -> anyhow::Result<()> {
        ctx.respond("Pong!").await?;

        Ok(())
    }
}
