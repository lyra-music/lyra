use std::fmt::Display;

use async_trait::async_trait;
use chrono::Utc;
use lyra_proc::{check, err, out};
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_mention::Mention;
use twilight_model::{
    application::interaction::application_command::InteractionChannel,
    channel::{Channel, ChannelType},
    gateway::payload::outgoing::UpdateVoiceState,
    guild::Permissions,
    id::{marker::ChannelMarker, Id},
};

use crate::bot::commands::{
    checks,
    errors::{AlreadyInVoiceError, ConnectionError, Error},
    models::{Context, LyraCommand},
};

enum JoinResponse {
    Joined(Voice),
    Moved { from: Voice, to: Voice },
}

impl Display for JoinResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinResponse::Joined(voice) => write!(f, "joined {}", voice),
            JoinResponse::Moved { from: old, to: new } => write!(f, "moved  {} -> {}", old, new),
        }
    }
}

struct Voice {
    id: Id<ChannelMarker>,
    kind: VoiceType,
}

enum VoiceType {
    Voice,
    Stage,
}

macro_rules! impl_into_voice {
    ($($ty:ty),+) => {
        $(
            impl From<$ty> for Voice {
                fn from(channel: $ty) -> Self {
                    let kind = match channel.kind {
                        ChannelType::GuildVoice => VoiceType::Voice,
                        ChannelType::GuildStageVoice => VoiceType::Stage,
                        _ => panic!("unexpected channel type: {:?}", channel.kind),
                    };
                    Voice { id: channel.id, kind }
                }
            }
        )+
    };
}

impl_into_voice!(Channel, InteractionChannel);

impl Display for Voice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            VoiceType::Voice => write!(f, "voice {}", self.id),
            VoiceType::Stage => write!(f, "stage {}", self.id),
        }
    }
}

async fn join(ctx: &Context, channel: Option<InteractionChannel>) -> anyhow::Result<JoinResponse> {
    let guild_id = ctx.guild_id_unchecked();
    let channel_id = match channel {
        Some(ref channel) => channel.id,
        None => match ctx.cache().voice_state(ctx.author_id(), guild_id) {
            Some(voice_state) => voice_state.channel_id(),
            None => {
                return Err(Error::UserNotInVoice.into());
            }
        },
    };

    let old_channel = match ctx.current_voice_state() {
        Some(voice_state) => {
            let old_channel_id = voice_state.channel_id();
            if old_channel_id == channel_id {
                return Err(Error::Connection {
                    channel_id,
                    source: ConnectionError::AlreadyInVoice(AlreadyInVoiceError::SameVoice),
                }
                .into());
            }

            checks::noone_else_in_voice(ctx, old_channel_id)?;
            Some(old_channel_id)
        }
        None => None,
    };

    let permission = Permissions::CONNECT;
    if (ctx.bot_permissions_for(channel_id)? & permission).is_empty() {
        return Err(Error::Connection {
            channel_id,
            source: ConnectionError::Forbidden,
        }
        .into());
    }

    // TODO: handle blacklists/whitelists

    ctx.bot()
        .sender()
        .command(&UpdateVoiceState::new(guild_id, channel_id, true, false))?;

    let new_voice = if let Some(channel) = channel {
        channel.into()
    } else {
        ctx.cache()
            .channel(channel_id)
            .ok_or(Error::Cache)?
            .clone()
            .into()
    };

    if let Voice {
        kind: VoiceType::Stage,
        ..
    } = new_voice
    {
        ctx.bot()
            .http()
            .update_current_user_voice_state(guild_id)
            .channel_id(channel_id)
            .request_to_speak_timestamp(&Utc::now().to_rfc3339())
            .await?;
    }

    let response = if let Some(old_channel) = old_channel {
        let old_channel = ctx
            .cache()
            .channel(old_channel)
            .ok_or(Error::Cache)?
            .clone();

        JoinResponse::Moved {
            from: old_channel.into(),
            to: new_voice,
        }
    } else {
        JoinResponse::Joined(new_voice)
    };

    tracing::info!("guild {guild_id} {response}");
    Ok(response)
}

#[derive(CreateCommand, CommandModel)]
#[command(name = "join", desc = "Joins a voice/stage channel")]
pub struct Join {
    #[command(
        desc = "Which channel? (if not given, your currently connected channel)",
        channel_types = "guild_voice guild_stage_voice"
    )]
    channel: Option<InteractionChannel>,
}

#[async_trait]
impl LyraCommand for Join {
    // TODO: Make the check macro an `ImplItemFn` attribute macro once async traits are stable.
    async fn callback(&self, ctx: Context) -> anyhow::Result<()> {
        check!(Guild);

        let mut txt;
        let stage_fmt = |txt: &mut String, voice: &Voice| {
            if matches!(voice.kind, VoiceType::Stage) {
                *txt = format!("🎭{txt}`");
            }
        };

        match join(&ctx, self.channel.clone()).await {
            Ok(response) => match response {
                JoinResponse::Joined(voice) => {
                    txt = format!("🖇️ {}", voice.id.mention());
                    stage_fmt(&mut txt, &voice);
                }
                JoinResponse::Moved { from, to } => {
                    txt = format!("️📎🖇️ ~~{}~~ ➜ __{}__", from.id.mention(), to.id.mention());
                    stage_fmt(&mut txt, &to);
                }
            },
            Err(e) => match e.downcast()? {
                Error::UserNotInVoice => {
                    err!("❌ Please specify a voice channel, or join one");
                }
                Error::Connection {
                    channel_id,
                    source: ConnectionError::AlreadyInVoice(AlreadyInVoiceError::SameVoice),
                } => {
                    err!(&format!("❕ Already connected to {}", channel_id.mention()));
                }
                Error::Connection {
                    channel_id,
                    source: ConnectionError::Forbidden,
                } => {
                    err!(&format!(
                        "⛔ Insufficient permissions to join {}",
                        channel_id.mention()
                    ));
                }
                other => return Err(other.into()),
            },
        };

        out!(&txt);
    }
}
