mod join;
mod leave;

pub use join::{auto as auto_join, Join};
pub use leave::Leave;
use lyra_ext::{iso8601_time, unix_time};

use std::sync::Arc;

use twilight_cache_inmemory::{model::CachedVoiceState, InMemoryCache};
use twilight_gateway::MessageSender;
use twilight_http::Client;
use twilight_mention::Mention;
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker},
    Id,
};

use self::join::JoinedChannel;
use crate::{
    command::require,
    component::connection::{
        join::JoinedChannelType,
        leave::{disconnect, pre_disconnect_cleanup, LeaveResponse},
    },
    core::{
        model::{BotState, BotStateAware, CacheAware, HttpAware, OwnedBotStateAware},
        r#const::{connection as const_connection, exit_code::NOTICE},
        traced,
    },
    error::{
        self,
        component::connection::{
            HandleVoiceStateUpdateError, MatchStateChannelIdError, StartInactivityTimeoutError,
        },
    },
    gateway::{voice, GuildIdAware, SenderAware},
    lavalink::Lavalink,
    LavalinkAndGuildIdAware, LavalinkAware,
};

pub fn users_in_voice(cx: &impl CacheAware, channel_id: Id<ChannelMarker>) -> Option<usize> {
    cx.cache()
        .voice_channel_states(channel_id)
        .map_or(Some(0), |voice_states| {
            let mut users = voice_states
                .map(|v| cx.cache().user(v.user_id()))
                .collect::<Option<Vec<_>>>()?;
            users.retain(|u| !u.bot);
            Some(users.len())
        })
}

struct InactivityTimeoutContext {
    inner: Arc<BotState>,
    sender: MessageSender,
    guild_id: Id<GuildMarker>,
}

impl InactivityTimeoutContext {
    fn new_via(cx: &(impl OwnedBotStateAware + SenderAware + GuildIdAware)) -> Self {
        Self {
            inner: cx.bot_owned(),
            sender: cx.sender().clone(),
            guild_id: cx.guild_id(),
        }
    }
}

impl SenderAware for InactivityTimeoutContext {
    fn sender(&self) -> &MessageSender {
        &self.sender
    }
}

impl CacheAware for InactivityTimeoutContext {
    fn cache(&self) -> &InMemoryCache {
        self.inner.cache()
    }
}

impl LavalinkAware for InactivityTimeoutContext {
    fn lavalink(&self) -> &Lavalink {
        self.inner.lavalink()
    }
}

impl HttpAware for InactivityTimeoutContext {
    fn http(&self) -> &Client {
        self.inner.http()
    }
}

impl GuildIdAware for InactivityTimeoutContext {
    fn guild_id(&self) -> Id<GuildMarker> {
        self.guild_id
    }
}

impl LavalinkAndGuildIdAware for InactivityTimeoutContext {}

async fn start_inactivity_timeout(
    ctx: InactivityTimeoutContext,
    channel_id: Id<ChannelMarker>,
    text_channel_id: Id<ChannelMarker>,
) -> Result<(), StartInactivityTimeoutError> {
    let guild_id = ctx.guild_id;
    tracing::debug!(
        "guild {} started channel {} inactivity timeout",
        guild_id,
        channel_id
    );

    for _ in 0..const_connection::INACTIVITY_TIMEOUT_POLL_N {
        tokio::time::sleep(const_connection::INACTIVITY_TIMEOUT_POLL_INTERVAL).await;
        if users_in_voice(&ctx, channel_id).is_some_and(|n| n >= 1) {
            return Ok(());
        }
    }

    let Some(connection) = ctx.get_connection() else {
        return Ok(());
    };
    connection.notify_change();
    pre_disconnect_cleanup(&ctx).await?;
    disconnect(&ctx)?;

    let response = LeaveResponse(channel_id);

    tracing::debug!("guild {} {} due to inactivity", guild_id, response);
    ctx.http()
        .create_message(text_channel_id)
        .content(&format!(
            "💤📎 ~~{}~~ `(Left due to inactivity)`",
            channel_id.mention()
        ))
        .await?;

    Ok(())
}

#[tracing::instrument(skip_all, name = "voice_state_update")]
pub async fn handle_voice_state_update(
    ctx: &voice::Context,
    connection_changed: bool,
) -> Result<(), HandleVoiceStateUpdateError> {
    let state = &ctx.inner;
    let maybe_old_state = ctx.old_voice_state();

    let guild_id = ctx.guild_id();

    tracing::trace!("handling voice state update");
    let (connected_channel_id, text_channel_id) = {
        let Some(connection) = ctx.get_connection() else {
            tracing::trace!("no active connection");
            return Ok(());
        };

        if connection_changed {
            tracing::trace!("received connection change notification");
            return Ok(());
        }
        tracing::trace!("no connection change notification");

        (connection.channel_id, connection.text_channel_id)
    };

    match maybe_old_state {
        Some(old_state) if state.user_id != ctx.bot().user_id() => {
            let old_channel_id = old_state.channel_id();
            if old_channel_id == connected_channel_id
                && state.channel_id != Some(old_channel_id)
                && users_in_voice(ctx, connected_channel_id).is_some_and(|n| n == 0)
            {
                if let Ok(player) = require::player(ctx) {
                    player.set_pause(true).await?;
                    ctx.http()
                        .create_message(text_channel_id)
                        .content("⚡▶ Paused `(Bot is not used by anyone)`")
                        .await?;
                };

                traced::tokio_spawn(start_inactivity_timeout(
                    InactivityTimeoutContext::new_via(ctx),
                    connected_channel_id,
                    text_channel_id,
                ));
            }
            return Ok(());
        }
        Some(old_state) if state.channel_id.is_none() => {
            pre_disconnect_cleanup(ctx).await?;

            let old_channel_id = old_state.channel_id();
            let response = LeaveResponse(old_channel_id);

            tracing::warn!("guild {} {} forcefully", guild_id, response);
            ctx.bot()
                .http()
                .create_message(text_channel_id)
                .content(&format!(
                    "{}📎 ~~{}~~ `(Bot was forcefully disconnected)`",
                    NOTICE,
                    old_channel_id.mention()
                ))
                .await?;
        }
        Some(old_state) => {
            match_state_channel_id(state.channel_id, old_state, guild_id, text_channel_id, ctx)
                .await?;
        }
        None => {}
    }

    Ok(())
}

async fn match_state_channel_id(
    channel_id: Option<Id<ChannelMarker>>,
    old_state: &CachedVoiceState,
    guild_id: Id<GuildMarker>,
    text_channel_id: Id<ChannelMarker>,
    ctx: &voice::Context,
) -> Result<(), MatchStateChannelIdError> {
    match channel_id {
        Some(channel_id) if channel_id != old_state.channel_id() => {
            let old_channel_id = old_state.channel_id();
            let joined = JoinedChannel::new(
                channel_id,
                ctx.cache().channel(channel_id).ok_or(error::Cache)?.kind,
            );

            let voice_is_empty = users_in_voice(ctx, channel_id).is_some_and(|n| n == 0);

            let response = join::Response::Moved {
                from: old_channel_id,
                to: joined,
                empty: voice_is_empty,
            };

            if let Ok(player) = require::player(ctx) {
                player.update_voice_channel(voice_is_empty).await?;
            }
            let forcefully_moved_notice = if voice_is_empty {
                format!(
                    "\n`(Bot was forcefully moved to an empty voice channel, and automatically disconnecting if no one else joins in` <t:{}:R> `)`",
                    unix_time().as_secs() + u64::from(const_connection::INACTIVITY_TIMEOUT_SECS)
                )
            } else {
                String::from("`(Bot was forcefully moved)`")
            };

            let stage_emoji = match joined.kind {
                JoinedChannelType::Stage => String::from("🎭"),
                JoinedChannelType::Voice => String::new(),
            };

            tracing::warn!("guild {} {} forcefully", guild_id, response);
            ctx.bot()
                .http()
                .create_message(text_channel_id)
                .content(&format!(
                    "{}{}📎🖇️ ~~{}~~ ➜ __{}__ {}",
                    NOTICE,
                    stage_emoji,
                    old_channel_id.mention(),
                    channel_id.mention(),
                    forcefully_moved_notice
                ))
                .await?;

            if let Some(mut connection) = ctx.get_connection_mut() {
                connection.channel_id = channel_id;
            }

            if matches!(joined.kind, JoinedChannelType::Stage) {
                ctx.bot()
                    .http()
                    .update_current_user_voice_state(guild_id)
                    .channel_id(channel_id)
                    .request_to_speak_timestamp(&iso8601_time())
                    .await?;
            }

            if voice_is_empty {
                traced::tokio_spawn(start_inactivity_timeout(
                    InactivityTimeoutContext::new_via(ctx),
                    channel_id,
                    text_channel_id,
                ));
            };
            Ok(())
        }
        Some(_) | None => Ok(()),
    }
}
