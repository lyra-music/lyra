use std::num::NonZeroU16;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{macros::out, model::BotSlashCommand, SlashCtx},
    component::tuning::common_checks,
    error::CommandResult,
    gateway::ExpectedGuildIdAware,
    lavalink::{DelegateMethods, LavalinkAware},
};

/// Set the playback volume
#[derive(CommandModel, CreateCommand)]
#[command(name = "set")]
pub struct Set {
    /// Set the volume to what percentage? [1~1000%]
    #[command(min_value = 1, max_value = 1_000)]
    percent: i64,
}

impl BotSlashCommand for Set {
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        common_checks(&ctx)?;

        let percent = NonZeroU16::new(self.percent as u16).expect("self.percent is non-zero");
        let lavalink = ctx.lavalink();
        let guild_id = ctx.guild_id();
        lavalink.player(guild_id).set_volume(percent.get()).await?;
        lavalink
            .player_data(guild_id)
            .write()
            .await
            .set_volume(percent);

        let emoji = super::volume_emoji(Some(percent));
        let warning = super::clipping_warning(percent);

        out!(
            format!("{emoji} Set playback volume to `{percent}`%{warning}."),
            ctx
        );
    }
}
