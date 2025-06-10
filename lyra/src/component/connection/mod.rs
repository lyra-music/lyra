mod join;
mod leave;

pub use join::{Join, auto as auto_join};
pub use leave::Leave;
use lyra_ext::{iso8601_time, unix_time};

use std::sync::Arc;

use twilight_cache_inmemory::{InMemoryCache, model::CachedVoiceState};
use twilight_gateway::{Event, MessageSender};
use twilight_http::Client;
use twilight_mention::{
    Mention,
    timestamp::{Timestamp, TimestampStyle},
};
use twilight_model::{
    channel::message::MessageFlags,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker},
    },
};

use self::join::JoinedChannel;
use crate::{
    LavalinkAndGuildIdAware, LavalinkAware,
    command::require,
    component::connection::{
        join::JoinedChannelType,
        leave::{LeaveResponse, disconnect, disconnect_cleanup},
    },
    core::{
        r#const::{
            connection::{self as const_connection, INACTIVITY_TIMEOUT},
            exit_code::NOTICE,
        },
        model::{
            BotState, BotStateAware, CacheAware, HttpAware, OwnedBotStateAware, OwnedCacheAware,
        },
        traced,
    },
    error::{
        self,
        component::connection::{
            HandleVoiceStateUpdateError, MatchStateChannelIdError, StartInactivityTimeoutError,
        },
    },
    gateway::{GuildIdAware, SenderAware, voice},
    lavalink::{ConnectionHead, Lavalink},
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

impl<T> From<&T> for InactivityTimeoutContext
where
    T: OwnedBotStateAware + SenderAware + GuildIdAware + LavalinkAware,
{
    fn from(value: &T) -> Self {
        Self {
            inner: value.bot_owned(),
            sender: value.sender().clone(),
            guild_id: value.guild_id(),
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

async fn start_inactivity_timeout(
    ctx: InactivityTimeoutContext,
    channel_id: Id<ChannelMarker>,
) -> Result<(), StartInactivityTimeoutError> {
    let guild_id = ctx.guild_id;
    tracing::debug!(
        "guild {} started channel {} inactivity timeout",
        guild_id,
        channel_id
    );

    let cache = ctx.inner.cache_owned();
    let bot_user_id = ctx.inner.user_id();

    if tokio::time::timeout(const_connection::INACTIVITY_TIMEOUT, {
        ctx.inner.standby().wait_for(guild_id, move |e: &Event| {
            let Event::VoiceStateUpdate(voice_state) = e else {
                return false;
            };
            let vs_channel_id = voice_state.channel_id;

            (voice_state.user_id == bot_user_id && vs_channel_id != Some(channel_id)) // bot changed channel
                || users_in_voice(&cache, channel_id).is_some_and(|n| n >= 1) // bot not alone
        })
    })
    .await
    .is_ok()
    {
        tracing::debug!(
            "guild {} stopped channel {} inactivity timeout",
            guild_id,
            channel_id,
        );
        return Ok(());
    }

    // CORRECTNESS: as the bot later leaves the voice channel, it invokes a
    // voice state update event, so this is correct.
    if ctx.disable_vsu_handler().await.is_err() {
        tracing::debug!(
            "guild {} stopped channel {} inactivity timeout (unrecognised connection)",
            guild_id,
            channel_id
        );
        return Ok(());
    }

    let text_channel_id = ctx
        .lavalink()
        .handle_for(guild_id)
        .get_head()
        .await?
        .text_channel_id();
    disconnect_cleanup(&ctx).await?;
    disconnect(&ctx)?;

    let response = LeaveResponse(channel_id);

    tracing::info!("guild {} {} due to inactivity", guild_id, response);

    ctx.http()
        .create_message(text_channel_id)
        .content(&format!(
            "💤📎 ~~{}~~ `(Left due to inactivity)`.",
            channel_id.mention()
        ))
        .await?;

    Ok(())
}

#[tracing::instrument(skip_all, name = "connection")]
pub async fn handle_voice_state_update(
    ctx: &voice::Context,
    head: ConnectionHead,
) -> Result<(), HandleVoiceStateUpdateError> {
    let state = &ctx.inner;
    let maybe_old_state = ctx.old_voice_state();

    let guild_id = ctx.guild_id();

    tracing::debug!("handling voice state update");
    let (connected_channel_id, text_channel_id) = (head.channel_id(), head.text_channel_id());

    match maybe_old_state {
        Some(old_state) if state.user_id != ctx.bot().user_id() => {
            let old_channel_id = old_state.channel_id();
            if old_channel_id == connected_channel_id
                && state.channel_id.is_none_or(|id| id != old_channel_id)
                && users_in_voice(ctx, connected_channel_id).is_some_and(|n| n == 0)
            {
                if let Ok(player) = require::player(ctx) {
                    if !player.paused().await {
                        player.set_pause(true).await?;
                        ctx.http()
                            .create_message(text_channel_id)
                            .content("⚡▶ Paused `(Bot is not used by anyone)`.")
                            .flags(MessageFlags::SUPPRESS_NOTIFICATIONS)
                            .await?;
                    }
                }

                traced::tokio_spawn(start_inactivity_timeout(
                    InactivityTimeoutContext::from(ctx),
                    connected_channel_id,
                ));
            }
            return Ok(());
        }
        Some(old_state) if state.channel_id.is_none() => {
            disconnect_cleanup(ctx).await?;

            let old_channel_id = old_state.channel_id();
            let response = LeaveResponse(old_channel_id);

            tracing::warn!("guild {} {} forcefully", guild_id, response);
            ctx.bot()
                .http()
                .create_message(text_channel_id)
                .content(&format!(
                    "{}📎 ~~{}~~ `(Bot was forcefully disconnected)`.",
                    NOTICE,
                    old_channel_id.mention()
                ))
                .await?;
        }
        Some(old_state) => {
            match_state_channel_id(
                state.channel_id,
                old_state,
                guild_id,
                text_channel_id,
                state.mute,
                ctx,
            )
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
    mute: bool,
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

            let response = join::Response::new(Some(old_channel_id), joined, voice_is_empty, mute);

            if let Ok(player) = require::player(ctx) {
                player.update_voice_channel(voice_is_empty).await?;
            }
            let forcefully_moved_notice = if voice_is_empty {
                let duration = unix_time() + INACTIVITY_TIMEOUT;
                let timestamp =
                    Timestamp::new(duration.as_secs(), Some(TimestampStyle::RelativeTime));
                format!(
                    "\n`(Bot was forcefully moved to an empty voice channel, and automatically disconnecting if no one else joins in` {} `)`",
                    timestamp.mention()
                )
            } else {
                String::from("` (Bot was forcefully moved)`")
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
                    "{}{}📎🖇️ ~~{}~~ ➜ __{}__{}.",
                    NOTICE,
                    stage_emoji,
                    old_channel_id.mention(),
                    channel_id.mention(),
                    forcefully_moved_notice
                ))
                .await?;

            ctx.get_conn().set_channel(channel_id);

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
                    InactivityTimeoutContext::from(ctx),
                    channel_id,
                ));
            }
            Ok(())
        }
        Some(_) | None => Ok(()),
    }
}
