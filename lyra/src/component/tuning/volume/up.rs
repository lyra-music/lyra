use std::num::NonZeroU16;

use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::{
    LavalinkAndGuildIdAware,
    command::{SlashCtx, model::BotSlashCommand, require},
    component::tuning::check_user_is_dj_and_require_player,
    core::model::{
        BotStateAware, HttpAware, response::initial::message::create::RespondWithMessage,
    },
    error::CommandResult,
    gateway::GuildIdAware,
};

/// Increases the playback volume.
#[derive(CommandModel, CreateCommand)]
#[command(name = "up")]
pub struct Up {
    /// Increase the volume by how many percentages? [1~1000%] (If not given, 10%)
    #[command(min_value = 1, max_value = 1_000)]
    percent: Option<i64>,
}

impl BotSlashCommand for Up {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        const MAX_PERCENT: NonZeroU16 = NonZeroU16::new(1_000).expect("`1_000 is non-zero");

        let mut ctx = require::guild(ctx)?;
        let (_, player) = check_user_is_dj_and_require_player(&ctx)?;

        let guild_id = ctx.guild_id();
        let data = player.data();
        #[expect(clippy::cast_possible_truncation)]
        let percent_u16 = self.percent.unwrap_or(10).unsigned_abs() as u16;

        let conn = ctx.get_conn();

        let (old_percent_str, new_percent) = if conn.get_head().await?.mute() {
            conn.set_mute(false);
            ctx.http()
                .update_guild_member(guild_id, ctx.bot().user_id())
                .mute(false)
                .await?;

            (
                String::from("Muted"),
                NonZeroU16::new(percent_u16).expect("percent should be non-zero"),
            )
        } else {
            let old_percent = data.read().await.volume();

            if old_percent >= MAX_PERCENT {
                ctx.note("Already at max playback volume.").await?;
                return Ok(());
            }

            (
                format!("`{old_percent}%`"),
                old_percent.saturating_add(percent_u16).min(MAX_PERCENT),
            )
        };

        let emoji = super::volume_emoji(Some(new_percent));
        let warning = super::clipping_warning(new_percent);

        let maxed_note = (new_percent == MAX_PERCENT)
            .then_some(" (`Max`)")
            .unwrap_or_default();

        player.context.set_volume(new_percent.get()).await?;
        data.write().await.set_volume(new_percent);

        ctx.out(format!(
            "{emoji}**`＋`** ~~{old_percent_str}~~ ➜ **`{new_percent}%`**{maxed_note}{warning}."
        ))
        .await?;
        Ok(())
    }
}
