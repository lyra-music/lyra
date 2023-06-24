mod join;
mod leave;

pub use join::Join;
pub use leave::Leave;
use twilight_gateway::MessageSender;
use twilight_http::Client;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use chrono::Utc;
use itertools::Itertools;
use twilight_cache_inmemory::InMemoryCache;
use twilight_mention::Mention;
use twilight_model::id::{
    marker::{ChannelMarker, GuildMarker},
    Id,
};

use self::join::JoinedChannel;
use crate::bot::{
    commands::errors::Error,
    lavalink::{Lavalink, Lavalinkful},
    lib::{
        consts::{
            connections::{
                INACTIVITY_TIMEOUT, INACTIVITY_TIMEOUT_POLL_INTERVAL, INACTIVITY_TIMEOUT_POLL_N,
            },
            exit_codes::NOTICE,
        },
        models::Cacheful,
        traced,
    },
    modules::connections::{
        join::{JoinResponse, JoinedChannelType},
        leave::{
            destroy_resources, disconnect, DestroyResourcesContext, DisconnectContext,
            LeaveResponse,
        },
    },
    voice::Context,
};

fn users_in_voice(ctx: &impl Cacheful, channel_id: Id<ChannelMarker>) -> Result<usize> {
    ctx.cache()
        .voice_channel_states(channel_id)
        .map_or(Ok(0), |voice_states| {
            let voice_states_count = voice_states
                .map(|v| ctx.cache().user(v.user_id()).ok_or(Error::Cache))
                .filter_ok(|u| !u.bot)
                .collect::<Result<Vec<_>, _>>()?
                .len();
            Ok(voice_states_count)
        })
}

struct InactivityTimeoutContext {
    http: Arc<Client>,
    cache: Arc<InMemoryCache>,
    sender: MessageSender,
    lavalink: Arc<Lavalink>,
    guild_id: Id<GuildMarker>,
}

impl From<&Context<'_>> for InactivityTimeoutContext {
    fn from(ctx: &Context<'_>) -> Self {
        Self {
            http: ctx.bot().clone_http(),
            cache: ctx.bot().clone_cache(),
            sender: ctx.bot().sender().clone(),
            lavalink: ctx.bot().clone_lavalink(),
            guild_id: ctx.guild_id(),
        }
    }
}

impl<'a> From<&'a InactivityTimeoutContext> for DestroyResourcesContext<'a> {
    fn from(ctx: &'a InactivityTimeoutContext) -> Self {
        let lavalink = &ctx.lavalink;
        let guild_id = ctx.guild_id;
        Self::new(lavalink, guild_id)
    }
}

impl<'a> From<&'a InactivityTimeoutContext> for DisconnectContext<'a> {
    fn from(ctx: &'a InactivityTimeoutContext) -> Self {
        let sender = &ctx.sender;
        let guild_id = ctx.guild_id;
        Self::new(sender, guild_id)
    }
}

impl Cacheful for InactivityTimeoutContext {
    fn cache(&self) -> &InMemoryCache {
        &self.cache
    }
}

async fn starts_inactivity_timeout(
    ctx: InactivityTimeoutContext,
    channel_id: Id<ChannelMarker>,
    text_channel_id: Id<ChannelMarker>,
) -> Result<()> {
    let guild_id = ctx.guild_id;
    tracing::debug!(
        "guild {} started channel {} inactivity timeout",
        guild_id,
        channel_id
    );

    for _ in 0..INACTIVITY_TIMEOUT_POLL_N {
        tokio::time::sleep(Duration::from_secs(INACTIVITY_TIMEOUT_POLL_INTERVAL.into())).await;
        if users_in_voice(&ctx, channel_id)? >= 1 {
            return Ok(());
        }
    }

    ctx.lavalink.dispatch_connection_change(guild_id);
    destroy_resources((&ctx).into()).await?;
    disconnect((&ctx).into())?;

    let response = LeaveResponse(channel_id);

    tracing::info!("guild {} {} due to inactivity", guild_id, response);
    ctx.http
        .create_message(text_channel_id)
        .content(&format!(
            "💤📎 ~~{}~~ `(Left due to inactivity)`",
            channel_id.mention()
        ))?
        .await?;

    Ok(())
}

pub async fn handle_voice_state_update(ctx: &Context<'_>) -> Result<()> {
    let state = ctx.inner;
    let maybe_old_state = ctx.old_voice_state();

    let guild_id = ctx.guild_id();

    let (connected_channel_id, text_channel_id, dispatched_connection_change) = {
        let connection_info = ctx.lavalink().connections().get(&guild_id);
        let Some(connection_info) = connection_info else {return Ok(());};

        (
            connection_info.channel_id().await,
            connection_info.text_channel_id().await,
            connection_info.dispatched_connection_change(),
        )
    };

    if dispatched_connection_change {
        ctx.lavalink().acknowledge_connection_change(guild_id);
        return Ok(());
    }

    match maybe_old_state {
        Some(old_state) if state.user_id != ctx.bot().user_id() => {
            let old_channel_id = old_state.channel_id();
            if old_channel_id == connected_channel_id
                && state.channel_id != Some(old_channel_id)
                && users_in_voice(ctx, connected_channel_id)? == 0
            {
                traced::tokio_spawn(starts_inactivity_timeout(
                    ctx.into(),
                    connected_channel_id,
                    text_channel_id,
                ));
            }
            return Ok(());
        }
        Some(old_state) if state.channel_id.is_none() => {
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
                ))?
                .await?;
        }
        Some(old_state) => match state.channel_id {
            Some(channel_id) if channel_id != old_state.channel_id() => {
                let old_channel_id = old_state.channel_id();
                let channel = ctx.cache().channel(channel_id).ok_or(Error::Cache)?;
                let joined = JoinedChannel::new(channel_id, channel.kind);

                let users_in_voice = users_in_voice(ctx, channel_id)?;
                let voice_is_empty = users_in_voice == 0;

                let response = JoinResponse::Moved {
                    from: old_channel_id,
                    to: joined,
                    empty: voice_is_empty,
                };

                let forcefully_moved_notice = match voice_is_empty {
                        true => format!(
                            "\n`(Bot was forcefully moved to an empty voice channel, and automatically disconnecting if no one else joins in` <t:{}:R> `)`",
                            Utc::now().timestamp() + INACTIVITY_TIMEOUT as i64
                        ),
                        false => "`(Bot was forcefully moved)`".into()
                    };

                let stage_emoji = match joined.kind {
                    JoinedChannelType::Stage => "🎭".into(),
                    _ => String::new(),
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
                    ))?
                    .await?;

                if let JoinedChannelType::Stage = joined.kind {
                    ctx.bot()
                        .http()
                        .update_current_user_voice_state(guild_id)
                        .channel_id(channel_id)
                        .request_to_speak_timestamp(&Utc::now().to_rfc3339())
                        .await?;
                }

                if voice_is_empty {
                    traced::tokio_spawn(starts_inactivity_timeout(
                        ctx.into(),
                        channel_id,
                        text_channel_id,
                    ));
                }
            }
            Some(_) | None => {}
        },
        None => {}
    }

    Ok(())
}
