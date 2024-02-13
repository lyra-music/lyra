use std::{marker::PhantomData, sync::Arc};

use sqlx::{Pool, Postgres};
use twilight_cache_inmemory::{
    model::{CachedMember, CachedVoiceState},
    InMemoryCache, Reference,
};
use twilight_gateway::{Latency, MessageSender};
use twilight_http::Client as HttpClient;
use twilight_model::{
    application::{
        command::CommandOptionChoice,
        interaction::{
            application_command::{
                CommandData, CommandDataOption, CommandInteractionDataResolved, CommandOptionValue,
            },
            modal::ModalInteractionData,
            InteractionData,
        },
    },
    channel::{
        message::{AllowedMentions, Component as MessageComponent, Embed, MessageFlags},
        Channel, Message,
    },
    gateway::payload::incoming::InteractionCreate,
    guild::{PartialMember, Permissions},
    http::interaction::InteractionResponseData,
    id::{
        marker::{
            ChannelMarker, CommandMarker, GenericMarker, GuildMarker, MessageMarker, UserMarker,
        },
        Id,
    },
    user::User,
};
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::bot::{
    core::model::{
        BotState, BotStateAware, CacheAware, HttpAware, InteractionInterface, MessageResponse,
        OwnedBotState, OwnedBotStateAware,
    },
    error::{
        command::{AutocompleteResult, FollowupError, RespondError, Result as CommandResult},
        core::DeserializeBodyFromHttpError,
    },
    gateway::{ExpectedGuildIdAware, GuildIdAware, SenderAware},
    lavalink,
};

type CurrentVoiceStateResult<'a> =
    Option<Reference<'a, (Id<GuildMarker>, Id<UserMarker>), CachedVoiceState>>;

type RespondResult<T> = Result<T, RespondError>;
type MessageRespondResult = RespondResult<MessageResponse>;
type UnitRespondResult = RespondResult<()>;
type MessageFollowupResult = Result<MessageResponse, FollowupError>;

pub trait CtxKind {}

pub struct AppCommand<T: AppCtxKind>(PhantomData<fn(T) -> T>);
impl<T: AppCtxKind> CtxKind for AppCommand<T> {}

pub trait AppCtxKind {}

pub struct SlashAppMarker;
impl AppCtxKind for SlashAppMarker {}
pub struct UserAppMarker;
impl AppCtxKind for UserAppMarker {}
pub struct MessageAppMarker;
impl AppCtxKind for MessageAppMarker {}

pub type SlashCommand = AppCommand<SlashAppMarker>;
// pub type UserCommand = AppCommand<UserAppMarker>;
pub type MessageCommand = AppCommand<MessageAppMarker>;

pub struct ModalMarker;
impl CtxKind for ModalMarker {}
pub struct ComponentMarker;
impl CtxKind for ComponentMarker {}
pub struct AutocompleteMarker;
impl CtxKind for AutocompleteMarker {}

pub type ModalCtx = Ctx<ModalMarker>;
pub type ComponentCtx = Ctx<ComponentMarker>;
pub type AutocompleteCtx = Ctx<AutocompleteMarker>;

pub trait CommandDataAware: CtxKind {}
impl<T: AppCtxKind> CommandDataAware for AppCommand<T> {}
impl CommandDataAware for AutocompleteMarker {}

pub trait RespondViaMessage: CtxKind {}
impl<T: AppCtxKind> RespondViaMessage for AppCommand<T> {}
impl RespondViaMessage for ModalMarker {}
impl RespondViaMessage for ComponentMarker {}

pub trait RespondViaModal: CtxKind {}
impl<T: AppCtxKind> RespondViaModal for AppCommand<T> {}
impl RespondViaModal for ComponentMarker {}

pub trait TargetIdAware: AppCtxKind {}
impl TargetIdAware for UserAppMarker {}
impl TargetIdAware for MessageAppMarker {}

pub struct Ctx<T: CtxKind> {
    inner: Box<InteractionCreate>,
    bot: OwnedBotState,
    latency: Latency,
    sender: MessageSender,
    data: Option<PartialInteractionData>,
    acknowledged: bool,
    kind: PhantomData<fn(T) -> T>,
}

impl<T: CtxKind> Ctx<T> {
    pub fn into_modal_interaction(self, inner: Box<InteractionCreate>) -> Ctx<ModalMarker> {
        Ctx {
            inner,
            bot: self.bot,
            latency: self.latency,
            sender: self.sender,
            data: None,
            acknowledged: false,
            kind: PhantomData::<fn(ModalMarker) -> ModalMarker>,
        }
    }

    pub const fn latency(&self) -> &Latency {
        &self.latency
    }

    pub const fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }

    pub fn db(&self) -> &Pool<Postgres> {
        self.bot.db()
    }

    pub fn bot_member(&self) -> Reference<'_, (Id<GuildMarker>, Id<UserMarker>), CachedMember> {
        self.cache()
            .member(self.guild_id(), self.bot().user_id())
            .expect("bot's member object must exist")
    }

    #[inline]
    pub fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel().id
    }

    pub fn channel(&self) -> &Channel {
        self.inner
            .channel
            .as_ref()
            .expect("`self.inner.channel` must not be `None`")
    }

    pub fn author(&self) -> &User {
        self.inner
            .author()
            .expect("`self.inner.author()` must not be `None`")
    }

    pub fn member(&self) -> &PartialMember {
        self.inner
            .member
            .as_ref()
            .expect("`self.inner.member` must not be `None`")
    }

    #[inline]
    pub fn author_id(&self) -> Id<UserMarker> {
        self.author().id
    }

    pub fn interaction(&self) -> &InteractionCreate {
        self.inner.as_ref()
    }

    pub fn interaction_token(&self) -> &str {
        &self.inner.token
    }

    fn interaction_data(&self) -> &PartialInteractionData {
        self.data.as_ref().expect("`self.data` must exist")
    }

    pub async fn interface(&self) -> Result<InteractionInterface, DeserializeBodyFromHttpError> {
        Ok(self.bot.interaction().await?.interfaces(&self.inner))
    }

    pub fn bot_permissions(&self) -> Permissions {
        self.inner
            .app_permissions
            .expect("this interaction must be executed in guilds")
    }

    pub fn bot_permissions_for(&self, channel_id: Id<ChannelMarker>) -> Permissions {
        if channel_id == self.channel_id() {
            return self.bot_permissions();
        }

        let guild_id = self.guild_id();
        let user_id = self.bot().user_id();
        let everyone_role = self
            .cache()
            .role(guild_id.cast())
            .expect("`@everyone` role must exist");
        let member_roles = self
            .bot_member()
            .roles()
            .iter()
            .filter(|&&r| r != everyone_role.id)
            .filter_map(|&r| {
                let role = self.cache().role(r)?;

                Some((r, role.permissions))
            })
            .collect::<Box<_>>();

        let channel = self
            .cache()
            .channel(channel_id)
            .expect("channel must exist");
        let channel_overwrites = channel
            .permission_overwrites
            .as_ref()
            .expect("permission overrwrites must exist");

        twilight_util::permission_calculator::PermissionCalculator::new(
            guild_id,
            user_id,
            everyone_role.permissions,
            &member_roles,
        )
        .in_channel(channel.kind, channel_overwrites)
    }

    pub fn current_voice_state(&self) -> CurrentVoiceStateResult {
        let user = self.bot().user_id();
        self.cache().voice_state(user, self.get_guild_id()?)
    }
}

impl<T: CtxKind> BotStateAware for Ctx<T> {
    fn bot(&self) -> &BotState {
        &self.bot
    }
}

impl<T: CtxKind> OwnedBotStateAware for Ctx<T> {
    fn bot_owned(&self) -> Arc<BotState> {
        self.bot.clone()
    }
}

impl<T: CtxKind> SenderAware for Ctx<T> {
    fn sender(&self) -> &MessageSender {
        &self.sender
    }
}

impl<T: CtxKind> CacheAware for Ctx<T> {
    fn cache(&self) -> &InMemoryCache {
        self.bot.cache()
    }
}

impl<T: CtxKind> HttpAware for Ctx<T> {
    fn http(&self) -> &HttpClient {
        self.bot.http()
    }
}

impl<T: CtxKind> lavalink::ClientAware for Ctx<T> {
    fn lavalink(&self) -> &lavalink::Lavalink {
        self.bot.lavalink()
    }
}

impl<T: CtxKind> GuildIdAware for Ctx<T> {
    fn get_guild_id(&self) -> Option<Id<GuildMarker>> {
        self.inner.guild_id
    }
}

impl<T: CtxKind> ExpectedGuildIdAware for Ctx<T> {
    fn guild_id(&self) -> Id<GuildMarker> {
        self.get_guild_id()
            .expect("this interaction must be executed in guilds")
    }
}

impl<T: CtxKind> crate::bot::core::model::AuthorPermissionsAware for Ctx<T> {
    fn author_permissions(&self) -> Permissions {
        self.inner
            .member
            .as_ref()
            .expect("this interaction must be executed in guilds")
            .permissions
            .expect("this field should exist")
    }
}

impl<T: TargetIdAware + AppCtxKind> Ctx<AppCommand<T>> {
    pub fn target_id(&self) -> Id<GenericMarker> {
        self.command_data()
            .target_id
            .expect("`self.command_data().target_id` must exist")
    }
}

// impl Ctx<UserCommand> {
//     #[inline]
//     pub fn target_user_id(&self) -> Id<UserMarker> {
//         self.target_id().cast()
//     }

//     pub fn target_user(&self) -> &User {
//         self.command_data()
//             .resolved
//             .as_ref()
//             .expect("`self.command_data().resolved` must exist")
//             .users
//             .get(&self.target_user_id())
//             .expect("user must exist")
//     }
// }

impl Ctx<MessageCommand> {
    #[inline]
    pub fn target_message_id(&self) -> Id<MessageMarker> {
        self.target_id().cast()
    }

    pub fn target_message(&self) -> &Message {
        self.command_data()
            .resolved
            .as_ref()
            .expect("`self.command_data().resolved` must exist")
            .messages
            .get(&self.target_message_id())
            .expect("message must exist")
    }
}

impl<T: CommandDataAware> Ctx<T> {
    pub fn from_partial_data(
        inner: Box<InteractionCreate>,
        data: &CommandData,
        bot: OwnedBotState,
        latency: Latency,
        sender: MessageSender,
    ) -> Self {
        Self {
            data: Some(PartialInteractionData::Command(PartialCommandData::new(
                data,
            ))),
            inner,
            bot,
            latency,
            sender,
            acknowledged: false,
            kind: PhantomData::<fn(T) -> T>,
        }
    }

    pub fn command_data(&self) -> &PartialCommandData {
        let PartialInteractionData::Command(data) = self.interaction_data() else {
            unreachable!()
        };
        data
    }

    pub fn take_partial_command_data(&mut self) -> Option<PartialCommandData> {
        self.data.take().and_then(|d| match d {
            PartialInteractionData::Command(data) => Some(data),
            _ => None,
        })
    }

    pub fn command_name_full(&self) -> Box<str> {
        fn recurse_through_names(
            mut names: Vec<Arc<str>>,
            command_data_options: &[CommandDataOption],
        ) -> Vec<Arc<str>> {
            match command_data_options {
                [CommandDataOption {
                    name,
                    value:
                        CommandOptionValue::SubCommand(command_data_options)
                        | CommandOptionValue::SubCommandGroup(command_data_options),
                }] => {
                    names.push(name.clone().into());
                    recurse_through_names(names, command_data_options)
                }
                _ => names,
            }
        }

        recurse_through_names(
            vec![self.command_data().name.clone()],
            &self.command_data().options,
        )
        .join(" ")
        .into()
    }

    pub fn command_mention_full(&self) -> Box<str> {
        format!("</{}:{}>", self.command_name_full(), self.command_data().id).into()
    }
}

impl Ctx<ModalMarker> {
    pub fn submit_data(&self) -> &ModalInteractionData {
        let Some(InteractionData::ModalSubmit(ref data)) = self.inner.data else {
            unreachable!()
        };
        data
    }
}

impl<T: RespondViaMessage> Ctx<T> {
    fn base_response_data_builder() -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }

    async fn respond_with(
        &mut self,
        data: Option<InteractionResponseData>,
    ) -> MessageRespondResult {
        let response = self.interface().await?.respond_with(data).await;
        self.acknowledge();
        Ok(response?)
    }

    pub async fn respond(&mut self, content: impl Into<String> + Send) -> MessageRespondResult {
        let data = Self::base_response_data_builder().content(content).build();
        self.respond_with(Some(data)).await
    }

    pub async fn update_no_components_embeds(&mut self, content: &str) -> MessageFollowupResult {
        Ok(self
            .interface()
            .await?
            .update_no_components_embeds(content)
            .await?)
    }

    pub async fn respond_embeds_only(
        &mut self,
        embeds: impl IntoIterator<Item = Embed> + Send,
    ) -> MessageRespondResult {
        let data = Self::base_response_data_builder().embeds(embeds).build();
        self.respond_with(Some(data)).await
    }

    pub async fn respond_embeds_and_components(
        &mut self,
        embeds: impl IntoIterator<Item = Embed> + Send,
        components: impl IntoIterator<Item = MessageComponent> + Send,
    ) -> MessageRespondResult {
        let data = Self::base_response_data_builder()
            .embeds(embeds)
            .components(components)
            .build();
        self.respond_with(Some(data)).await
    }

    pub async fn ephem(&mut self, content: impl Into<String> + Send) -> MessageRespondResult {
        let data = Self::base_response_data_builder()
            .content(content)
            .flags(MessageFlags::EPHEMERAL)
            .build();
        self.respond_with(Some(data)).await
    }

    pub async fn followup(&self, content: &str) -> MessageFollowupResult {
        Ok(self.interface().await?.followup(content).await?)
    }

    pub async fn followup_ephem(&self, content: &str) -> MessageFollowupResult {
        Ok(self.interface().await?.followup_ephem(content).await?)
    }
}

impl<T: RespondViaModal> Ctx<T> {
    pub async fn modal(
        &mut self,
        custom_id: impl Into<String> + Send,
        title: impl Into<String> + Send,
        text_inputs: impl IntoIterator<Item = impl Into<MessageComponent>> + Send,
    ) -> UnitRespondResult {
        let response = self
            .interface()
            .await?
            .modal(custom_id, title, text_inputs)
            .await;
        self.acknowledge();
        Ok(response?)
    }
}

impl Ctx<AutocompleteMarker> {
    pub async fn autocomplete(
        &mut self,
        choices: impl IntoIterator<Item = CommandOptionChoice> + Send,
    ) -> UnitRespondResult {
        let response = self.interface().await?.autocomplete(choices).await;
        self.acknowledge();
        Ok(response?)
    }
}

#[derive(Debug)]
pub struct PartialCommandData {
    pub id: Id<CommandMarker>,
    pub name: Arc<str>,
    pub target_id: Option<Id<GenericMarker>>,
    pub resolved: Option<CommandInteractionDataResolved>,
    pub options: Box<[CommandDataOption]>,
}

impl PartialCommandData {
    pub fn new(data: &CommandData) -> Self {
        Self {
            id: data.id,
            name: data.name.to_string().into(),
            target_id: data.target_id,
            resolved: data.resolved.clone(),
            options: data.options.clone().into(),
        }
    }
}

#[non_exhaustive]
pub enum PartialInteractionData {
    Command(PartialCommandData),
    _Other,
}

pub trait CommandInfoAware {
    fn name() -> Box<str>;
}

pub trait BotSlashCommand: CommandInfoAware {
    async fn run(self, ctx: Ctx<SlashCommand>) -> CommandResult;
}

// pub trait BotUserCommand: CommandInfoAware {
//     async fn run(ctx: Ctx<UserCommand>) -> CommandResult;
// }

pub trait BotMessageCommand: CommandInfoAware {
    async fn run(ctx: Ctx<MessageCommand>) -> CommandResult;
}

pub trait BotAutocomplete {
    async fn execute(self, ctx: Ctx<AutocompleteMarker>) -> AutocompleteResult;
}
