use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PluginTrackInfo {
    /// The name of the album
    album_name: Option<String>,
    /// The url of the album art
    album_art_url: Option<String>,
    /// The url of the artist
    artist_url: Option<String>,
    /// The url of the artist artwork
    artist_artwork_uri: Option<String>,
    /// The url of the preview
    preview_url: Option<String>,
    /// Whether the track is a preview
    is_preview: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PluginPlaylistInfo {
    /// The type of the playlist
    #[serde(rename = "type")]
    kind: PlaylistType,
    /// The url of the playlist
    url: Option<String>,
    /// The url of the playlist artwork
    artwork_url: Option<String>,
    /// The author of the playlist
    author: Option<String>,
    /// The total number of tracks in the playlist
    total_tracks: Option<usize>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum PlaylistType {
    /// The playlist is an album
    Album,
    /// The playlist is a playlist
    Playlist,
    /// The playlist is an artist
    Artist,
    /// The playlist is a recommendations playlist
    Recommendations,
}

pub trait PluginInfo {
    type Info;
    fn get_plugin_info(&self) -> Option<&serde_json::Value>;
    fn parse_plugin_info(&self) -> Option<Self::Info>
    where
        for<'de> Self::Info: Deserialize<'de>,
    {
        let value = self.get_plugin_info()?;
        tracing::trace!(%value, "deserialising value");
        value.as_object().filter(|o| !o.is_empty())?;

        serde_json::from_value(value.clone()).ok()
    }
}

impl PluginInfo for lavalink_rs::model::track::TrackData {
    type Info = PluginTrackInfo;

    fn get_plugin_info(&self) -> Option<&serde_json::Value> {
        self.plugin_info.as_ref()
    }
}

impl PluginInfo for lavalink_rs::model::track::PlaylistData {
    type Info = PluginPlaylistInfo;

    fn get_plugin_info(&self) -> Option<&serde_json::Value> {
        self.plugin_info.as_ref()
    }
}
