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
use twilight_interactions::command::{
    AutocompleteValue, CommandModel, CommandOption, CreateCommand, CreateOption,
};
use twilight_model::{
    application::command::{Command, CommandOptionChoice, CommandOptionChoiceValue, CommandType},
    channel::{Attachment, Message},
    id::{Id, marker::GuildMarker},
};
use twilight_util::builder::command::CommandBuilder;

use crate::{
    LavalinkAware,
    command::{
        AutocompleteCtx, MessageCtx, SlashCtx,
        macros::{bad, bad_or_fol, crit_or_fol, out_or_fol, what_or_fol},
        model::{BotAutocomplete, BotMessageCommand, BotSlashCommand, GuildCtx, RespondViaMessage},
        require, util,
    },
    core::{
        r#const::{discord::COMMAND_CHOICES_LIMIT, misc::ADD_TRACKS_WRAP_LIMIT, regex},
        model::UserIdAware,
    },
    error::{
        CommandResult, LoadFailed as LoadFailedError,
        command::AutocompleteResult,
        component::queue::play::{self, LoadTrackProcessManyError, QueryError},
    },
    gateway::GuildIdAware,
    lavalink::{CorrectPlaylistInfo, CorrectTrackInfo, UnwrappedData, UnwrappedPlayerInfoUri},
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
    uri: Box<str>,
    info: PlaylistInfo,
    tracks: Box<[TrackData]>,
}

impl Playlist {
    fn new(loaded: LoadedTracks, uri: Box<str>) -> Self {
        match loaded.load_type {
            TrackLoadType::Playlist => {
                let Some(TrackLoadData::Playlist(data)) = loaded.data else {
                    panic!("loaded playlist missing playlist load data")
                };

                Self {
                    uri,
                    info: data.info,
                    tracks: data.tracks.into(),
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

impl From<LoadTrackResults> for Vec<TrackData> {
    fn from(value: LoadTrackResults) -> Self {
        value
            .0
            .into_vec()
            .into_iter()
            .flat_map(|result| match result {
                LoadTrackResult::Track(t) => Self::from([t]),
                LoadTrackResult::Playlist(p) => p.tracks.into_vec(),
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
        let title = track_info.take_and_correct_title();
        let author = track_info.take_and_correct_author();

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

impl BotAutocomplete for Autocomplete {
    async fn execute(self, ctx: AutocompleteCtx) -> AutocompleteResult {
        let mut ctx = require::guild(ctx)?;
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
            (!regex::URL.is_match(&q))
                .then(|| format!("{}{}", source.value(), q).into_boxed_str())
                .unwrap_or_else(|| q.into_boxed_str())
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

        Ok(ctx.autocomplete(choices).await?)
    }
}

async fn play(
    ctx: &mut GuildCtx<impl RespondViaMessage>,
    queries: impl IntoIterator<Item = Box<str>> + Send,
) -> Result<(), play::Error> {
    ctx.defer().await?;
    let load_ctx = LoadTrackContext::from(&*ctx);
    match load_ctx.process_many(queries).await {
        Ok(results) => {
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
                            p.info.corrected_name(),
                            p.uri
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
            let plus = match tracks_len + playlists_len {
                0 => panic!("no tracks or playlists loaded"),
                1 => "**`＋`**",
                _ => "**`≡+`**",
            };

            util::auto_join_or_check_in_voice_with_user_and_check_not_suppressed(ctx).await?;

            let total_tracks = Vec::from(results);
            let first_track = total_tracks
                .first()
                .expect("at least one track must be loaded")
                .clone();

            let player = util::auto_new_player(ctx).await?;

            player
                .data_unwrapped()
                .write()
                .await
                .queue_mut()
                .enqueue(total_tracks, ctx.user_id());
            player.play(&first_track).await?;

            out_or_fol!(format!("{} Added {}", plus, enqueued_text), ctx);
        }
        Err(e) => match e {
            LoadTrackProcessManyError::Query(query) => match query {
                QueryError::LoadFailed(LoadFailedError(query)) => {
                    crit_or_fol!(format!("Failed to load tracks for query: `{}`", query), ctx);
                }
                QueryError::NoMatches(query) => {
                    what_or_fol!(format!("No matches found for query: `{}`", query), ctx);
                }
                QueryError::SearchResult(query) => {
                    bad_or_fol!(
                        format!(
                            "Given query is not a URL: `{}`. Try using the command's autocomplete to search for tracks.",
                            query
                        ),
                        ctx
                    );
                }
            },
            LoadTrackProcessManyError::Lavalink(e) => Err(e.into()),
        },
    }
}

#[derive(CommandOption, CreateOption, Default)]
enum PlaySource {
    #[default]
    #[option(name = "Youtube", value = "ytsearch:")]
    Youtube,
    #[option(name = "Youtube Music", value = "ytmsearch:")]
    YoutubeMusic,
    #[option(name = "SoundCloud", value = "scsearch:")]
    SoundCloud,
    #[option(name = "Deezer (Search Query)", value = "dzsearch:")]
    DeezerQuery,
    #[option(name = "Deezer (ISRC)", value = "dzisrc:")]
    DeezerIsrc,
    #[option(name = "Spotify", value = "spsearch:")]
    Spotify,
}

/// Adds track(s) to the queue
#[derive(CreateCommand, CommandModel)]
#[command(name = "play", dm_permission = false)]
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

impl BotSlashCommand for Play {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
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

/// Adds track(s) from audio files to the queue
#[derive(CreateCommand, CommandModel)]
#[command(name = "play-file", dm_permission = false)]
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

impl BotSlashCommand for File {
    async fn run(self, ctx: SlashCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
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
            bad!(format!("`{}` is not an audio file.", file.filename), ctx);
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

pub struct AddToQueue;

impl AddToQueue {
    const NAME: &'static str = "➕ Add to queue";
    pub fn create_command() -> Command {
        CommandBuilder::new(Self::NAME, String::new(), CommandType::Message).build()
    }
}

impl CreateCommand for AddToQueue {
    const NAME: &'static str = Self::NAME;
    fn create_command() -> twilight_interactions::command::ApplicationCommandData {
        unreachable!()
    }
}

impl BotMessageCommand for AddToQueue {
    async fn run(ctx: MessageCtx) -> CommandResult {
        let mut ctx = require::guild(ctx)?;
        let message = ctx.target_message();

        let queries = extract_queries(message);
        if queries.is_empty() {
            bad!("No audio files or URLs found in this message.", ctx);
        }
        Ok(play(&mut ctx, queries).await?)
    }
}
