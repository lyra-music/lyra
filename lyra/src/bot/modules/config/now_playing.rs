use anyhow::Result;
use async_trait::async_trait;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_model::id::{marker::CommandMarker, Id};

use crate::bot::commands::{
    macros::out,
    models::{App, Context, LyraCommand, ResolvedCommandInfo},
};
use lyra_proc::LyraCommandGroup;

#[derive(CommandModel, CreateCommand, LyraCommandGroup)]
#[command(name = "now-playing", desc = ".")]
pub enum NowPlaying {
    #[command(name = "toggle")]
    Toggle(Toggle),
}

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "toggle",
    desc = "Toggles whether now-playing track messages should be automatically sent or not"
)]
pub struct Toggle;

#[async_trait]
impl LyraCommand for Toggle {
    async fn execute(self, ctx: Context<App>) -> Result<()> {
        let new_now_playing = sqlx::query!(
            r#"--sql
            UPDATE guild_configs SET now_playing = NOT now_playing WHERE id = $1 RETURNING now_playing;
            "#,
            ctx.guild_id_unchecked().get() as i64,
        )
        .fetch_one(ctx.db())
        .await?
        .now_playing;

        let (emoji, action) = match new_now_playing {
            true => ("🔔", "Sending"),
            false => ("🔕", "Not sending"),
        };

        out!(
            format!("{emoji} **{action}** now-playing track messages from now on."),
            ctx
        );
    }
}
