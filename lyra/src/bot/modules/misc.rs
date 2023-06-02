use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::commands::models::{Context, LyraCommand};
use lyra_proc::{err, out};

#[derive(CreateCommand, CommandModel)]
#[command(name = "ping", desc = "Shows the bot's latency.")]
pub struct Ping;

#[async_trait]
impl LyraCommand for Ping {
    async fn callback(&self, ctx: Context) -> anyhow::Result<()> {
        if let Some(latency) = ctx.bot().latency().await.average() {
            out!(&format!("🏓 Pong! `({}ms)`", latency.as_millis()));
        } else {
            err!("‼️ Cannot calculate the ping at the moment, try again later.");
        }
    }
}
