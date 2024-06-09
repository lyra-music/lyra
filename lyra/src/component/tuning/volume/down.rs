use std::num::NonZeroU16;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    command::{macros::out, model::BotSlashCommand, require, SlashCtx},
    component::tuning::check_user_is_dj_and_require_unsuppressed_player,
    core::model::{BotStateAware, HttpAware},
    error::CommandResult,
    gateway::GuildIdAware,
    lavalink::LavalinkAware,
};

/// Decrease the playback volume
#[derive(CommandModel, CreateCommand)]
#[command(name = "down")]
pub struct Down {
    /// Decrease the volume by how many percentages? [1~1000%] (If not given, 10%)
    #[command(min_value = 1, max_value = 1_000)]
    percent: Option<i64>,
}

impl BotSlashCommand for Down {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (in_voice, player) = check_user_is_dj_and_require_unsuppressed_player(&ctx)?;

        let guild_id = ctx.guild_id();
        let data = player.data();
        let old_percent = data.read().await.volume();

        let maybe_new_percent = old_percent
            .get()
            .checked_sub(self.percent.unwrap_or(10) as u16)
            .and_then(NonZeroU16::new);

        let emoji = super::volume_emoji(maybe_new_percent);
        let (new_percent_str, warning) = if let Some(new_percent) = maybe_new_percent {
            player.context.set_volume(new_percent.get()).await?;
            data.write().await.set_volume(new_percent);

            (
                format!("`{new_percent}%`"),
                super::clipping_warning(new_percent),
            )
        } else {
            ctx.lavalink().connection_mut_from(&in_voice).mute = true;
            ctx.http()
                .update_guild_member(guild_id, ctx.bot().user_id())
                .mute(true)
                .await?;

            (String::from("Muted"), "")
        };

        out!(
            format!("{emoji}**`ー`** ~~{old_percent}%~~ ➜ **{new_percent_str}**{warning}"),
            ctx
        );
    }
}
