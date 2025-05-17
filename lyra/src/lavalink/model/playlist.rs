use std::sync::Arc;

use lavalink_rs::model::track::{PlaylistData, PlaylistInfo, TrackData};

use crate::lavalink::{PluginInfo, PluginPlaylistInfo};

use super::CorrectPlaylistInfo;

#[derive(Debug)]
pub struct PlaylistMetadata {
    pub uri: Box<str>,
    info: PlaylistInfo,
    plugin_info: Option<PluginPlaylistInfo>,
}

impl PlaylistMetadata {
    pub fn new(uri: Box<str>, data: PlaylistData) -> Self {
        Self {
            uri,
            plugin_info: data.parse_plugin_info(),
            info: data.info,
        }
    }

    pub fn corrected_name(&self) -> &str {
        self.info.corrected_name()
    }

    pub const fn plugin_info(&self) -> Option<&PluginPlaylistInfo> {
        self.plugin_info.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct PlaylistAwareTrackData {
    inner: TrackData,
    playlist: Option<Arc<PlaylistMetadata>>,
}

impl PlaylistAwareTrackData {
    pub const fn new(inner: TrackData, playlist: Arc<PlaylistMetadata>) -> Self {
        Self {
            inner,
            playlist: Some(playlist),
        }
    }

    const fn new_no_playlist(inner: TrackData) -> Self {
        Self {
            inner,
            playlist: None,
        }
    }

    pub const fn inner(&self) -> &TrackData {
        &self.inner
    }

    pub fn into_inner(self) -> TrackData {
        self.inner
    }

    pub fn playlist(&self) -> Option<&PlaylistMetadata> {
        self.playlist.as_deref()
    }
}

impl From<TrackData> for PlaylistAwareTrackData {
    fn from(value: TrackData) -> Self {
        Self::new_no_playlist(value)
    }
}

pub fn make_playlist_aware(
    tracks: impl IntoIterator<Item = TrackData>,
    playlist: PlaylistMetadata,
) -> Vec<PlaylistAwareTrackData> {
    let p = Arc::new(playlist);
    tracks
        .into_iter()
        .map(|t| PlaylistAwareTrackData::new(t, p.clone()))
        .collect()
}
