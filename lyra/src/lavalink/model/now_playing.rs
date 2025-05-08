use std::{fmt::Write, num::NonZeroUsize, sync::Arc, time::Duration};

use lavalink_rs::model::GuildId;
use lyra_ext::{image::dominant_palette, pretty::duration_display::DurationDisplay};
use twilight_http::Client;
use twilight_mention::{
    Mention,
    timestamp::{Timestamp, TimestampStyle},
};
use twilight_model::{
    channel::message::{
        Component, Embed, EmojiReactionType,
        component::{ActionRow, Button, ButtonStyle},
    },
    id::{
        Id,
        marker::{ChannelMarker, MessageMarker},
    },
};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::{
    command::util::{DisplayAvatarUrlAware, DisplayNameAware, GuildAvatarUrlAware},
    core::{emoji, model::HttpAware, r#static::component::NOW_PLAYING_BUTTON_IDS},
    error::{
        Cache,
        core::DeserialiseBodyFromHttpError,
        lavalink::{
            BuildNowPlayingEmbedError, GetDominantPaletteFromUrlError, NewNowPlayingDataError,
            NewNowPlayingMessageError, UpdateNowPlayingMessageError,
        },
    },
    lavalink::PluginInfo,
};

use super::{
    ArtworkCache, ClientData, IndexerType, PlayerDataRead, QueueItem, RepeatMode,
    UnwrappedPlayerInfoUri,
};

#[derive(Clone)]
struct Artwork {
    url: Box<str>,
    colour: u32,
}

pub struct Data {
    paused: bool,
    title: Box<str>,
    url: Box<str>,
    queue_position: NonZeroUsize,
    queue_len: usize,
    speed: f64,
    enqueued: Duration,
    timestamp: Duration,
    duration: Duration,
    requester_name: Box<str>,
    requester_avatar: Box<str>,
    artist: Box<str>,
    repeat_mode: RepeatMode,
    indexer: IndexerType,
    artist_url: Option<Box<str>>,
    artwork: Option<Artwork>,
    album_name: Option<Box<str>>,
    artist_artwork_url: Option<Box<str>>,
}

impl Data {
    async fn get_dominant_palette_from_url(
        cache: &ArtworkCache,
        url: &str,
        palette_size: usize,
    ) -> Result<Arc<[u32]>, Arc<GetDominantPaletteFromUrlError>> {
        cache
            .try_get_with((url.into(), palette_size), async {
                let bytes = reqwest::get(url).await?.bytes().await?;
                let palette = dominant_palette::from_bytes(&bytes, palette_size)?;
                Ok(dominant_palette::normalise(palette).into())
            })
            .await
    }

    pub async fn new(
        client_data: &ClientData,
        guild_id: GuildId,
        data: &PlayerDataRead<'_>,
        track: &QueueItem,
    ) -> Result<Self, NewNowPlayingDataError> {
        let requester_id = track.requester();
        let track_data = track.data();
        let track_info = &track_data.info;
        let twilight_guild_id = Id::new(guild_id.0);
        let cache = &client_data.cache;
        let requester = cache.member(twilight_guild_id, requester_id).ok_or(Cache)?;
        let plugin_info = track_data.parse_plugin_info();
        let queue = data.queue();

        let (requester_name, requester_avatar) = {
            let get_user = || cache.user(requester_id).ok_or(Cache);
            let (name, avatar) = match (
                requester.nick(),
                requester.guild_avatar_url(twilight_guild_id),
            ) {
                (Some(nick), Some(url)) => (nick.to_owned(), url),
                (Some(nick), None) => (nick.to_owned(), get_user()?.display_avatar_url()),
                (None, Some(url)) => (get_user()?.display_name().to_owned(), url),
                (None, None) => {
                    let user = get_user()?;
                    (user.display_name().to_owned(), user.display_avatar_url())
                }
            };
            (name.into(), avatar.into())
        };
        drop(requester);

        let artwork_url = plugin_info
            .as_ref()
            .and_then(|info| info.album_art_url.as_ref())
            .or(track_info.artwork_url.as_ref());
        let artwork = if let Some(url) = artwork_url {
            let dominant_palette =
                Self::get_dominant_palette_from_url(&client_data.artwork_cache, url, 4).await?;
            Some(Artwork {
                url: url.clone().into_boxed_str(),
                colour: dominant_palette[0],
            })
        } else {
            None
        };

        let album_name = plugin_info
            .as_ref()
            .and_then(|info| info.album_name.clone())
            .map(From::from);

        let artist_url = plugin_info
            .as_ref()
            .and_then(|info| info.artist_url.clone())
            .map(From::from);

        let artist_artwork_url = plugin_info
            .as_ref()
            .and_then(|info| info.artist_artwork_url.clone())
            .map(From::from);

        Ok(Self {
            requester_name,
            requester_avatar,
            artist_url,
            artwork,
            album_name,
            artist_artwork_url,
            speed: data.speed(),
            paused: data.paused(),
            title: track_info.title.clone().into(),
            url: track_info.uri_unwrapped().into(),
            enqueued: track.enqueued(),
            queue_position: queue.position(),
            queue_len: queue.len(),
            timestamp: Duration::ZERO,
            duration: Duration::from_millis(track_info.length),
            artist: track_info.author.clone().into(),
            repeat_mode: queue.repeat_mode(),
            indexer: queue.indexer_type(),
        })
    }
}

#[derive(Clone, Copy)]
pub enum Update {
    Indexer(IndexerType),
    Repeat(RepeatMode),
    Paused(bool),
}

pub struct Message {
    id: Id<MessageMarker>,
    channel_id: Id<ChannelMarker>,
    data: Data,
    http: Arc<Client>,
}

impl HttpAware for Message {
    fn http(&self) -> &Client {
        &self.http
    }
}

struct DurationLeft {
    inner: Duration,
    total: Duration,
    paused: bool,
}

impl From<&Data> for DurationLeft {
    fn from(value: &Data) -> Self {
        Self {
            inner: (value.duration - value.timestamp).div_f64(value.speed),
            total: value.duration,
            paused: value.paused,
        }
    }
}

impl std::fmt::Display for DurationLeft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.paused {
            f.write_char('`')?;
            (self.total - self.inner).pretty_display().fmt(f)?;
            f.write_char('`')
        } else {
            let unix = (lyra_ext::unix_time() + self.inner).as_secs();
            let style = Some(TimestampStyle::RelativeTime);
            Timestamp::new(unix, style).mention().fmt(f)
        }
    }
}

impl Message {
    pub async fn new(
        http: Arc<Client>,
        msg_data: Data,
        channel_id: Id<ChannelMarker>,
    ) -> Result<Self, NewNowPlayingMessageError> {
        let mut msg = Self {
            id: Id::new(u64::MAX), // default dummy value...
            channel_id,
            data: msg_data,
            http: http.clone(),
        };
        let req = http
            .create_message(channel_id)
            .content(msg.build_content())
            .embeds(&[msg.build_embeds()?])
            .components(&[msg.build_components().await?])
            .await?;
        msg.id = req.model().await?.id; // ...to be updated later here.

        Ok(msg)
    }

    pub const fn id(&self) -> Id<MessageMarker> {
        self.id
    }

    pub const fn channel_id(&self) -> Id<ChannelMarker> {
        self.channel_id
    }

    pub(super) const fn update(&mut self, update: Update) {
        match update {
            Update::Indexer(idx) => self.data.indexer = idx,
            Update::Repeat(mode) => self.data.repeat_mode = mode,
            Update::Paused(paused) => self.data.paused = paused,
        }
    }

    pub(super) const fn update_timestamp(&mut self, timestamp: Duration) {
        self.data.timestamp = timestamp;
    }

    pub async fn apply_update(&self) -> Result<(), UpdateNowPlayingMessageError> {
        self.http
            .update_message(self.channel_id, self.id)
            .content(Some(self.build_content()))
            .embeds(Some(&[self.build_embeds()?]))
            .components(Some(&[self.build_components().await?]))
            .await?;
        Ok(())
    }

    const fn button(
        custom_id: String,
        disabled: bool,
        emoji: EmojiReactionType,
        style: ButtonStyle,
    ) -> Component {
        Component::Button(Button {
            disabled,
            style,
            custom_id: Some(custom_id),
            emoji: Some(emoji),
            label: None,
            url: None,
            sku_id: None,
        })
    }

    const fn build_content(&self) -> &'static str {
        if self.data.paused {
            return "Now Playing";
        }
        "🎵 **Now Playing**"
    }

    async fn build_components(&self) -> Result<Component, DeserialiseBodyFromHttpError> {
        Ok(Component::ActionRow(ActionRow {
            components: vec![
                self.shuffle().await?,
                self.previous().await?,
                self.play_pause().await?,
                self.next().await?,
                self.repeat().await?,
            ],
        }))
    }

    #[inline]
    async fn previous(&self) -> Result<Component, DeserialiseBodyFromHttpError> {
        let emoji = emoji::previous(self).await?.clone();
        let custom_id = NOW_PLAYING_BUTTON_IDS.previous.to_owned();
        let previous_button = Self::button(custom_id, false, emoji, ButtonStyle::Secondary);
        Ok(previous_button)
    }

    #[inline]
    async fn next(&self) -> Result<Component, DeserialiseBodyFromHttpError> {
        let emoji = emoji::next(self).await?.clone();
        let custom_id = NOW_PLAYING_BUTTON_IDS.next.to_owned();
        let next_button = Self::button(custom_id, false, emoji, ButtonStyle::Secondary);
        Ok(next_button)
    }

    #[inline]
    async fn repeat(&self) -> Result<Component, DeserialiseBodyFromHttpError> {
        let emoji = match self.data.repeat_mode {
            RepeatMode::Off => emoji::repeat_off(self).await,
            RepeatMode::All => emoji::repeat_all(self).await,
            RepeatMode::Track => emoji::repeat_track(self).await,
        }?
        .clone();
        let custom_id = NOW_PLAYING_BUTTON_IDS.repeat.to_owned();
        let repeat_button = Self::button(custom_id, false, emoji, ButtonStyle::Success);
        Ok(repeat_button)
    }

    #[inline]
    async fn play_pause(&self) -> Result<Component, DeserialiseBodyFromHttpError> {
        let emoji = if self.data.paused {
            emoji::play(self).await?.clone()
        } else {
            emoji::pause(self).await?.clone()
        };
        let custom_id = NOW_PLAYING_BUTTON_IDS.play_pause.to_owned();
        let play_pause_button = Self::button(custom_id, false, emoji, ButtonStyle::Primary);
        Ok(play_pause_button)
    }

    #[inline]
    async fn shuffle(&self) -> Result<Component, DeserialiseBodyFromHttpError> {
        let (emoji, disabled) = {
            let (shuffle_emoji, shuffle_disabled) = match self.data.indexer {
                IndexerType::Standard => (emoji::shuffle_off(self).await, false),
                IndexerType::Fair => (emoji::shuffle_off(self).await, true),
                IndexerType::Shuffled => (emoji::shuffle_on(self).await, false),
            };
            (shuffle_emoji?.clone(), shuffle_disabled)
        };
        let custom_id = NOW_PLAYING_BUTTON_IDS.shuffle.to_owned();
        let shuffle_button = Self::button(custom_id, disabled, emoji, ButtonStyle::Danger);
        Ok(shuffle_button)
    }

    fn build_embeds(&self) -> Result<Embed, BuildNowPlayingEmbedError> {
        let data = &self.data;
        let description = {
            let album_info = data
                .album_name
                .clone()
                .map_or_else(String::new, |name| format!("📀 **{name}**\n"));
            format!(
                "{}#️⃣ **{}** / {} ⏳ {} / {}",
                album_info,
                data.queue_position,
                data.queue_len,
                DurationLeft::from(data),
                data.duration.pretty_display()
            )
        };

        #[allow(clippy::cast_possible_truncation)]
        let timestamp =
            twilight_model::util::Timestamp::from_micros(data.enqueued.as_micros() as i64)?;

        let footer = EmbedFooterBuilder::new(data.requester_name.clone())
            .icon_url(ImageSource::url(data.requester_avatar.clone())?)
            .build();

        let mut author = EmbedAuthorBuilder::new(data.artist.clone());
        if let Some(url) = data.artist_url.clone() {
            author = author.url(url);
        }
        if let Some(url) = data.artist_artwork_url.clone() {
            author = author.icon_url(ImageSource::url(url)?);
        }

        let mut embed = EmbedBuilder::new()
            .title(data.title.clone())
            .url(data.url.clone())
            .description(description)
            .timestamp(timestamp)
            .author(author.build())
            .footer(footer);

        if let Some(artwork) = data.artwork.clone() {
            embed = embed
                .color(artwork.colour)
                .thumbnail(ImageSource::url(artwork.url)?);
        }
        Ok(embed.build())
    }
}
