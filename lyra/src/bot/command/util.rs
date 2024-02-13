use std::time::Duration;

use chrono::Utc;
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
    macros::{hid_fol, note_fol},
    model::{CommandDataAware, ModalCtx, RespondViaMessage, RespondViaModal},
    Ctx,
};
use crate::bot::{
    component::connection::auto_join,
    core::{
        model::{BotStateAware, CacheAware, OwnedBotStateAware},
        r#const::{
            discord::{BASE_URL, CDN_URL},
            misc::{DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT, WAIT_FOR_NOT_SUPPRESSED_TIMEOUT},
        },
    },
    error::{
        command::{
            check::NotSuppressedError,
            util::{
                AutoJoinOrCheckInVoiceWithUserError, AutoJoinSuppressedError, ConfirmationError,
                HandleSuppressedAutoJoinError, PromptForConfirmationError,
            },
        },
        Suppressed as SuppressedError,
    },
    gateway::ExpectedGuildIdAware,
};

pub trait MessageLinkAware {
    fn id(&self) -> Id<MessageMarker>;
    fn channel_id(&self) -> Id<ChannelMarker>;
    fn guild_id(&self) -> Option<Id<GuildMarker>>;
    fn link(&self) -> String {
        let guild_id_str = self
            .guild_id()
            .map_or_else(|| String::from("@me"), |g| g.to_string());
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

        format!("{}/avatars/{}/{}.{}", CDN_URL, self.id(), avatar, ext).into()
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

        format!(
            "{}/guilds/{}/users/{}/avatars/{}.{}",
            CDN_URL,
            guild_id,
            self.id(),
            avatar,
            ext,
        )
        .into()
    }
}

impl GuildAvatarUrlAware for PartialMember {
    fn id(&self) -> Id<UserMarker> {
        self.user.as_ref().expect("user must exist").id
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
    ctx: &mut Ctx<impl RespondViaMessage>,
) -> Result<(), AutoJoinOrCheckInVoiceWithUserError> {
    if let Some(voice_state) = ctx.current_voice_state() {
        check::user_in(voice_state.channel_id(), ctx)?;
        check::not_suppressed(ctx)?;
        return Ok(());
    }

    let Err(e) = auto_join(ctx).await else {
        let Err(NotSuppressedError::Suppressed(suppressed)) = check::not_suppressed(ctx) else {
            return Ok(());
        };
        handle_suppressed_auto_join(suppressed, ctx).await?;
        return Ok(());
    };

    Err(e.unflatten_into_auto_join_attempt())?
}

async fn handle_suppressed_auto_join(
    error: SuppressedError,
    ctx: &Ctx<impl RespondViaMessage>,
) -> Result<(), HandleSuppressedAutoJoinError> {
    let bot_user_id = ctx.bot().user_id();
    match error {
        SuppressedError::Muted => Err(AutoJoinSuppressedError::Muted)?,
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
                    Utc::now().timestamp() + i64::from(WAIT_FOR_NOT_SUPPRESSED_TIMEOUT)
                ),
                ?ctx
            );
            let requested_to_speak_message = requested_to_speak.model().await?;
            let wait_for_speaker = tokio::time::timeout(
                Duration::from_secs(WAIT_FOR_NOT_SUPPRESSED_TIMEOUT.into()),
                wait_for_speaker,
            );

            if wait_for_speaker.await.is_err() {
                Err(AutoJoinSuppressedError::StillNotSpeaker {
                    last_followup_id: requested_to_speak_message.id,
                })?;
            }
            Ok(())
        }
    }
}

pub async fn prompt_for_confirmation(
    mut ctx: Ctx<impl CommandDataAware + RespondViaModal>,
) -> Result<ModalCtx, PromptForConfirmationError> {
    let text_input = TextInput {
        custom_id: String::new(),
        label: "This is a destructive command. Are you sure?".into(),
        max_length: None,
        min_length: None,
        required: true.into(),
        placeholder: Some(r#"Type "YES" (All Caps) to confirm..."#.into()),
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
        Duration::from_secs(DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT.into()),
        wait_for_modal_submit,
    )
    .await;

    let modal_ctx = match wait_for_modal_submit {
        Ok(Ok(Event::InteractionCreate(interaction))) => {
            let ctx = ctx.into_modal_interaction(interaction);
            if ctx.submit_data().components[0].components[0]
                .value
                .as_ref()
                .is_some_and(|s| s == "YES")
            {
                Err(ConfirmationError::Cancelled)?;
            }
            ctx
        }
        Ok(Ok(_)) => unreachable!(),
        Ok(Err(e)) => Err(e)?,
        Err(_) => Err(ConfirmationError::TimedOut)?,
    };

    Ok(modal_ctx)
}
