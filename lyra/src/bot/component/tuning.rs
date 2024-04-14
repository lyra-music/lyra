mod equaliser;
mod filter;
mod volume;

use std::num::NonZeroU16;

pub use volume::Volume;

use crate::bot::{
    core::model::{BotStateAware, HttpAware},
    gateway::{voice, ExpectedGuildIdAware},
    lavalink::{DelegateMethods, LavalinkAware},
};

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
