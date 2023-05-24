use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::commands::models::{Context, LyraCommand};

#[derive(CreateCommand, CommandModel)]
#[command(name = "ping", desc = "Shows the bot's latency.")]
pub struct Ping;

#[async_trait]
impl LyraCommand for Ping {
    async fn callback(&self, ctx: Context) -> anyhow::Result<()> {
        match ctx.bot().latency().await.average() {
            Some(latency) => {
                ctx.respond(&format!("🏓 Pong! `({}ms)`", latency.as_millis()))
                    .await?;
            }
            None => {
                ctx.ephem("‼️ Cannot calculate the ping at the moment, try again later.")
                    .await?;
            }
        }

        Ok(())
    }
}
