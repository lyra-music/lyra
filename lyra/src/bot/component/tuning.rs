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
        model::{Ctx, CtxKind},
    },
    core::model::{BotStateAware, HttpAware},
    error::CommandResult,
    gateway::{voice, ExpectedGuildIdAware},
    lavalink::{DelegateMethods, ExpectedPlayerAware, LavalinkAware},
};

#[inline]
fn common_checks(ctx: &Ctx<impl CtxKind>) -> CommandResult {
    check::user_is_dj(ctx)?;
    check::in_voice(ctx)?;
    check::not_suppressed(ctx)?;
    check::player_exist(ctx)?;

    Ok(())
}

#[inline]
fn unmuting_checks(ctx: &Ctx<impl CtxKind>) -> CommandResult {
    check::user_is_dj(ctx)?;
    check::in_voice(ctx)?;

    Ok(())
}

#[inline]
fn unmuting_player_checks(ctx: &Ctx<impl CtxKind>) -> CommandResult {
    check::user_is_dj(ctx)?;
    check::in_voice(ctx)?;
    check::player_exist(ctx)?;

    Ok(())
}

trait UpdateFilter {
    fn apply(self, filter: Filters) -> Filters;
}

async fn set_filter(
    ctx: &(impl ExpectedPlayerAware + Sync),
    update: impl UpdateFilter + Send + Sync,
) -> LavalinkResult<()> {
    let player = ctx.player();
    let old_filter = player.get_player().await?.filters.unwrap_or_default();

    player.set_filters(update.apply(old_filter)).await?;
    Ok(())
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
            Some(NonZeroU16::new(100).expect("volume is non-zero"))
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
