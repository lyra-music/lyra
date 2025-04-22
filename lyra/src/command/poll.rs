use std::{
    collections::{
        HashMap, HashSet,
        hash_map::{DefaultHasher, Entry},
    },
    hash::{Hash, Hasher},
    time::Duration,
};

use futures::StreamExt;
use itertools::Itertools;
use lyra_ext::rgb_hex::{hex_to_rgb, rgb_to_hex};
use rand::{Rng, distr::Alphanumeric};
use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    channel::message::{
        Component, Embed, EmojiReactionType,
        component::{ActionRow, Button, ButtonStyle},
    },
    guild::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, MessageMarker, UserMarker},
    },
};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::{
    LavalinkAware,
    command::macros::{caut, hid, nope},
    core::{
        r#const::{
            colours,
            poll::{BASE, DOWNVOTE, RATIO_BAR_SIZE, UPVOTE},
        },
        model::{BotStateAware, CacheAware, HttpAware, UserIdAware},
    },
    error::{
        Cache as CacheError,
        command::poll::{GenerateEmbedError, StartPollError, UpdateEmbedError, WaitForVotesError},
    },
    lavalink::{Event, EventRecvResult},
};

use super::{
    model::{GuildCtx, GuildInteraction, NonPingInteraction, RespondViaMessage},
    require::PartialInVoice,
    util::{GuildIdAndDisplayAvatarUrlAware, GuildIdAndDisplayNameAware, MessageLinkAware},
};

#[derive(Hash)]
pub enum Topic {
    Repeat(crate::lavalink::RepeatMode),
}

impl Topic {
    const fn is_voided_by(&self, event: &Event) -> bool {
        match self {
            Self::Repeat(_) => matches!(event, Event::QueueClear | Event::QueueRepeat),
        }
    }
}

impl std::fmt::Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Repeat(mode) => match mode {
                crate::lavalink::RepeatMode::Off => "Disable repeat for the entire queue",
                crate::lavalink::RepeatMode::All => "Enable repeat for the entire queue",
                crate::lavalink::RepeatMode::Track => "Enable repeat for only the current track",
            },
        };
        f.write_str(message)
    }
}

#[derive(Debug)]
pub struct Poll {
    topic_hash: u64,
    message: super::util::MessageLinkComponent,
}

impl Poll {
    fn new(topic: &Topic, message: super::util::MessageLinkComponent) -> Self {
        let mut s = DefaultHasher::new();
        topic.hash(&mut s);

        Self {
            topic_hash: s.finish(),
            message,
        }
    }

    pub const fn topic_hash(&self) -> u64 {
        self.topic_hash
    }

    pub fn message_owned(&self) -> super::util::MessageLinkComponent {
        self.message.clone()
    }
}

struct Voter {
    permissions: Permissions,
}

impl Voter {
    const fn new(permissions: Permissions) -> Self {
        Self { permissions }
    }
}

impl crate::core::model::UserPermissionsAware for Voter {
    fn user_permissions(&self) -> Permissions {
        self.permissions
    }
}

#[derive(Copy, Clone, Debug, const_panic::PanicFmt)]
pub struct Vote(bool);

impl Vote {
    const fn value(self) -> isize {
        if self.0 { 1 } else { -1 }
    }
}

impl std::fmt::Display for Vote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = if self.0 { "Agree" } else { "Disagree" };
        f.write_str(s)
    }
}

#[derive(Debug)]
pub enum VoidingEvent {
    QueueClear,
    QueueRepeat,
}

impl VoidingEvent {
    const fn new(event: &Event) -> Self {
        match event {
            Event::QueueClear => Self::QueueClear,
            Event::QueueRepeat => Self::QueueRepeat,
            _ => const_panic::concat_panic!("invalid event: ", {}: event),
        }
    }
}

enum PollAction {
    Cast {
        user_id: Id<UserMarker>,
        vote: Vote,
        interaction: Interaction,
    },
    AlternateCast(Id<UserMarker>),
    AlternateDjCast,
    DjUpvote(Interaction),
    DjDownvote(Interaction),
    Void(VoidingEvent),
}

pub enum Resolution {
    UnanimousWin,
    UnanimousLoss,
    TimedOut,
    Voided(VoidingEvent),
    SupersededWinViaDj,
    SupersededLossViaDj,
}

struct LatentEmbedColours {
    base: [f32; 7],
    upvote: [f32; 7],
    downvote: [f32; 7],
}

struct UpdatePollEmbedContext {
    embed: Embed,
    latent: LatentEmbedColours,
}

struct WaitForPollActionsContext<'a> {
    topic: &'a Topic,
    components: &'a mut twilight_standby::future::WaitForComponentStream,
    upvote_button_id: String,
    message: super::util::MessageLinkComponent,
}

fn handle_interactions(inter: Interaction, upvote_button_id: &String) -> PollAction {
    let user_id = inter.author_id_expected();

    let Some(InteractionData::MessageComponent(ref component)) = inter.data else {
        unreachable!()
    };

    let voter_permissions = inter.author_permissions_expected();

    match (
        super::check::is_user_dj(&Voter::new(voter_permissions)),
        component.custom_id == *upvote_button_id,
    ) {
        (true, true) => PollAction::DjUpvote(inter),
        (true, false) => PollAction::DjDownvote(inter),
        (false, upvote) => PollAction::Cast {
            user_id,
            vote: Vote(upvote),
            interaction: inter,
        },
    }
}

fn generate_embed(
    topic: &Topic,
    author_name: &str,
    author_icon: String,
    votes: &HashMap<Id<UserMarker>, Vote>,
    threshold: usize,
    latent: &LatentEmbedColours,
) -> Result<Embed, GenerateEmbedError> {
    let embed_color = generate_embed_colour(votes, threshold, latent);
    let embed = EmbedBuilder::new()
        .author(EmbedAuthorBuilder::new(author_name).icon_url(ImageSource::url(author_icon)?))
        .title(format!("{topic}?"))
        .description(generate_poll_description(votes, threshold))
        .footer(EmbedFooterBuilder::new(
            "Cast your votes via pressing the buttons below",
        ))
        .color(rgb_to_hex(embed_color))
        .validate()?
        .build();
    Ok(embed)
}

fn generate_upvote_button_id_and_row() -> (String, Component) {
    let (upvote_button_id, downvote_button_id): (String, _) = {
        let mut button_id_iter = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(200)
            .map(char::from);

        (
            button_id_iter.by_ref().take(100).collect(),
            button_id_iter.collect(),
        )
    };

    let upvote_button = Component::Button(Button {
        custom_id: Some(upvote_button_id.clone()),
        disabled: false,
        emoji: Some(EmojiReactionType::Unicode {
            name: String::from("➕"),
        }),
        label: None,
        style: ButtonStyle::Primary,
        url: None,
        sku_id: None,
    });
    let downvote_button = Component::Button(Button {
        custom_id: Some(downvote_button_id),
        disabled: false,
        emoji: Some(EmojiReactionType::Unicode {
            name: String::from("➖"),
        }),
        label: None,
        style: ButtonStyle::Danger,
        url: None,
        sku_id: None,
    });
    let row = Component::ActionRow(ActionRow {
        components: vec![upvote_button, downvote_button],
    });
    (upvote_button_id, row)
}

fn generate_latent_embed_colours() -> LatentEmbedColours {
    let upvote = mixbox::rgb_to_latent(&(hex_to_rgb(colours::UPVOTE)));
    let downvote = mixbox::rgb_to_latent(&hex_to_rgb(colours::DOWNVOTE));
    let base = mixbox::rgb_to_latent(&hex_to_rgb(colours::POLL_BASE));

    LatentEmbedColours {
        base,
        upvote,
        downvote,
    }
}

fn get_users_in_voice(
    ctx: &GuildCtx<impl RespondViaMessage>,
    in_voice: &PartialInVoice,
) -> Result<HashSet<Id<UserMarker>>, CacheError> {
    let users_in_voice = ctx
        .cache()
        .voice_channel_states(ctx.lavalink().connection_from(in_voice).channel_id)
        .ok_or(CacheError)?
        .map(|v| ctx.cache().user(v.user_id()).ok_or(CacheError))
        .filter_map_ok(|u| (!u.bot).then_some(u.id))
        .collect::<Result<HashSet<_>, _>>()?;
    Ok(users_in_voice)
}

async fn wait_for_poll_actions(
    rx: &mut tokio::sync::broadcast::Receiver<Event>,
    ctx: &mut WaitForPollActionsContext<'_>,
) -> EventRecvResult<Option<PollAction>> {
    tokio::select! {
        event = rx.recv() => {
            Ok(match event? {
                Event::AlternateVoteCast(id) => Some(PollAction::AlternateCast(id.into())),
                Event::AlternateVoteDjCast => Some(PollAction::AlternateDjCast),
                e if ctx.topic.is_voided_by(&e) => Some(PollAction::Void(VoidingEvent::new(&e))),
                _ => None
            })
        }
        Some(inter) = ctx.components.next() => Ok(Some(handle_interactions(inter, &ctx.upvote_button_id))),
    }
}

enum EmbedUpdate<'a> {
    InteractionResponse(crate::core::model::InteractionInterface<'a>),
    Http {
        client: &'a twilight_http::Client,
        channel_id: Id<ChannelMarker>,
        message_id: Id<MessageMarker>,
    },
}

impl EmbedUpdate<'_> {
    async fn update_embed(self, embed: Embed) -> Result<(), UpdateEmbedError> {
        match self {
            EmbedUpdate::InteractionResponse(i) => i.update_message_embeds_only([embed]).await?,
            EmbedUpdate::Http {
                client,
                channel_id,
                message_id,
            } => {
                client
                    .update_message(channel_id, message_id)
                    .embeds(Some(&[embed]))
                    .await?
            }
        };
        Ok(())
    }
}

fn calculate_votes_and_ratios(
    votes: &HashMap<Id<UserMarker>, Vote>,
    threshold: usize,
) -> ((usize, usize), (f64, f64, f64)) {
    let total_votes = votes.len();
    let upvotes = votes
        .values()
        .copied()
        .filter(|v| v.value().is_positive())
        .count();
    let downvotes = total_votes - upvotes;
    let votes_left = threshold - total_votes;

    #[allow(clippy::cast_precision_loss)]
    let (threshold_f64, upvotes_f64, downvotes_f64, votes_left_f64) = (
        threshold as f64,
        upvotes as f64,
        downvotes as f64,
        votes_left as f64,
    );

    let upvote_ratio = upvotes_f64 / threshold_f64;
    let downvote_ratio = downvotes_f64 / threshold_f64;
    let votes_left_ratio = votes_left_f64 / threshold_f64;

    (
        (upvotes, downvotes),
        (upvote_ratio, downvote_ratio, votes_left_ratio),
    )
}

fn calculate_vote_ratios(
    votes: &HashMap<Id<UserMarker>, Vote>,
    threshold: usize,
) -> (f64, f64, f64) {
    calculate_votes_and_ratios(votes, threshold).1
}

fn generate_embed_colour(
    votes: &HashMap<Id<UserMarker>, Vote>,
    threshold: usize,
    latent: &LatentEmbedColours,
) -> [u8; 3] {
    let (upvote_ratio, downvote_ratio, votes_left_ratio) = calculate_vote_ratios(votes, threshold);
    let mut z_mix = [0.0; mixbox::LATENT_SIZE];
    for (i, z) in z_mix.iter_mut().enumerate() {
        *z = votes_left_ratio.mul_add(
            f64::from(latent.base[i]),
            upvote_ratio.mul_add(
                f64::from(latent.upvote[i]),
                downvote_ratio * f64::from(latent.downvote[i]),
            ),
        );
    }

    #[allow(clippy::cast_possible_truncation)]
    mixbox::latent_to_rgb(&z_mix.map(|f| f as f32))
}

async fn update_embed(
    votes: &HashMap<Id<UserMarker>, Vote>,
    threshold: usize,
    embed_ctx: &UpdatePollEmbedContext,
    updater: EmbedUpdate<'_>,
) -> Result<(), UpdateEmbedError> {
    let embed_color = generate_embed_colour(votes, threshold, &embed_ctx.latent);
    let embed = EmbedBuilder::from(embed_ctx.embed.clone())
        .color(rgb_to_hex(embed_color))
        .description(generate_poll_description(votes, threshold))
        .validate()?
        .build();
    updater.update_embed(embed).await?;
    Ok(())
}

pub async fn start(
    topic: &Topic,
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    in_voice: &PartialInVoice,
) -> Result<Resolution, StartPollError> {
    let author_name = ctx.guild_display_name();
    let author_icon = ctx.guild_display_avatar_url();
    let embed_latent = generate_latent_embed_colours();
    let (upvote_button_id, row) = generate_upvote_button_id_and_row();

    let users_in_voice = get_users_in_voice(ctx, in_voice)?;
    let votes = HashMap::from([(ctx.user_id(), Vote(true))]);

    #[allow(clippy::cast_precision_loss)]
    let users_in_voice_plus_1 = (users_in_voice.len() + 1) as f64;

    #[allow(clippy::cast_possible_truncation)]
    let threshold = ((users_in_voice_plus_1 / 2.).round() as isize).unsigned_abs();

    let embed = generate_embed(
        topic,
        author_name,
        author_icon,
        &votes,
        threshold,
        &embed_latent,
    )?;
    let message = super::util::MessageLinkComponent::from(
        ctx.respond_embeds_and_components([embed.clone()], [row])
            .await?
            .model()
            .await?,
    );

    let message_id = message.id();
    {
        ctx.lavalink()
            .connection_mut_from(in_voice)
            .set_poll(Poll::new(topic, message.clone()));
    }
    let components = &mut ctx
        .bot()
        .standby()
        .wait_for_component_stream(message_id, |_: &_| true);

    let embed_ctx = UpdatePollEmbedContext {
        embed,
        latent: embed_latent,
    };

    let poll_ctx = WaitForPollActionsContext {
        topic,
        components,
        upvote_button_id,
        message,
    };

    Ok(Box::pin(wait_for_votes(
        poll_ctx,
        ctx,
        users_in_voice,
        votes,
        threshold,
        embed_ctx,
        in_voice,
    ))
    .await?)
}

fn calculate_vote_resolution(
    votes: &HashMap<Id<UserMarker>, Vote>,
    threshold: usize,
) -> Option<Resolution> {
    let res = votes.values().copied().map(Vote::value).sum::<isize>();
    if res.unsigned_abs() == threshold {
        if res.is_positive() {
            return Some(Resolution::UnanimousWin);
        }
        return Some(Resolution::UnanimousLoss);
    }
    None
}

fn generate_poll_description(votes: &HashMap<Id<UserMarker>, Vote>, threshold: usize) -> String {
    let ((upvotes, downvotes), (upvote_ratio, downvote_ratio, _)) =
        calculate_votes_and_ratios(votes, threshold);

    #[allow(clippy::cast_precision_loss)]
    let ratio_bar_size = RATIO_BAR_SIZE as f64;

    #[allow(clippy::cast_possible_truncation)]
    let (upvote_char_n, downvote_char_n) = (
        ((upvote_ratio * ratio_bar_size) as isize).unsigned_abs(),
        ((downvote_ratio * ratio_bar_size) as isize).unsigned_abs(),
    );
    let votes_left_char_n = RATIO_BAR_SIZE - upvote_char_n - downvote_char_n;

    format!(
        "**Upvotes** / **Downvotes** / **Votes Needed** **»**  **`{upvotes}`**/**`{downvotes}`**/**`{threshold}`**\n\
        {}{}{}",
        UPVOTE.repeat(upvote_char_n),
        DOWNVOTE.repeat(downvote_char_n),
        BASE.repeat(votes_left_char_n)
    )
}

async fn wait_for_votes(
    mut poll_ctx: WaitForPollActionsContext<'_>,
    ctx: &GuildCtx<impl RespondViaMessage>,
    users_in_voice: HashSet<Id<UserMarker>>,
    mut votes: HashMap<Id<UserMarker>, Vote>,
    threshold: usize,
    embed_ctx: UpdatePollEmbedContext,
    in_voice: &PartialInVoice,
) -> Result<Resolution, WaitForVotesError> {
    let mut rx = ctx.lavalink().connection_from(in_voice).subscribe();
    loop {
        let poll_stream = wait_for_poll_actions(&mut rx, &mut poll_ctx);
        match tokio::time::timeout(Duration::from_secs(30), poll_stream).await {
            Ok(Ok(Some(action))) => match action {
                PollAction::Cast {
                    user_id,
                    vote,
                    interaction,
                } => {
                    let i = ctx.bot().interaction().await?.interfaces(&interaction);
                    if !users_in_voice.contains(&user_id) {
                        nope!("You are not eligible to cast a vote to this poll.", ?i);
                        continue;
                    }

                    match votes.entry(user_id) {
                        Entry::Vacant(e) => {
                            e.insert(vote);
                        }
                        Entry::Occupied(e) => {
                            caut!(
                                format!("You've already casted a vote: **{}**.", e.get()),
                                ?i
                            );
                            continue;
                        }
                    }

                    if let Some(res) = calculate_vote_resolution(&votes, threshold) {
                        return Ok(res);
                    }

                    update_embed(
                        &votes,
                        threshold,
                        &embed_ctx,
                        EmbedUpdate::InteractionResponse(i),
                    )
                    .await?;
                }
                PollAction::AlternateCast(user_id) => {
                    if !users_in_voice.contains(&user_id) {
                        ctx.lavalink()
                            .connection_from(in_voice)
                            .dispatch(Event::AlternateVoteCastDenied);
                        continue;
                    }
                    match votes.entry(user_id) {
                        Entry::Vacant(e) => {
                            e.insert(Vote(true));
                        }
                        Entry::Occupied(e) => {
                            ctx.lavalink()
                                .connection_from(in_voice)
                                .dispatch(Event::AlternateVoteCastedAlready(*e.get()));
                            continue;
                        }
                    }

                    if let Some(res) = calculate_vote_resolution(&votes, threshold) {
                        return Ok(res);
                    }

                    update_embed(
                        &votes,
                        threshold,
                        &embed_ctx,
                        EmbedUpdate::Http {
                            client: ctx.http(),
                            channel_id: poll_ctx.message.channel_id(),
                            message_id: poll_ctx.message.id(),
                        },
                    )
                    .await?;
                }
                PollAction::DjUpvote(inter) => {
                    let i = ctx.bot().interaction().await?.interfaces(&inter);
                    hid!(format!("🪄 Superseded this poll to win."), ?i);
                    return Ok(Resolution::SupersededWinViaDj);
                }
                PollAction::DjDownvote(inter) => {
                    let i = ctx.bot().interaction().await?.interfaces(&inter);
                    hid!(format!("🪄 Superseded this poll to lose."), ?i);
                    return Ok(Resolution::SupersededLossViaDj);
                }
                PollAction::AlternateDjCast => return Ok(Resolution::SupersededWinViaDj),
                PollAction::Void(e) => return Ok(Resolution::Voided(e)),
            },
            Ok(Ok(None)) => {}
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Ok(Resolution::TimedOut),
        }
    }
}
