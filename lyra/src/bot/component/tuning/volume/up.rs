use std::num::NonZeroU16;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::bot::{
    command::{
        macros::{note, out},
        model::BotSlashCommand,
        SlashCtx,
    },
    component::tuning::unmuting_player_checks,
    core::model::{BotStateAware, HttpAware},
    error::CommandResult,
    gateway::ExpectedGuildIdAware,
    lavalink::{DelegateMethods, LavalinkAware},
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
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
        unmuting_player_checks(&ctx)?;

        let lavalink = ctx.lavalink();
        let guild_id = ctx.guild_id();
        let data = &lavalink.player_data(guild_id);
        let percent_u16 = self.percent.unwrap_or(10) as u16;

        let max_percent = NonZeroU16::new(1_000).expect("1_000 is non-zero");
        let (old_percent_str, new_percent) = if lavalink.connection(guild_id).mute {
            lavalink.connection_mut(guild_id).mute = false;
            ctx.http()
                .update_guild_member(guild_id, ctx.bot().user_id())
                .mute(false)
                .await?;

            (
                String::from("Muted"),
                NonZeroU16::new(percent_u16).expect("self.percent is non-zero"),
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

        lavalink
            .player(guild_id)
            .set_volume(new_percent.get())
            .await?;
        data.write().await.set_volume(new_percent);

        out!(
            format!(
                "{emoji}**`＋`** ~~{old_percent_str}~~ ➜ **`{new_percent}%`**{maxed_note}{warning}"
            ),
            ctx
        );
    }
}
