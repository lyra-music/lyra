use std::time::Duration;

use futures::future;
use itertools::{Either, Itertools};
use lavalink_rs::{
    client::LavalinkClient,
    error::LavalinkResult,
    model::track::{
        PlaylistData, PlaylistInfo, Track as LoadedTracks, TrackData, TrackLoadData, TrackLoadType,
    },
};
use linkify::{LinkFinder, LinkKind};
use lyra_ext::{
    as_grapheme::AsGrapheme,
    pretty::{duration_display::DurationDisplay, join::PrettyJoiner, truncate::PrettyTruncator},
};
use twilight_interactions::command::{AutocompleteValue, CommandModel, CreateCommand};
use twilight_model::{
    application::{
        command::{Command, CommandOptionChoice, CommandOptionChoiceValue, CommandType},
        interaction::InteractionContextType,
    },
    channel::{Attachment, Message},
    id::{Id, marker::GuildMarker},
};
use twilight_util::builder::command::CommandBuilder;

use crate::{
    LavalinkAware,
    command::{
        common::PlaySource,
        model::{
            BotGuildAutocomplete, BotGuildMessageCommand, BotGuildSlashCommand, FollowupKind,
            GuildAutocompleteCtx, GuildCtx, GuildMessageCmdCtx, GuildSlashCmdCtx,
            RespondWithDeferKind, RespondWithMessageKind,
        },
        require, util,
    },
    core::{
        konst::{discord::COMMAND_CHOICES_LIMIT, misc::ADD_TRACKS_WRAP_LIMIT, regex},
        model::{
            UserIdAware,
            response::{
                either::RespondOrFollowup,
                initial::{
                    autocomplete::RespondAutocomplete, defer::RespondWithDefer,
                    message::create::RespondWithMessage,
                },
            },
        },
    },
    error::{
        CommandResult, LoadFailed as LoadFailedError,
        command::AutocompleteResult,
        component::queue::play::{self, LoadTrackProcessManyError, QueryError},
    },
    gateway::GuildIdAware,
    lavalink::{
        CorrectPlaylistInfo, CorrectTrackInfo, PlaylistAwareTrackData, PlaylistMetadata,
        UnwrappedData, UnwrappedPlayerInfoUri, make_playlist_aware,
    },
};

struct LoadTrackContext {
    guild_id: Id<GuildMarker>,
    lavalink: LavalinkClient,
}

impl<T> From<&T> for LoadTrackContext
where
    T: GuildIdAware + LavalinkAware,
{
    fn from(value: &T) -> Self {
        Self {
            guild_id: value.guild_id(),
            lavalink: value.lavalink().clone_inner(),
        }
    }
}

impl LoadTrackContext {
    async fn process(&self, query: &str) -> LavalinkResult<LoadedTracks> {
        self.lavalink.load_tracks(self.guild_id, query).await
    }

    async fn process_many(
        &self,
        queries: impl IntoIterator<Item = Box<str>> + Send,
    ) -> Result<LoadTrackResults, LoadTrackProcessManyError> {
        let queries = queries.into_iter().map(|query| async move {
            let loaded = self.process(&query).await?;
            match loaded.load_type {
                TrackLoadType::Track => {
                    let Some(TrackLoadData::Track(t)) = loaded.data else {
                        panic!("loaded track missing track load data")
                    };
                    Ok(LoadTrackResult::Track(t))
                }
                TrackLoadType::Playlist => {
                    Ok(LoadTrackResult::Playlist(Playlist::new(loaded, query)))
                }
                TrackLoadType::Search => Err(LoadTrackProcessManyError::Query(
                    QueryError::SearchResult(query),
                )),
                TrackLoadType::Empty => Err(LoadTrackProcessManyError::Query(
                    QueryError::NoMatches(query),
                )),
                TrackLoadType::Error => Err(LoadTrackProcessManyError::Query(
                    QueryError::LoadFailed(LoadFailedError(query)),
                )),
            }
        });

        let results = future::try_join_all(queries).await?;
        Ok(LoadTrackResults(results.into()))
    }
}

struct Playlist {
    metadata: PlaylistMetadata,
    tracks: Box<[TrackData]>,
}

impl Playlist {
    fn new(loaded: LoadedTracks, uri: Box<str>) -> Self {
        match loaded.load_type {
            TrackLoadType::Playlist => {
                let Some(TrackLoadData::Playlist(mut data)) = loaded.data else {
                    panic!("loaded playlist missing playlist load data")
                };

                let tracks = std::mem::take(&mut data.tracks).into();
                Self {
                    metadata: PlaylistMetadata::new(uri, data),
                    tracks,
                }
            }
            _ => panic!("`loaded resources not a playlist"),
        }
    }
}

#[must_use]
enum LoadTrackResult {
    Track(TrackData),
    Playlist(Playlist),
}

#[must_use]
struct LoadTrackResults(Box<[LoadTrackResult]>);

impl LoadTrackResults {
    fn split(&self) -> (Vec<&TrackData>, Vec<&Playlist>) {
        let (tracks, playlists): (Vec<_>, Vec<_>) =
            self.0.iter().partition_map(|result| match result {
                LoadTrackResult::Track(track) => Either::Left(track),
                LoadTrackResult::Playlist(playlist) => Either::Right(playlist),
            });

        (tracks, playlists)
    }
}

impl From<LoadTrackResults> for Vec<PlaylistAwareTrackData> {
    fn from(value: LoadTrackResults) -> Self {
        value
            .0
            .into_vec()
            .into_iter()
            .flat_map(|result| match result {
                LoadTrackResult::Track(t) => Self::from([t.into()]),
                LoadTrackResult::Playlist(p) => make_playlist_aware(p.tracks, p.metadata),
            })
            .collect()
    }
}

#[derive(CommandModel)]
#[command(autocomplete = true)]
pub struct Autocomplete {
    query: AutocompleteValue<String>,
    query_2: AutocompleteValue<String>,
    query_3: AutocompleteValue<String>,
    query_4: AutocompleteValue<String>,
    query_5: AutocompleteValue<String>,
    source: Option<PlaySource>,
}

trait AutocompleteResultPrettify {
    fn prettify(&mut self) -> String;
}

impl AutocompleteResultPrettify for TrackData {
    fn prettify(&mut self) -> String {
        let track_info = &mut self.info;

        let track_length = Duration::from_millis(track_info.length);
        let author = track_info.take_and_correct_author();
        let title = track_info.take_and_correct_title();

        format!(
            "⌛{} 👤{} 🎵{}",
            track_length.pretty_display(),
            author.pretty_truncate(15),
            title.pretty_truncate(55)
        )
    }
}

impl AutocompleteResultPrettify for LoadedTracks {
    fn prettify(&mut self) -> String {
        let Some(TrackLoadData::Playlist(ref mut data)) = self.data else {
            panic!("loaded searches missing playlist search data")
        };

        let name = data.info.take_and_correct_name();
        let track_length = Duration::from_millis(data.tracks.iter().map(|t| t.info.length).sum());
        let track_count = data.tracks.len();

        format!(
            "📚{} tracks ⌛{} 🎵{}",
            track_count,
            track_length.pretty_display(),
            name.pretty_truncate(80)
        )
    }
}

impl BotGuildAutocomplete for Autocomplete {
    async fn execute(self, mut ctx: GuildAutocompleteCtx) -> AutocompleteResult {
        let query = [
            self.query,
            self.query_2,
            self.query_3,
            self.query_4,
            self.query_5,
        ]
        .into_iter()
        .find_map(|q| match q {
            AutocompleteValue::Focused(q) => Some(q),
            _ => None,
        })
        .map(|q| {
            let source = self.source.unwrap_or_default();
            if regex::URL.is_match(&q) {
                q.into_boxed_str()
            } else {
                format!("{}:{}", source.value(), q).into_boxed_str()
            }
        })
        .expect("exactly one autocomplete option should be focused");

        let guild_id = ctx.guild_id();
        let load_ctx = LoadTrackContext {
            guild_id,
            lavalink: ctx.lavalink().clone_inner(),
        };

        let mut loaded = load_ctx.process(&query).await?;
        let choices = match loaded.load_type {
            TrackLoadType::Search => {
                let Some(TrackLoadData::Search(tracks)) = loaded.data else {
                    panic!("loaded searches missing playlist search data")
                };

                tracks
                    .into_iter()
                    .map(|mut t| CommandOptionChoice {
                        name: t.prettify(),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String(t.info.into_uri_unwrapped()),
                    })
                    .take(COMMAND_CHOICES_LIMIT)
                    .collect()
            }
            TrackLoadType::Track => {
                let Some(TrackLoadData::Track(mut track)) = loaded.data else {
                    panic!("loaded track missing track load data")
                };

                vec![CommandOptionChoice {
                    name: track.prettify(),
                    name_localizations: None,
                    value: CommandOptionChoiceValue::String(track.info.into_uri_unwrapped()),
                }]
            }
            TrackLoadType::Playlist => {
                let mut choices = vec![CommandOptionChoice {
                    name: loaded.prettify(),
                    name_localizations: None,
                    value: CommandOptionChoiceValue::String(
                        query.grapheme_truncate(100).to_string(),
                    ),
                }];

                if let Some(TrackLoadData::Playlist(PlaylistData {
                    info:
                        PlaylistInfo {
                            selected_track: Some(selected),
                            ..
                        },
                    mut tracks,
                    ..
                })) = loaded.data
                {
                    let mut track = tracks.swap_remove(selected as usize);
                    choices.push(CommandOptionChoice {
                        name: track.prettify(),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String(track.info.into_uri_unwrapped()),
                    });
                }

                choices
            }
            TrackLoadType::Error => return Err(LoadFailedError(query).into()),
            TrackLoadType::Empty => Vec::new(),
        };

        ctx.autocomplete(choices).await?;
        Ok(())
    }
}

async fn play(
    ctx: &mut GuildCtx<impl RespondWithMessageKind + FollowupKind + RespondWithDeferKind>,
    queries: impl IntoIterator<Item = Box<str>> + Send,
) -> Result<(), play::Error> {
    ctx.defer().await?;
    let load_ctx = LoadTrackContext::from(&*ctx);
    match load_ctx.process_many(queries).await {
        Ok(results) => Ok(handle_load_track_results(ctx, results).await?),
        Err(e) => match e {
            LoadTrackProcessManyError::Query(query) => match query {
                QueryError::LoadFailed(LoadFailedError(query)) => {
                    ctx.unkn_f(format!("Failed to load tracks for query: `{query}`."))
                        .await?;
                    Ok(())
                }
                QueryError::SearchResult(query) | QueryError::NoMatches(query) => {
                    ctx.wrng_f(
                        format!(
                            "**Given query is not a URL: `{query}`**; Use the command's autocomplete to search for tracks instead. \n\
                            -# If the autocomplete results are empty, try using a different search query.",
                        ),
                    ).await?;
                    Ok(())
                }
            },
            LoadTrackProcessManyError::Lavalink(e) => Err(e.into()),
        },
    }
}

async fn handle_load_track_results(
    ctx: &mut GuildCtx<impl RespondWithMessageKind + FollowupKind>,
    results: LoadTrackResults,
) -> Result<(), play::HandleLoadTrackResultsError> {
    let (tracks, playlists) = results.split();
    let tracks_len = tracks.len();
    let track_text = match tracks_len {
        0 => String::new(),
        1..=ADD_TRACKS_WRAP_LIMIT => tracks
            .iter()
            .map(|t| {
                format!(
                    "[`{}`](<{}>)",
                    t.info.corrected_title(),
                    t.info.uri_unwrapped()
                )
            })
            .collect::<Vec<_>>()
            .pretty_join_with_and(),
        _ => format!("`{tracks_len} tracks`"),
    };
    let playlists_len = playlists.len();
    let playlist_text = match playlists_len {
        0 => String::new(),
        1..=ADD_TRACKS_WRAP_LIMIT => playlists
            .iter()
            .map(|p| {
                format!(
                    "`{} tracks` from playlist [`{}`](<{}>)",
                    p.tracks.len(),
                    p.metadata.corrected_name(),
                    p.metadata.uri
                )
            })
            .collect::<Vec<_>>()
            .pretty_join_with_and(),
        _ => format!(
            "`{} tracks` in total from `{} playlists`",
            playlists.iter().fold(0, |l, p| l + p.tracks.len()),
            playlists_len
        ),
    };
    let enqueued_text = [track_text, playlist_text]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .pretty_join_with_and();
    util::auto_join_or_check_in_voice_with_user_and_check_not_suppressed(ctx).await?;
    let total_tracks = Vec::from(results);
    let total_tracks_len = total_tracks.len();
    let plus = match total_tracks_len {
        0 => panic!("no tracks or playlists loaded"),
        1 => "**`＋`**",
        _ => "**`≡+`**",
    };
    let first_track = total_tracks
        .first()
        .expect("at least one track must be loaded")
        .clone();
    let player = util::auto_new_player(ctx).await?;
    let data = player.data_unwrapped();
    let (now_playing_msg_exists, queue_len) = {
        let data_r = data.read().await;
        let queue = data_r.queue();
        let pair = (require::current_track(queue).is_ok(), queue.len());
        drop(data_r);
        pair
    };
    if now_playing_msg_exists {
        data.write()
            .await
            .update_and_apply_now_playing_queue_len(queue_len + total_tracks_len)
            .await?;
    }
    data.write()
        .await
        .queue_mut()
        .enqueue(total_tracks, ctx.user_id());
    ctx.out_f(format!("{plus} Added {enqueued_text}.")).await?;
    player.play(first_track.inner()).await?;
    Ok(())
}

/// Adds track(s) to the queue.
#[derive(CreateCommand, CommandModel)]
#[command(name = "play", contexts = "guild")]
pub struct Play {
    /// What song? [search query / direct link]
    #[command(autocomplete = true)]
    query: String,
    /// What song? [search query / direct link] (2)
    #[command(autocomplete = true)]
    query_2: Option<String>,
    /// What song? [search query / direct link] (3)
    #[command(autocomplete = true)]
    query_3: Option<String>,
    /// What song? [search query / direct link] (4)
    #[command(autocomplete = true)]
    query_4: Option<String>,
    /// What song? [search query / direct link] (5)
    #[command(autocomplete = true)]
    query_5: Option<String>,
    /// Search from where? (if not given, Youtube)
    #[command(rename = "source")]
    _source: Option<PlaySource>,
}

impl BotGuildSlashCommand for Play {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let queries = [
            Some(self.query),
            self.query_2,
            self.query_3,
            self.query_4,
            self.query_5,
        ]
        .into_iter()
        .flatten()
        .map(String::into_boxed_str);

        Ok(play(&mut ctx, queries).await?)
    }
}

/// Adds track(s) from audio files to the queue.
#[derive(CreateCommand, CommandModel)]
#[command(name = "play-file", contexts = "guild")]
pub struct File {
    /// What track?
    track: Attachment,
    /// What track? (2)
    track_2: Option<Attachment>,
    /// What track? (3)
    track_3: Option<Attachment>,
    /// What track? (4)
    track_4: Option<Attachment>,
    /// What track? (5)
    track_5: Option<Attachment>,
}

impl BotGuildSlashCommand for File {
    async fn run(self, mut ctx: GuildSlashCmdCtx) -> CommandResult {
        let files = [
            Some(self.track),
            self.track_2,
            self.track_3,
            self.track_4,
            self.track_5,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if let Some(file) = files.iter().find(|f| {
            f.content_type
                .as_ref()
                .is_some_and(|ty| !ty.starts_with("audio"))
        }) {
            ctx.wrng(format!("`{}` is not an audio file.", file.filename))
                .await?;
            return Ok(());
        }

        let urls = files.into_iter().map(|f| f.url.into());
        Ok(play(&mut ctx, urls).await?)
    }
}

fn extract_queries(message: &Message) -> Vec<Box<str>> {
    let mut link_finder = LinkFinder::new();
    link_finder.kinds(&[LinkKind::Url]);

    let content_queries = link_finder
        .links(&message.content)
        .map(|m| m.as_str().into());
    let audio_files = message
        .attachments
        .iter()
        .filter(|&f| {
            f.content_type
                .as_ref()
                .is_some_and(|ty| ty.starts_with("audio"))
        })
        .map(|f| f.url.clone().into());

    content_queries.chain(audio_files).collect()
}

// TODO: make a derive macro like `twilight_interactions::CreateCommand` for
// more menu commands in the future. It may look like:
//
// ```rust
// #[derive(CreateMessageCommand)]
// #[command(name = "➕ Add to queue", contexts = "guild")]
// pub struct struct AddToQueue;
// ```
pub struct AddToQueue;

impl AddToQueue {
    pub const NAME: &'static str = "➕ Add to queue";
    pub fn create_command() -> Command {
        CommandBuilder::new(Self::NAME, String::new(), CommandType::Message)
            .contexts(std::iter::once(InteractionContextType::Guild))
            .build()
    }
}

impl BotGuildMessageCommand for AddToQueue {
    async fn run(mut ctx: GuildMessageCmdCtx) -> CommandResult {
        let message = ctx.target_message();

        let queries = extract_queries(message);
        if queries.is_empty() {
            ctx.wrng("No audio files or URLs found in this message.")
                .await?;
            return Ok(());
        }
        Ok(play(&mut ctx, queries).await?)
    }
}
