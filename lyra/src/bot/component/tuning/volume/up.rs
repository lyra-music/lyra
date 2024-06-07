use std::num::NonZeroU16;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        macros::{note, out},
        model::BotSlashCommand,
        require, SlashCtx,
    },
    component::tuning::check_user_is_dj_and_require_player,
    core::model::{BotStateAware, HttpAware},
    error::CommandResult,
    gateway::GuildIdAware,
    lavalink::LavalinkAware,
};

/// Increase the playback volume
#[derive(CommandModel, CreateCommand)]
#[command(name = "up")]
pub struct Up {
    /// Increase the volume by how many percentages? [1~1000%] (If not given, 10%)
    #[command(min_value = 1, max_value = 1_000)]
    percent: Option<i64>,
}

impl BotSlashCommand for Up {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let (in_voice, player) = check_user_is_dj_and_require_player(&ctx)?;

        let lavalink = ctx.lavalink();
        let guild_id = ctx.guild_id();
        let data = player.data();
        let percent_u16 = self.percent.unwrap_or(10) as u16;

        // SAFETY: `1_000` is non-zero
        let max_percent = unsafe { NonZeroU16::new_unchecked(1_000) };
        let (old_percent_str, new_percent) = if lavalink.connection_from(&in_voice).mute {
            lavalink.connection_mut_from(&in_voice).mute = false;
            ctx.http()
                .update_guild_member(guild_id, ctx.bot().user_id())
                .mute(false)
                .await?;

            (
                String::from("Muted"),
                // SAFETY: `percent_u16` is in range [1, 1_000], so it is non-zero
                unsafe { NonZeroU16::new_unchecked(percent_u16) },
            )
        } else {
            let old_percent = data.read().await.volume();

            if old_percent >= max_percent {
                note!("Already at max playback volume.", ctx);
            }

            (
                format!("`{old_percent}%`"),
                old_percent.saturating_add(percent_u16).min(max_percent),
            )
        };

        let emoji = super::volume_emoji(Some(new_percent));
        let warning = super::clipping_warning(new_percent);

        let maxed_note = (new_percent == max_percent)
            .then_some(" (`Max`)")
            .unwrap_or_default();

        player.context.set_volume(new_percent.get()).await?;
        data.write().await.set_volume(new_percent);

        out!(
            format!(
                "{emoji}**`＋`** ~~{old_percent_str}~~ ➜ **`{new_percent}%`**{maxed_note}{warning}"
            ),
            ctx
        );
    }
}
