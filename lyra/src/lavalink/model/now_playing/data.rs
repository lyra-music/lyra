use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use lavalink_rs::model::GuildId;
use lyra_ext::image::dominant_palette;
use twilight_model::id::Id;

use crate::{
    command::util::{DisplayAvatarUrlAware, DisplayNameAware, GuildAvatarUrlAware},
    error::{
        Cache,
        lavalink::{GetDominantPaletteFromUrlError, NewNowPlayingDataError},
    },
    lavalink::{
        ClientData, CorrectTrackInfo, IndexerType, PlayerDataRead, PluginInfo, QueueItem,
        RepeatMode, UnwrappedPlayerInfoUri, model::ArtworkCache,
    },
};

#[derive(Clone)]
pub(super) struct Artwork {
    pub(super) colour: u32,
    url: Box<str>,
}

impl Artwork {
    pub(super) const fn url(&self) -> &str {
        &self.url
    }
}

pub(super) struct Playlist {
    name: Box<str>,
    uri: Box<str>,
}

impl Playlist {
    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn uri(&self) -> &str {
        &self.uri
    }
}

pub(super) struct Artist {
    name: Box<str>,
    url: Option<Box<str>>,
    artwork_url: Option<Box<str>>,
}

impl Artist {
    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    pub(super) fn artwork_url(&self) -> Option<&str> {
        self.artwork_url.as_deref()
    }
}

pub(super) struct Queue {
    len: usize,
    position: NonZeroUsize,
    indexer: IndexerType,
    repeat_mode: RepeatMode,
}

impl Queue {
    pub(super) const fn len(&self) -> usize {
        self.len
    }

    pub(super) const fn set_len(&mut self, len: usize) {
        self.len = len;
    }

    pub(super) const fn position(&self) -> NonZeroUsize {
        self.position
    }

    pub(super) const fn set_position(&mut self, position: NonZeroUsize) {
        self.position = position;
    }

    pub(super) const fn indexer(&self) -> IndexerType {
        self.indexer
    }

    pub(super) const fn set_indexer(&mut self, indexer: IndexerType) {
        self.indexer = indexer;
    }

    pub(super) const fn repeat_mode(&self) -> RepeatMode {
        self.repeat_mode
    }

    pub(super) const fn set_repeat_mode(&mut self, repeat_mode: RepeatMode) {
        self.repeat_mode = repeat_mode;
    }
}

pub(super) struct Requester {
    name: Box<str>,
    avatar: Box<str>,
}

impl Requester {
    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn avatar(&self) -> &str {
        &self.avatar
    }
}

pub(super) struct Preview {
    pub(super) is_preview: bool,
    url: Option<Box<str>>,
}

impl Preview {
    pub(super) fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

pub struct Data {
    pub(super) paused: bool,
    pub(super) speed: f64,
    pub(super) timestamp: Duration,
    pub(super) duration: Duration,
    pub(super) enqueued: Duration,
    queue: Queue,
    artist: Artist,
    requester: Requester,
    preview: Option<Preview>,
    artwork: Option<Artwork>,
    playlist: Option<Playlist>,
    title: Box<str>,
    url: Box<str>,
    album_name: Option<Box<str>>,
}

impl Data {
    pub(super) const fn playlist(&self) -> Option<&Playlist> {
        self.playlist.as_ref()
    }

    pub(super) fn album_name(&self) -> Option<&str> {
        self.album_name.as_deref()
    }

    pub(super) fn title(&self) -> &str {
        &self.title
    }

    pub(super) fn url(&self) -> &str {
        &self.url
    }

    pub(super) const fn requester(&self) -> &Requester {
        &self.requester
    }

    pub(super) const fn artist(&self) -> &Artist {
        &self.artist
    }

    pub(super) const fn artwork(&self) -> Option<&Artwork> {
        self.artwork.as_ref()
    }

    pub(super) const fn queue(&self) -> &Queue {
        &self.queue
    }

    pub(super) const fn queue_mut(&mut self) -> &mut Queue {
        &mut self.queue
    }

    pub(super) const fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }

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

    async fn impl_new(
        client_data: &ClientData,
        guild_id: GuildId,
        data: &PlayerDataRead<'_>,
        track: &QueueItem,
        timestamp: Duration,
    ) -> Result<Self, NewNowPlayingDataError> {
        let requester_id = track.requester();
        let track_data = track.data();
        let playlist_data = track.playlist_data();
        let track_info = &track_data.info;
        let twilight_guild_id = Id::new(guild_id.0);
        let cache = &client_data.cache;
        let requester_m = cache.member(twilight_guild_id, requester_id).ok_or(Cache)?;
        let plugin_info = track_data.parse_plugin_info();
        let queue = data.queue();

        let (requester_name, requester_avatar) = {
            let get_user = || cache.user(requester_id).ok_or(Cache);
            let (name, avatar) = match (
                requester_m.nick(),
                requester_m.guild_avatar_url(twilight_guild_id),
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
        drop(requester_m);

        let artwork_url = plugin_info
            .as_ref()
            .and_then(|info| info.album_art_url.as_deref())
            .or(track_info.artwork_url.as_deref())
            .or_else(|| {
                playlist_data
                    .and_then(|x| x.plugin_info())
                    .and_then(|p| p.artwork_url.as_deref())
            });
        let album_name = plugin_info
            .as_ref()
            .and_then(|info| info.album_name.clone().map(From::from));

        Ok(Self {
            artwork: if let Some(url) = artwork_url {
                let dominant_palette =
                    Self::get_dominant_palette_from_url(&client_data.artwork_cache, url, 4).await?;
                Some(Artwork {
                    url: url.into(),
                    colour: dominant_palette[0],
                })
            } else {
                None
            },
            playlist: playlist_data.map(|i| Playlist {
                name: i.corrected_name().into(),
                uri: i.uri.clone(),
            }),
            artist: Artist {
                name: track_info.corrected_author().into(),
                url: plugin_info
                    .as_ref()
                    .and_then(|i| i.artist_url.clone().map(From::from)),
                artwork_url: plugin_info
                    .as_ref()
                    .and_then(|i| i.artist_artwork_url.clone().map(From::from)),
            },
            queue: Queue {
                len: queue.len(),
                position: queue.position(),
                indexer: queue.indexer_type(),
                repeat_mode: queue.repeat_mode(),
            },
            requester: Requester {
                name: requester_name,
                avatar: requester_avatar,
            },
            preview: plugin_info.as_ref().map(|i| Preview {
                is_preview: i.is_preview,
                url: i.preview_url.clone().map(From::from),
            }),
            album_name,
            timestamp,
            speed: data.speed(),
            paused: data.paused(),
            title: track_info.corrected_title().into(),
            url: track_info.uri_unwrapped().into(),
            enqueued: track.enqueued(),
            duration: Duration::from_millis(track_info.length),
        })
    }

    #[inline]
    pub async fn new(
        client_data: &ClientData,
        guild_id: GuildId,
        data: &PlayerDataRead<'_>,
        track: &QueueItem,
    ) -> Result<Self, NewNowPlayingDataError> {
        Self::impl_new(client_data, guild_id, data, track, data.timestamp()).await
    }

    #[inline]
    pub async fn new_zeroed_timestamp(
        client_data: &ClientData,
        guild_id: GuildId,
        data: &PlayerDataRead<'_>,
        track: &QueueItem,
    ) -> Result<Self, NewNowPlayingDataError> {
        Self::impl_new(client_data, guild_id, data, track, Duration::ZERO).await
    }
}
