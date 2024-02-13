use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        macros::out,
        model::{BotSlashCommand, CommandInfoAware, Ctx, SlashCommand},
    },
    error::command::Result as CommandResult,
    gateway::ExpectedGuildIdAware,
};
use lyra_proc::BotCommandGroup;

/// -
#[derive(CommandModel, CreateCommand, BotCommandGroup)]
#[command(name = "now-playing")]
pub enum NowPlaying {
    #[command(name = "toggle")]
    Toggle(Toggle),
}

/// Toggles whether now-playing track messages should be automatically sent or not
#[derive(CommandModel, CreateCommand)]
#[command(name = "toggle")]
pub struct Toggle;

impl BotSlashCommand for Toggle {
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
        let new_now_playing = sqlx::query!(
            r"--sql
            UPDATE guild_configs SET now_playing = NOT now_playing WHERE id = $1 RETURNING now_playing;
            ",
            ctx.guild_id().get() as i64,
        )
        .fetch_one(ctx.db())
        .await?
        .now_playing;

        let (emoji, action) = if new_now_playing {
            ("🔔", "Sending")
        } else {
            ("🔕", "Not sending")
        };

        out!(
            format!("{emoji} **{action}** now-playing track messages from now on."),
            ctx
        );
    }
}
