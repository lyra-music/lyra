use std::{fmt::Write, num::NonZeroUsize, sync::Arc, time::Duration};

use lyra_ext::pretty::duration_display::DurationDisplay;
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
    core::{emoji, model::HttpAware, r#static::component::NOW_PLAYING_BUTTON_IDS},
    error::{
        core::DeserialiseBodyFromHttpError,
        lavalink::{
            BuildNowPlayingEmbedError, NewNowPlayingMessageError, UpdateNowPlayingMessageError,
        },
    },
    lavalink::{IndexerType, RepeatMode},
};

use super::{Data, data::Playlist};

#[derive(Clone, Copy)]
pub enum Update {
    Indexer(IndexerType),
    Repeat(RepeatMode),
    Paused(bool),
    QueueLen(usize),
    QueuePosition(NonZeroUsize),
    Speed(f64),
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

struct AlbumInfo<'a>(Option<&'a str>);

impl<'a> From<&'a Data> for AlbumInfo<'a> {
    fn from(value: &'a Data) -> Self {
        Self(value.album_name())
    }
}

impl std::fmt::Display for AlbumInfo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(n) = self.0.as_ref() {
            f.write_str("📀 **")?;
            f.write_str(n)?;
            f.write_str("**\n")?;
        }
        Ok(())
    }
}

struct PlaylistInfo<'a>(Option<&'a Playlist>);

impl<'a> From<&'a Data> for PlaylistInfo<'a> {
    fn from(value: &'a Data) -> Self {
        Self(value.playlist())
    }
}

impl std::fmt::Display for PlaylistInfo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(p) = self.0.as_ref() {
            f.write_str("📚 **[")?;
            f.write_str(p.name())?;
            f.write_str("](")?;
            f.write_str(p.uri())?;
            f.write_str(")**\n")?;
        }
        Ok(())
    }
}

struct Description<'a>(&'a Data);

impl std::fmt::Display for Description<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.0;
        AlbumInfo::from(data).fmt(f)?;
        PlaylistInfo::from(data).fmt(f)?;
        f.write_str("#️⃣ **")?;
        data.queue().position().fmt(f)?;
        f.write_str("** / ")?;
        data.queue().len().fmt(f)?;
        f.write_str(" ⏳ ")?;
        DurationLeft::from(data).fmt(f)?;
        f.write_str(" / ")?;
        data.duration.pretty_display().fmt(f)?;
        if let Some(p) = data.preview() {
            if p.is_preview || p.url().is_some() {
                f.write_str("\n-# ")?;
                if p.is_preview {
                    f.write_str("This track is a preview. ")?;
                }
                if let Some(url) = p.url() {
                    f.write_str("Listen to the preview of this track [here](")?;
                    f.write_str(url)?;
                    f.write_str("). ")?;
                }
            }
        }
        Ok(())
    }
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

    pub(in super::super) const fn update(&mut self, update: Update) {
        match update {
            Update::Indexer(idx) => self.data.queue_mut().set_indexer(idx),
            Update::Repeat(mode) => self.data.queue_mut().set_repeat_mode(mode),
            Update::Paused(paused) => self.data.paused = paused,
            Update::QueueLen(i) => self.data.queue_mut().set_len(i),
            Update::QueuePosition(i) => self.data.queue_mut().set_position(i),
            Update::Speed(s) => self.data.speed = s,
        }
    }

    pub(in super::super) const fn update_timestamp(&mut self, timestamp: Duration) {
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
        let emoji = match self.data.queue().repeat_mode() {
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
            let (shuffle_emoji, shuffle_disabled) = match self.data.queue().indexer() {
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

        #[expect(clippy::cast_possible_truncation)]
        let timestamp =
            twilight_model::util::Timestamp::from_micros(data.enqueued.as_micros() as i64)?;

        let footer = EmbedFooterBuilder::new(data.requester().name())
            .icon_url(ImageSource::url(data.requester().avatar())?)
            .build();

        let artist = data.artist();
        let mut author = EmbedAuthorBuilder::new(artist.name());
        if let Some(url) = artist.url() {
            author = author.url(url);
        }
        if let Some(url) = artist.artwork_url() {
            author = author.icon_url(ImageSource::url(url)?);
        }

        let mut embed = EmbedBuilder::new()
            .title(data.title())
            .url(data.url())
            .description(Description(data).to_string())
            .timestamp(timestamp)
            .author(author.build())
            .footer(footer);

        if let Some(artwork) = data.artwork() {
            embed = embed
                .color(artwork.colour)
                .thumbnail(ImageSource::url(artwork.url())?);
        }
        Ok(embed.build())
    }
}
