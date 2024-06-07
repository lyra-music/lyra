mod equaliser;
mod filter;
mod speed;
mod volume;

use std::num::NonZeroU16;

pub use equaliser::Equaliser;
pub use filter::Filter;
use lavalink_rs::{error::LavalinkResult, model::player::Filters};
pub use speed::Speed;
pub use volume::Volume;

use crate::bot::{
    command::{
        check,
        model::{CtxKind, GuildCtx},
        require::{self, InVoice, Player},
    },
    core::model::{BotStateAware, HttpAware},
    error::CommandError,
    gateway::{voice, GuildIdAware},
    lavalink::{DelegateMethods, LavalinkAware},
};

#[inline]
fn check_user_is_dj_and_require_unsuppressed_player(
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<(InVoice, Player), CommandError> {
    check::user_is_dj(ctx)?;
    let in_voice = require::in_voice(ctx)?.and_unsuppressed()?;
    let player = require::player(ctx)?;

    Ok((in_voice, player))
}

#[inline]
fn unmuting_checks(ctx: &GuildCtx<impl CtxKind>) -> Result<InVoice, CommandError> {
    check::user_is_dj(ctx)?;
    let in_voice = require::in_voice(ctx)?;

    Ok(in_voice)
}

#[inline]
fn check_user_is_dj_and_require_player(
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<(InVoice, Player), CommandError> {
    check::user_is_dj(ctx)?;
    let in_voice = require::in_voice(ctx)?;
    let player = require::player(ctx)?;

    Ok((in_voice, player))
}

trait ApplyFilter {
    fn apply_to(self, filter: Filters) -> Filters;
}

trait UpdateFilter {
    async fn update_filter(&self, update: impl ApplyFilter + Send + Sync) -> LavalinkResult<()>;
}

impl UpdateFilter for Player {
    async fn update_filter(&self, update: impl ApplyFilter + Send + Sync) -> LavalinkResult<()> {
        let old_filter = self.info().await?.filters.unwrap_or_default();

        self.context
            .set_filters(update.apply_to(old_filter))
            .await?;
        Ok(())
    }
}

#[tracing::instrument(skip_all, name = "voice_state_update")]
pub async fn handle_voice_state_update(ctx: &voice::Context) -> Result<(), twilight_http::Error> {
    let bot = ctx.bot();
    let guild_id = ctx.guild_id();
    let lavalink = bot.lavalink();
    let Some(mut connection) = lavalink.get_connection_mut(guild_id) else {
        return Ok(());
    };

    let state_mute = ctx.inner.mute;
    if connection.mute != state_mute {
        connection.mute = state_mute;

        let emoji = volume::volume_emoji(if state_mute {
            None
        } else if let Some(d) = lavalink.get_player_data(guild_id) {
            Some(d.read().await.volume())
        } else {
            // SAFETY: `100` is non-zero
            Some(unsafe { NonZeroU16::new_unchecked(100) })
        });
        let describe = if state_mute { "muted" } else { "unmuted" };

        tracing::warn!("guild {} {} forcefully", guild_id, describe);
        bot.http()
            .create_message(connection.text_channel_id)
            .content(&format!("{emoji} `(Bot was forcefully {describe})`"))
            .await?;
    }
    Ok(())
}
