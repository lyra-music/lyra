use anyhow::Result;
use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::commands::{
    macros::{caut, hid, out},
    models::{App, LyraCommand},
    Context,
};

#[derive(CreateCommand, CommandModel)]
#[command(name = "ping", desc = "Shows the bot's latency.")]
pub struct Ping;

#[async_trait]
impl LyraCommand for Ping {
    async fn execute(self, ctx: Context<App>) -> Result<()> {
        if let Some(latency) = ctx.bot().latency().average() {
            out!(format!("🏓 Pong! `({}ms)`", latency.as_millis()), ctx);
        } else {
            caut!(
                "Cannot calculate the ping at the moment, try again later.",
                ctx
            );
        }
    }
}
