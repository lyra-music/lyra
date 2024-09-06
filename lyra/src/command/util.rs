use lavalink_rs::{error::LavalinkResult, player_context::PlayerContext};
use lyra_ext::unix_time;
use rand::{distributions::Alphanumeric, Rng};
use twilight_gateway::Event;
use twilight_model::{
    application::interaction::{InteractionData, InteractionType},
    channel::{
        message::component::{TextInput, TextInputStyle},
        ChannelType,
    },
    guild::PartialMember,
    id::{
        marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker},
        Id,
    },
    user::User,
    util::ImageHash,
};

use super::{
    check,
    macros::note_fol,
    model::{
        CommandDataAware, CtxKind, GuildCtx, GuildModalCtx, RespondViaMessage, RespondViaModal,
    },
    require::{self, InVoice},
};
use crate::{
    component::connection::auto_join,
    core::{
        model::{AuthorIdAware, BotStateAware, CacheAware, OwnedBotStateAware},
        r#const::{
            self,
            discord::{BASE_URL, CDN_URL},
        },
    },
    error::{
        command::{
            require::UnsuppressedError,
            util::{
                AutoJoinOrCheckInVoiceWithUserError, AutoJoinSuppressedError,
                HandleSuppressedAutoJoinError, PromptForConfirmationError,
            },
        },
        ConfirmationTimedOut, Suppressed as SuppressedError,
    },
    gateway::GuildIdAware,
    lavalink::DelegateMethods,
    LavalinkAware,
};

pub trait MessageLinkAware {
    fn id(&self) -> Id<MessageMarker>;
    fn channel_id(&self) -> Id<ChannelMarker>;
    fn guild_id(&self) -> Option<Id<GuildMarker>>;
    fn link(&self) -> String {
        let guild_id_str = self
            .guild_id()
            .map_or_else(|| String::from("@me"), |i| i.to_string());
        format!(
            "{}/channels/{}/{}/{}",
            BASE_URL,
            guild_id_str,
            self.channel_id(),
            self.id()
        )
    }
}

impl MessageLinkAware for twilight_model::channel::Message {
    fn id(&self) -> Id<MessageMarker> {
        self.id
    }

    fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel_id
    }

    fn guild_id(&self) -> Option<Id<GuildMarker>> {
        self.guild_id
    }
}

impl MessageLinkAware for twilight_cache_inmemory::model::CachedMessage {
    fn id(&self) -> Id<MessageMarker> {
        self.id()
    }

    fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel_id()
    }

    fn guild_id(&self) -> Option<Id<GuildMarker>> {
        self.guild_id()
    }
}

#[derive(Debug, Clone)]
pub struct MessageLinkComponent {
    id: Id<MessageMarker>,
    channel_id: Id<ChannelMarker>,
    guild_id: Option<Id<GuildMarker>>,
}

impl From<twilight_model::channel::Message> for MessageLinkComponent {
    fn from(value: twilight_model::channel::Message) -> Self {
        Self {
            id: value.id,
            channel_id: value.channel_id,
            guild_id: value.guild_id,
        }
    }
}

impl MessageLinkAware for MessageLinkComponent {
    fn id(&self) -> Id<MessageMarker> {
        self.id
    }

    fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel_id
    }

    fn guild_id(&self) -> Option<Id<GuildMarker>> {
        self.guild_id
    }
}

pub trait AvatarUrlAware {
    fn id(&self) -> Id<UserMarker>;
    fn avatar(&self) -> Option<ImageHash>;
    fn avatar_url(&self) -> Option<String> {
        let avatar = self.avatar()?;
        let ext = if avatar.is_animated() { "gif" } else { "png" };

        let formatted = format!("{}/avatars/{}/{}.{}", CDN_URL, self.id(), avatar, ext);
        Some(formatted)
    }
}

impl AvatarUrlAware for User {
    fn id(&self) -> Id<UserMarker> {
        self.id
    }
    fn avatar(&self) -> Option<ImageHash> {
        self.avatar
    }
}

pub trait GuildAvatarUrlAware {
    fn id(&self) -> Id<UserMarker>;
    fn avatar(&self) -> Option<ImageHash>;
    fn avatar_url(&self, guild_id: Id<GuildMarker>) -> Option<String> {
        let avatar = self.avatar()?;
        let ext = if avatar.is_animated() { "gif" } else { "png" };

        let formatted = format!(
            "{}/guilds/{}/users/{}/avatars/{}.{}",
            CDN_URL,
            guild_id,
            self.id(),
            avatar,
            ext,
        );
        Some(formatted)
    }
}

impl GuildAvatarUrlAware for PartialMember {
    fn id(&self) -> Id<UserMarker> {
        self.user
            .as_ref()
            .unwrap_or_else(|| panic!("user field is missing"))
            .id
    }
    fn avatar(&self) -> Option<ImageHash> {
        self.avatar
    }
}

pub trait DefaultAvatarUrlAware {
    fn id(&self) -> Id<UserMarker>;
    fn discriminator(&self) -> u16;
    fn default_avatar_url(&self) -> String {
        let discriminator = self.discriminator();
        if discriminator == 0 {
            return format!(
                "{}/embed/avatars/{}.png",
                CDN_URL,
                (self.id().get() >> 22) % 6
            );
        }
        format!("{}/embed/avatars/{}.png", CDN_URL, self.discriminator() % 5)
    }
}

impl DefaultAvatarUrlAware for User {
    fn id(&self) -> Id<UserMarker> {
        self.id
    }

    fn discriminator(&self) -> u16 {
        self.discriminator
    }
}

pub async fn auto_join_or_check_in_voice_with_user_and_check_not_suppressed(
    ctx: &mut GuildCtx<impl RespondViaMessage>,
) -> Result<(), AutoJoinOrCheckInVoiceWithUserError> {
    if let Ok(in_voice) = require::in_voice(ctx) {
        let in_voice = in_voice.and_unsuppressed()?;
        check::user_in(in_voice)?;
        return Ok(());
    }

    match auto_join(ctx).await {
        Err(e) => Err(e.unflatten_into_auto_join_attempt().into()),
        Ok(state) => {
            let Err(UnsuppressedError::Suppressed(suppressed)) = {
                // SAFETY: as `auto_join` was called and ran successfully,
                //         there must now be an active voice connection.
                unsafe { InVoice::new(state, ctx) }
            }
            .and_unsuppressed() else {
                return Ok(());
            };
            handle_suppressed_auto_join(suppressed, ctx).await?;
            Ok(())
        }
    }
}

async fn handle_suppressed_auto_join(
    error: SuppressedError,
    ctx: &GuildCtx<impl RespondViaMessage>,
) -> Result<(), HandleSuppressedAutoJoinError> {
    let bot_user_id = ctx.bot().user_id();
    match error {
        SuppressedError::Muted => Err(AutoJoinSuppressedError::Muted.into()),
        SuppressedError::NotSpeaker => {
            let bot = ctx.bot_owned();
            let wait_for_speaker =
                ctx.bot()
                    .standby()
                    .wait_for(ctx.guild_id(), move |e: &Event| {
                        let Event::VoiceStateUpdate(e) = e else {
                            return false;
                        };
                        e.user_id == bot_user_id
                            && e.channel_id.is_some_and(|id| {
                                bot.cache()
                                    .channel(id)
                                    .is_some_and(|c| matches!(c.kind, ChannelType::GuildStageVoice))
                            })
                            && !e.suppress
                    });

            let requested_to_speak = note_fol!(
                &format!(
                    "Requested to speak. **Accept the request in <t:{}:R> to continue.**",
                    unix_time().as_secs()
                        + u64::from(r#const::misc::WAIT_FOR_NOT_SUPPRESSED_TIMEOUT_SECS)
                ),
                ?ctx
            );
            let requested_to_speak_message = requested_to_speak.model().await?;
            let wait_for_speaker =
                tokio::time::timeout(r#const::misc::WAIT_FOR_BOT_EVENTS_TIMEOUT, wait_for_speaker);

            if wait_for_speaker.await.is_err() {
                return Err(AutoJoinSuppressedError::StillNotSpeaker {
                    last_followup_id: requested_to_speak_message.id,
                }
                .into());
            }
            Ok(())
        }
    }
}

pub async fn prompt_for_confirmation(
    mut ctx: GuildCtx<impl CommandDataAware + RespondViaModal>,
) -> Result<(GuildModalCtx, bool), PromptForConfirmationError> {
    let text_input = TextInput {
        custom_id: String::new(),
        label: String::from("This is a destructive command. Are you sure?"),
        max_length: None,
        min_length: None,
        required: Some(true),
        placeholder: Some(String::from(r#"Type "YES" (All Caps) to confirm..."#)),
        style: TextInputStyle::Short,
        value: None,
    };

    let modal_custom_id = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(100)
        .map(char::from)
        .collect::<String>();
    ctx.modal(
        modal_custom_id.clone(),
        ctx.command_name_full(),
        [text_input],
    )
    .await?;

    let author_id = ctx.author_id();
    let wait_for_modal_submit = ctx
        .bot()
        .standby()
        .wait_for(ctx.guild_id(), move |e: &Event| {
            let Event::InteractionCreate(ref i) = e else {
                return false;
            };
            let Some(InteractionData::ModalSubmit(ref m)) = i.data else {
                return false;
            };
            m.custom_id == modal_custom_id
                && matches!(i.kind, InteractionType::ModalSubmit)
                && i.author_id() == Some(author_id)
        });

    let wait_for_modal_submit = tokio::time::timeout(
        r#const::misc::DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT,
        wait_for_modal_submit,
    )
    .await;

    let ctx_and_confirmed = match wait_for_modal_submit {
        Ok(Ok(Event::InteractionCreate(interaction))) => {
            let ctx = ctx.into_modal_interaction(interaction);
            let confirmed = ctx.submit_data().components[0].components[0]
                .value
                .as_ref()
                .is_some_and(|s| s == "YES");
            (ctx, confirmed)
        }
        // SAFETY: the future has been filtered to only match modal submit interaction
        //         so this branch is unreachable
        Ok(Ok(_)) => unsafe { std::hint::unreachable_unchecked() },
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(ConfirmationTimedOut.into()),
    };

    Ok(ctx_and_confirmed)
}

pub async fn auto_new_player(ctx: &GuildCtx<impl CtxKind>) -> LavalinkResult<PlayerContext> {
    let guild_id = ctx.guild_id();
    let lavalink = ctx.lavalink();

    let player = match lavalink.get_player_context(guild_id) {
        Some(player) => player,
        None => lavalink.new_player(guild_id, ctx.channel_id()).await?,
    };

    Ok(player)
}
