use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use lavalink_rs::{
    client::LavalinkClient,
    model::{events::TrackStart, GuildId},
};
use lyra_ext::{
    image::dominant_palette, num::u64_to_i64_truncating, pretty::duration_display::DurationDisplay,
};
use twilight_cache_inmemory::{InMemoryCache, Reference};
use twilight_mention::{timestamp::TimestampStyle, Mention};
use twilight_model::{
    channel::message::Embed,
    id::{marker::UserMarker, Id},
    user::User,
};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::{
    command::util::{AvatarUrlAware, DefaultAvatarUrlAware, GuildAvatarUrlAware},
    core::model::{CacheAware, DatabaseAware, HttpAware},
    error::{
        lavalink::{GenerateNowPlayingEmbedError, GetDominantPaletteFromUrlError, ProcessResult},
        Cache,
    },
    lavalink::{
        model::ArtworkCache, CorrectTrackInfo, PluginInfo, QueueItem, UnwrappedData,
        UnwrappedPlayerInfoUri,
    },
};

#[tracing::instrument(err, skip_all, name = "track_start")]
pub(super) async fn impl_start(
    lavalink: LavalinkClient,
    _: String,
    event: &TrackStart,
) -> ProcessResult {
    let guild_id = event.guild_id;
    tracing::debug!(
        "guild {} started {:?}",
        event.guild_id.0,
        event.track.info.checked_title()
    );

    let Some(player) = lavalink.get_player_context(guild_id) else {
        tracing::error!(?guild_id, "track started without player");

        return Ok(());
    };
    player
        .data_unwrapped()
        .write()
        .await
        .reset_track_timestamp();

    let data = player.data_unwrapped();
    let data_r = data.read().await;
    let queue = data_r.queue();
    let Some(track) = queue.current() else {
        return Ok(());
    };

    let lavalink_data = lavalink.data_unwrapped();
    let rec = sqlx::query!(
        "SELECT now_playing FROM guild_configs WHERE id = $1;",
        u64_to_i64_truncating(guild_id.0)
    )
    .fetch_one(lavalink_data.db())
    .await?;

    if !rec.now_playing {
        return Ok(());
    }

    let embed = generate_now_playing_embed(
        lavalink_data.cache(),
        lavalink_data.artwork_cache(),
        guild_id,
        track,
        queue.len(),
        queue.position(),
        data_r.speed(),
    )
    .await?;
    let req = lavalink_data
        .http()
        .create_message(data_r.now_playing_message_channel_id())
        .content("🎵 **Now Playing**")
        .embeds(&[embed])
        .await?;
    let message_id = req.model().await?.id;

    drop(data_r);
    data.write().await.set_now_playing_message_id(message_id);
    Ok(())
}

async fn generate_now_playing_embed(
    cache: &InMemoryCache,
    artwork_cache: &ArtworkCache,
    guild_id: GuildId,
    track: &QueueItem,
    queue_len: usize,
    queue_position: NonZeroUsize,
    speed: f64,
) -> Result<Embed, GenerateNowPlayingEmbedError> {
    let requester_id = track.requester();
    let track_data = track.data();
    let track_info = &track_data.info;
    let twilight_guild_id = Id::new(guild_id.0);
    let requester = cache.member(twilight_guild_id, requester_id).ok_or(Cache)?;
    let plugin_info = track_data.parse_plugin_info();

    let description = {
        let duration = Duration::from_millis(track_info.length);
        let duration_left = twilight_mention::timestamp::Timestamp::new(
            (lyra_ext::unix_time() + duration.div_f64(speed)).as_secs(),
            Some(TimestampStyle::RelativeTime),
        );
        let album_info = plugin_info
            .as_ref()
            .and_then(|info| info.album_name.as_ref())
            .map_or_else(String::new, |name| format!("📀 **{name}**\n"));
        format!(
            "{}#️⃣ **{}** / {} ⏳ {} / {}",
            album_info,
            queue_position,
            queue_len,
            duration_left.mention(),
            duration.pretty_display()
        )
    };

    #[allow(clippy::cast_possible_truncation)]
    let timestamp =
        twilight_model::util::Timestamp::from_micros(track.enqueued().as_micros() as i64)?;

    let footer = {
        let (requester_name, requester_avatar) = {
            type CachedUserRef<'a> = Reference<'a, Id<UserMarker>, User>;
            let get_user = || cache.user(requester_id).ok_or(Cache);
            let get_display_name = |user: &CachedUserRef| {
                user.global_name
                    .as_deref()
                    .unwrap_or(user.name.as_str())
                    .to_owned()
            };
            let get_display_avatar = |user: &CachedUserRef| {
                user.avatar_url()
                    .unwrap_or_else(|| user.default_avatar_url())
            };
            match (requester.nick(), requester.avatar_url(twilight_guild_id)) {
                (Some(nick), Some(url)) => (nick.to_owned(), url),
                (Some(nick), None) => {
                    let user = get_user()?;
                    (nick.to_owned(), get_display_avatar(&user))
                }
                (None, Some(url)) => {
                    let user = get_user()?;
                    (get_display_name(&user), url)
                }
                (None, None) => {
                    let user = get_user()?;
                    (get_display_name(&user), get_display_avatar(&user))
                }
            }
        };

        EmbedFooterBuilder::new(requester_name)
            .icon_url(ImageSource::url(requester_avatar)?)
            .build()
    };

    let mut author = EmbedAuthorBuilder::new(track_info.author.clone());
    if let Some(url) = plugin_info
        .as_ref()
        .and_then(|info| info.artist_url.as_ref())
    {
        author = author.url(url);
    }
    if let Some(url) = plugin_info
        .as_ref()
        .and_then(|info| info.artist_artwork_url.as_ref())
    {
        author = author.icon_url(ImageSource::url(url)?);
    }

    let mut embed = EmbedBuilder::new()
        .title(track_info.title.clone())
        .url(track_info.uri_unwrapped())
        .description(description)
        .timestamp(timestamp)
        .author(author.build())
        .footer(footer);

    let artwork_url = plugin_info
        .as_ref()
        .and_then(|info| info.album_art_url.as_ref())
        .or(track_info.artwork_url.as_ref());
    if let Some(url) = artwork_url {
        let dominant_palette = get_dominant_palette_from_url(artwork_cache, url, 4).await?;
        embed = embed
            .color(dominant_palette[0])
            .thumbnail(ImageSource::url(url)?);
    }
    Ok(embed.build())
}

pub async fn get_dominant_palette_from_url(
    cache: &ArtworkCache,
    url: &str,
    palette_size: usize,
) -> Result<Arc<[u32]>, Arc<GetDominantPaletteFromUrlError>> {
    let key = (Box::from(url), palette_size);
    cache
        .try_get_with(key, async {
            let bytes = reqwest::get(url).await?.bytes().await?;
            let palette = dominant_palette::from_bytes(&bytes, palette_size)?;
            Ok(dominant_palette::normalise(palette).into())
        })
        .await
}
