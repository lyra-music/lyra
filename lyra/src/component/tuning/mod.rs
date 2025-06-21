mod equaliser;
mod filter;
mod speed;
mod volume;

pub use equaliser::Equaliser;
pub use filter::Filter;
pub use speed::Speed;
pub use volume::Volume;

use std::num::NonZeroU16;

use lavalink_rs::{error::LavalinkResult, model::player::Filters};

use crate::{
    LavalinkAndGuildIdAware, LavalinkAware,
    command::{
        model::{CtxKind, GuildCtx},
        require::{self, InVoice, PlayerInterface},
    },
    core::model::{BotStateAware, HttpAware},
    error::component::tuning::RequireInVoiceUnsuppressedAndPlayerError,
    gateway::{GuildIdAware, voice},
    lavalink::{ConnectionHead, DelegateMethods},
};

#[inline]
fn require_in_voice_unsuppressed_and_player(
    ctx: &GuildCtx<impl CtxKind>,
) -> Result<(InVoice, PlayerInterface), RequireInVoiceUnsuppressedAndPlayerError> {
    let in_voice = require::in_voice(ctx)?.and_unsuppressed()?;
    let player = require::player(ctx)?;

    Ok((in_voice, player))
}

trait ApplyFilter {
    fn apply_to(self, filter: Filters) -> Filters;
}

trait UpdateFilter {
    async fn update_filter(&self, update: impl ApplyFilter + Send + Sync) -> LavalinkResult<()>;
}

impl UpdateFilter for PlayerInterface {
    async fn update_filter(&self, update: impl ApplyFilter + Send + Sync) -> LavalinkResult<()> {
        let old_filter = self.info().await?.filters.unwrap_or_default();

        self.context
            .set_filters(update.apply_to(old_filter))
            .await?;
        Ok(())
    }
}

#[tracing::instrument(skip_all, name = "tuning")]
pub async fn handle_voice_state_update(
    ctx: &voice::Context,
    head: ConnectionHead,
) -> Result<(), twilight_http::Error> {
    tracing::debug!("handling voice state update");

    let bot = ctx.bot();
    let guild_id = ctx.guild_id();
    let conn = ctx.get_conn();
    let state_mute = ctx.inner.mute;
    if head.mute() != state_mute {
        conn.set_mute(state_mute);

        let emoji = volume::volume_emoji(if state_mute {
            None
        } else if let Some(d) = bot.lavalink().get_player_data(guild_id) {
            Some(d.read().await.volume())
        } else {
            Some(NonZeroU16::new(100).expect("100 must be non-zero"))
        });
        let describe = if state_mute { "muted" } else { "unmuted" };

        tracing::warn!("guild {} {} forcefully", guild_id, describe);
        bot.http()
            .create_message(head.text_channel_id())
            .content(&format!("{emoji} `(Bot was forcefully {describe})`."))
            .await?;
    }
    Ok(())
}
