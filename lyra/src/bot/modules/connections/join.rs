use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::Mention;
use twilight_model::{
    application::interaction::application_command::InteractionChannel,
    channel::ChannelType,
    gateway::payload::outgoing::UpdateVoiceState,
    guild::Permissions,
    id::{marker::ChannelMarker, Id},
};

use super::InactivityTimeoutContext;
use crate::bot::{
    commands::{
        checks,
        errors::{AlreadyInVoiceError, ConnectionError, Error},
        macros::{bad, cant, hid, nope, note, out},
        models::{App, LyraCommand},
        Context,
    },
    lavalink::Lavalinkful,
    lib::{
        consts::{connections::INACTIVITY_TIMEOUT, exit_codes::NOTICE},
        models::Cacheful,
        traced,
    },
    modules::connections::{starts_inactivity_timeout, users_in_voice},
};

impl From<&Context<App>> for InactivityTimeoutContext {
    fn from(ctx: &Context<App>) -> Self {
        Self {
            http: ctx.bot().clone_http(),
            cache: ctx.bot().clone_cache(),
            sender: ctx.bot().sender().clone(),
            lavalink: ctx.bot().clone_lavalink(),
            guild_id: ctx.guild_id_unchecked(),
        }
    }
}

pub(super) enum JoinResponse {
    Joined {
        voice: JoinedChannel,
        empty: bool,
    },
    Moved {
        from: Id<ChannelMarker>,
        to: JoinedChannel,
        empty: bool,
    },
}

impl Display for JoinResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinResponse::Joined { voice, .. } => write!(f, "joined {}", voice),
            JoinResponse::Moved { from, to, .. } => write!(f, "moved  {} -> {}", from, to),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct JoinedChannel {
    id: Id<ChannelMarker>,
    pub(super) kind: JoinedChannelType,
}

#[derive(Clone, Copy)]
pub(super) enum JoinedChannelType {
    Voice,
    Stage,
}

impl JoinedChannel {
    pub(super) fn new(id: Id<ChannelMarker>, kind: ChannelType) -> Self {
        let kind = match kind {
            ChannelType::GuildVoice => JoinedChannelType::Voice,
            ChannelType::GuildStageVoice => JoinedChannelType::Stage,
            _ => unreachable!(),
        };
        Self { id, kind }
    }
}

impl Display for JoinedChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            JoinedChannelType::Voice => write!(f, "voice {}", self.id),
            JoinedChannelType::Stage => write!(f, "stage {}", self.id),
        }
    }
}

async fn join(ctx: &Context<App>, channel: Option<InteractionChannel>) -> Result<JoinResponse> {
    let guild_id = ctx.guild_id_unchecked();
    let (channel_id, channel_type, channel_parent_id) = channel.as_ref().map_or_else(
        || {
            ctx.cache()
                .channel(ctx.channel_id())
                .filter(|maybe_voice| {
                    matches!(
                        maybe_voice.kind,
                        ChannelType::GuildVoice | ChannelType::GuildStageVoice
                    )
                })
                .map_or_else(
                    || {
                        ctx.cache()
                            .voice_state(ctx.author_id(), guild_id)
                            .ok_or(Error::UserNotInVoice)
                            .and_then(|voice_state| {
                                let channel_id = voice_state.channel_id();
                                let voice = ctx.cache().channel(channel_id).ok_or(Error::Cache)?;
                                let channel_type = voice.kind;
                                let channel_parent_id = voice.parent_id;

                                Ok((channel_id, channel_type, channel_parent_id))
                            })
                    },
                    |voice| Ok((voice.id, voice.kind, voice.parent_id)),
                )
        },
        |voice| Ok((voice.id, voice.kind, voice.parent_id)),
    )?;

    let old_channel_id = ctx
        .current_voice_state()
        .map(|voice_state| {
            let old_channel_id = voice_state.channel_id();
            if old_channel_id == channel_id {
                return Err(Error::Connection {
                    channel_id,
                    source: ConnectionError::AlreadyInVoice(AlreadyInVoiceError::SameVoice),
                });
            }

            checks::noone_else_in_voice(ctx, old_channel_id)?;

            Ok(old_channel_id)
        })
        .transpose()?;

    if !ctx.bot_permissions().contains(Permissions::CONNECT) {
        return Err(Error::Connection {
            channel_id,
            source: ConnectionError::Forbidden,
        }
        .into());
    }

    checks::user_allowed_to_use(ctx, channel_id, channel_parent_id).await?;

    ctx.lavalink().dispatch_connection_change(guild_id);
    ctx.bot()
        .sender()
        .command(&UpdateVoiceState::new(guild_id, channel_id, true, false))?;

    let users_in_voice = users_in_voice(ctx, channel_id)?;
    let voice_is_empty = users_in_voice == 0;

    let joined = JoinedChannel::new(channel_id, channel_type);

    if let JoinedChannelType::Stage = joined.kind {
        ctx.bot()
            .http()
            .update_current_user_voice_state(guild_id)
            .channel_id(channel_id)
            .request_to_speak_timestamp(&Utc::now().to_rfc3339())
            .await?;
    }

    let response = match old_channel_id {
        Some(old_channel_id) => {
            ctx.lavalink()
                .update_connected_channel(guild_id, channel_id)
                .await;
            JoinResponse::Moved {
                from: old_channel_id,
                to: joined,
                empty: voice_is_empty,
            }
        }
        None => {
            ctx.lavalink()
                .new_connection(guild_id, channel_id, ctx.channel_id());
            JoinResponse::Joined {
                voice: joined,
                empty: voice_is_empty,
            }
        }
    };

    tracing::info!("guild {guild_id} {response}");
    Ok(response)
}

#[derive(CreateCommand, CommandModel)]
#[command(
    name = "join",
    desc = "Joins a voice/stage channel",
    dm_permission = false
)]
pub struct Join {
    #[command(
        desc = "Which channel? (if not given, your currently connected channel)",
        channel_types = "guild_voice guild_stage_voice"
    )]
    channel: Option<InteractionChannel>,
}

#[async_trait]
impl LyraCommand for Join {
    async fn execute(self, ctx: Context<App>) -> Result<()> {
        let stage_fmt = |txt: String, instance: &JoinedChannel| match instance.kind {
            JoinedChannelType::Stage => format!("🎭{txt}"),
            _ => txt,
        };

        let empty_voice_notice_txt = &format!(
            "{} Joined an empty voice channel. The bot will automatically disconnects if no one else joins in <t:{}:R>",
            NOTICE, Utc::now().timestamp() + INACTIVITY_TIMEOUT as i64
        );
        let empty_voice_notice = ctx.followup_ephem(empty_voice_notice_txt);
        let text_channel_id = ctx.channel_id();

        match join(&ctx, self.channel).await {
            Ok(JoinResponse::Joined { voice, empty }) => {
                out!(
                    stage_fmt(format!("🖇️ {}", voice.id.mention()), &voice),
                    ctx,
                    !
                );

                if empty {
                    empty_voice_notice.await?;
                    traced::tokio_spawn(starts_inactivity_timeout(
                        (&ctx).into(),
                        voice.id,
                        text_channel_id,
                    ));
                }
                Ok(())
            }
            Ok(JoinResponse::Moved { from, to, empty }) => {
                out!(
                    stage_fmt(
                        format!("️📎🖇️ ~~{}~~ ➜ __{}__", from.mention(), to.id.mention()),
                        &to,
                    ),
                    ctx,
                    !
                );

                if empty {
                    empty_voice_notice.await?;
                    traced::tokio_spawn(starts_inactivity_timeout(
                        (&ctx).into(),
                        to.id,
                        text_channel_id,
                    ));
                }
                Ok(())
            }
            Err(e) => match e.downcast()? {
                Error::UserNotInVoice => {
                    bad!("Please specify a voice channel, join one, or use the command in a voice channel's chat.", ctx);
                }
                Error::UserNotAllowed => {
                    nope!("You are not allowed to use that channel.", ctx);
                }
                Error::Connection {
                    channel_id,
                    source: ConnectionError::AlreadyInVoice(AlreadyInVoiceError::SameVoice),
                } => {
                    note!(
                        format!("Already connected to {}.", channel_id.mention()),
                        ctx
                    );
                }
                Error::Connection {
                    channel_id,
                    source: ConnectionError::Forbidden,
                } => {
                    cant!(
                        format!("Insufficient permissions to join {}.", channel_id.mention()),
                        ctx
                    );
                }
                other => Err(other.into()),
            },
        }
    }
}
