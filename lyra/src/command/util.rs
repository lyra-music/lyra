use std::borrow::Cow;

use lavalink_rs::{error::LavalinkResult, player_context::PlayerContext};
use lyra_ext::unix_time;
use rand::{Rng, distr::Alphanumeric};
use twilight_gateway::Event;
use twilight_mention::{
    Mention,
    timestamp::{Timestamp, TimestampStyle},
};
use twilight_model::{
    application::interaction::{InteractionData, InteractionType},
    channel::{
        ChannelType,
        message::component::{TextInput, TextInputStyle},
    },
    guild::PartialMember,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker},
    },
};

use super::{
    check,
    model::{
        AppCtxKind, AppCtxMarker, CtxKind, FollowupCtxKind, GuildCtx, GuildModalCtx,
        RespondViaMessage,
    },
    require::{self, InVoice},
};
use crate::{
    LavalinkAware,
    component::connection::auto_join,
    core::{
        r#const::{
            self,
            discord::{BASE_URL, CDN_URL},
        },
        model::{
            AvatarAware, BotStateAware, CacheAware, DiscriminatorAware, GuildAvatarAware,
            OwnedBotStateAware, UserGlobalNameAware, UserIdAware, UserNickAware, UsernameAware,
            response::{followup::Followup, initial::modal::RespondWithModal},
        },
    },
    error::{
        ConfirmationTimedOut, Suppressed as SuppressedError,
        command::{
            require::UnsuppressedError,
            util::{
                AutoJoinOrCheckInVoiceWithUserError, AutoJoinSuppressedError,
                HandleSuppressedAutoJoinError, PromptForConfirmationError,
            },
        },
    },
    gateway::{GuildIdAware, OptionallyGuildIdAware},
    lavalink::DelegateMethods,
};

pub trait MessageLinkAware: OptionallyGuildIdAware {
    fn id(&self) -> Id<MessageMarker>;
    fn channel_id(&self) -> Id<ChannelMarker>;
    fn link(&self) -> String {
        let guild_id_str = self
            .get_guild_id()
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

impl OptionallyGuildIdAware for twilight_model::channel::Message {
    fn get_guild_id(&self) -> Option<Id<GuildMarker>> {
        self.guild_id
    }
}

impl MessageLinkAware for twilight_model::channel::Message {
    fn id(&self) -> Id<MessageMarker> {
        self.id
    }

    fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel_id
    }
}

impl OptionallyGuildIdAware for twilight_cache_inmemory::model::CachedMessage {
    fn get_guild_id(&self) -> Option<Id<GuildMarker>> {
        self.guild_id()
    }
}

impl MessageLinkAware for twilight_cache_inmemory::model::CachedMessage {
    fn id(&self) -> Id<MessageMarker> {
        self.id()
    }

    fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel_id()
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

impl OptionallyGuildIdAware for MessageLinkComponent {
    fn get_guild_id(&self) -> Option<Id<GuildMarker>> {
        self.guild_id
    }
}

impl MessageLinkAware for MessageLinkComponent {
    fn id(&self) -> Id<MessageMarker> {
        self.id
    }

    fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel_id
    }
}

pub trait AvatarUrlAware: AvatarAware + UserIdAware {
    fn avatar_url(&self) -> Option<String> {
        let avatar = self.avatar()?;
        let ext = if avatar.is_animated() { "gif" } else { "png" };

        let formatted = format!("{}/avatars/{}/{}.{}", CDN_URL, self.user_id(), avatar, ext);
        Some(formatted)
    }
}

impl<T: AvatarAware + UserIdAware> AvatarUrlAware for T {}

pub trait GuildAvatarUrlAware: UserIdAware + GuildAvatarAware {
    fn guild_avatar_url(&self, guild_id: Id<GuildMarker>) -> Option<String> {
        let avatar = self.guild_avatar()?;
        let ext = if avatar.is_animated() { "gif" } else { "png" };

        let formatted = format!(
            "{}/guilds/{}/users/{}/avatars/{}.{}",
            CDN_URL,
            guild_id,
            self.user_id(),
            avatar,
            ext,
        );
        Some(formatted)
    }
}

impl<T: UserIdAware + GuildAvatarAware> GuildAvatarUrlAware for T {}

impl UserIdAware for PartialMember {
    fn user_id(&self) -> Id<UserMarker> {
        self.user.as_ref().expect("user field must exist").id
    }
}

pub trait DefaultAvatarUrlAware: UserIdAware + DiscriminatorAware {
    fn default_avatar_url(&self) -> String {
        let discriminator = self.discriminator();
        if discriminator == 0 {
            return format!(
                "{}/embed/avatars/{}.png",
                CDN_URL,
                (self.user_id().get() >> 22) % 6
            );
        }
        format!("{}/embed/avatars/{}.png", CDN_URL, self.discriminator() % 5)
    }
}

impl<T: UserIdAware + DiscriminatorAware> DefaultAvatarUrlAware for T {}

pub trait DisplayAvatarUrlAware: AvatarUrlAware + DefaultAvatarUrlAware {
    fn display_avatar_url(&self) -> String {
        self.avatar_url()
            .unwrap_or_else(|| self.default_avatar_url())
    }
}

impl<T: AvatarUrlAware + DefaultAvatarUrlAware> DisplayAvatarUrlAware for T {}

pub trait GuildIdAndDisplayAvatarUrlAware:
    GuildAvatarUrlAware + DisplayAvatarUrlAware + GuildIdAware
{
    fn guild_display_avatar_url(&self) -> String {
        self.guild_avatar_url(self.guild_id())
            .unwrap_or_else(|| self.display_avatar_url())
    }
}

impl<T> GuildIdAndDisplayAvatarUrlAware for T where
    T: GuildAvatarUrlAware + DisplayAvatarUrlAware + GuildIdAware
{
}

pub trait GuildIdAndDisplayNameAware: UserNickAware + DisplayNameAware + GuildIdAware {
    fn guild_display_name(&self) -> &str {
        self.nick()
            .unwrap_or_else(|| DisplayNameAware::display_name(self))
    }
}

impl<T> GuildIdAndDisplayNameAware for T where T: UserNickAware + DisplayNameAware + GuildIdAware {}

pub trait DisplayNameAware: UsernameAware + UserGlobalNameAware {
    fn display_name(&self) -> &str {
        self.user_global_name().unwrap_or_else(|| self.username())
    }
}

impl<T: UsernameAware + UserGlobalNameAware> DisplayNameAware for T {}

pub fn controller_fmt<'a>(
    ctx: &impl UserIdAware,
    via_controller: bool,
    string: &'a str,
) -> Cow<'a, str> {
    if via_controller {
        return format!("{} {}", ctx.user_id().mention(), string).into();
    }
    string.into()
}

pub async fn auto_join_or_check_in_voice_with_user_and_check_not_suppressed(
    ctx: &mut GuildCtx<impl RespondViaMessage + FollowupCtxKind>,
) -> Result<(), AutoJoinOrCheckInVoiceWithUserError> {
    if let Ok(in_voice) = require::in_voice(ctx) {
        let in_voice = in_voice.and_unsuppressed()?;
        check::user_in(in_voice)?;
        return Ok(());
    }

    match auto_join(ctx).await {
        Err(e) => Err(e.unflatten_into_auto_join_attempt().into()),
        Ok(state) => {
            let in_voice = InVoice::new(state, ctx);
            let Err(UnsuppressedError::Suppressed(suppressed)) = in_voice.and_unsuppressed() else {
                return Ok(());
            };
            handle_suppressed_auto_join(suppressed, ctx).await?;
            Ok(())
        }
    }
}

async fn handle_suppressed_auto_join(
    error: SuppressedError,
    ctx: &GuildCtx<impl RespondViaMessage + FollowupCtxKind>,
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

            let duration = unix_time() + r#const::misc::WAIT_FOR_NOT_SUPPRESSED_TIMEOUT;
            let timestamp = Timestamp::new(duration.as_secs(), Some(TimestampStyle::RelativeTime));
            let requested_to_speak = ctx
                .notef(format!(
                    "Requested to speak. **Accept the request in {} to continue.**",
                    timestamp.mention()
                ))
                .await?;
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
    mut ctx: GuildCtx<AppCtxMarker<impl AppCtxKind>>,
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

    let modal_custom_id = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(100)
        .map(char::from)
        .collect::<String>();
    ctx.modal(
        modal_custom_id.clone(),
        ctx.command_name_full(),
        [text_input.into()],
    )
    .await?;

    let author_id = ctx.user_id();
    let wait_for_modal_submit = ctx
        .bot()
        .standby()
        .wait_for(ctx.guild_id(), move |e: &Event| {
            let Event::InteractionCreate(i) = e else {
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
        Ok(Ok(_)) => panic!("event type not interaction create"),
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
