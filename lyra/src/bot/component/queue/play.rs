use chrono::Duration;
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
use twilight_interactions::command::{
    AutocompleteValue, CommandModel, CommandOption, CreateCommand, CreateOption,
};
use twilight_model::{
    application::command::{Command, CommandOptionChoice, CommandOptionChoiceValue, CommandType},
    channel::{Attachment, Message},
    id::{marker::GuildMarker, Id},
};
use twilight_util::builder::command::CommandBuilder;

use crate::bot::{
    command::{
        macros::{bad, crit, hid, out_or_fol, what},
        model::{BotAutocomplete, BotMessageCommand, BotSlashCommand, Ctx, RespondViaMessage},
        util, AutocompleteCtx, MessageCtx, SlashCtx,
    },
    core::r#const::{discord::COMMAND_CHOICES_LIMIT, misc::ADD_TRACKS_WRAP_LIMIT, regex},
    error::{
        command::{AutocompleteResult, Result as CommandResult},
        component::queue::play::{self, LoadTrackProcessManyError, QueryError},
        LoadFailed as LoadFailedError,
    },
    ext::util::{PrettifiedTimestamp, PrettyJoiner, PrettyTruncator, ViaGrapheme},
    gateway::ExpectedGuildIdAware,
    lavalink::{CorrectPlaylistInfo, CorrectTrackInfo, DelegateMethods, LavalinkAware},
};

struct LoadTrackContext {
    guild_id: Id<GuildMarker>,
    lavalink: LavalinkClient,
}

impl LoadTrackContext {
    fn new_via(ctx: &(impl ExpectedGuildIdAware + LavalinkAware)) -> Self {
        Self {
            guild_id: ctx.guild_id(),
            lavalink: ctx.lavalink().clone_inner(),
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
                        unreachable!()
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
                    unreachable!()
                };

                Self {
                    uri,
                    info: data.info,
                    tracks: data.tracks.into(),
                }
            }
            _ => panic!("`loaded.load_type` not `LoadType::PlaylistLoaded`"),
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

        let track_length =
            PrettifiedTimestamp::from(Duration::milliseconds(track_info.length as i64));
        let title = track_info.take_and_correct_title();
        let author = track_info.take_and_correct_author();

        format!(
            "⌛{} 👤{} 🎵{}",
            track_length,
            author.pretty_truncate(15),
            title.pretty_truncate(55)
        )
    }
}

impl AutocompleteResultPrettify for LoadedTracks {
    fn prettify(&mut self) -> String {
        let Some(TrackLoadData::Playlist(ref mut data)) = self.data else {
            unreachable!()
        };

        let name = data.info.take_and_correct_name();
        let track_length = PrettifiedTimestamp::from(Duration::milliseconds(
            data.tracks.iter().map(|t| t.info.length as i64).sum(),
        ));
        let track_count = data.tracks.len();

        format!(
            "📚{} tracks ⌛{} 🎵{}",
            track_count,
            track_length,
            name.pretty_truncate(80)
        )
    }
}

impl BotAutocomplete for Autocomplete {
    async fn execute(self, mut ctx: AutocompleteCtx) -> AutocompleteResult {
        let query = [
            self.query,
            self.query_2,
            self.query_3,
            self.query_4,
            self.query_5,
        ]
        .into_iter()
        .find_map(|q| match q {
            AutocompleteValue::Focused(q) => Some(q.into_boxed_str()),
            _ => None,
        })
        .map(|q| {
            let source = self.source.unwrap_or_default();
            (!regex::URL.is_match(&q))
                .then(|| format!("{}search:{}", source.value(), q).into_boxed_str())
                .unwrap_or(q)
        })
        .expect("exactly one option is focused");

        let guild_id = ctx.guild_id();
        let load_ctx = LoadTrackContext {
            guild_id,
            lavalink: ctx.lavalink().clone_inner(),
        };

        let mut loaded = load_ctx.process(&query).await?;
        let choices = match loaded.load_type {
            TrackLoadType::Search => {
                let Some(TrackLoadData::Search(tracks)) = loaded.data else {
                    unreachable!()
                };

                tracks
                    .into_iter()
                    .map(|mut t| CommandOptionChoice {
                        name: t.prettify(),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String(
                            t.info.uri.expect("track is nonlocal"),
                        ),
                    })
                    .take(COMMAND_CHOICES_LIMIT)
                    .collect()
            }
            TrackLoadType::Track => {
                let Some(TrackLoadData::Track(mut track)) = loaded.data else {
                    unreachable!()
                };

                vec![CommandOptionChoice {
                    name: track.prettify(),
                    name_localizations: None,
                    value: CommandOptionChoiceValue::String(
                        track.info.uri.expect("track is nonlocal"),
                    ),
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
                        value: CommandOptionChoiceValue::String(
                            track.info.uri.expect("track is nonlocal"),
                        ),
                    });
                }

                choices
            }
            TrackLoadType::Error => Err(LoadFailedError(query))?,
            TrackLoadType::Empty => Vec::new(),
        };

        Ok(ctx.autocomplete(choices).await?)
    }
}

async fn play(
    ctx: &mut Ctx<impl RespondViaMessage>,
    queries: impl IntoIterator<Item = Box<str>> + Send,
) -> Result<(), play::Error> {
    let guild_id = ctx.guild_id();

    let load_ctx = LoadTrackContext::new_via(ctx);
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
                            t.info.uri.as_ref().expect("track is nonlocal")
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
                0 => unreachable!(),
                1 => "**`＋`**",
                _ => "**`≡+`**",
            };

            util::auto_join_or_check_in_voice_with_user_and_check_not_suppressed(ctx).await?;

            let total_tracks = Vec::from(results);
            let first_track = total_tracks.first().expect("results is non-empty").clone();

            util::auto_new_player_data(ctx).await?;

            ctx.lavalink()
                .player_data(guild_id)
                .write()
                .await
                .queue_mut()
                .enqueue(total_tracks, ctx.author_id());

            ctx.lavalink().player(guild_id).play(&first_track).await?;

            out_or_fol!(format!("{} Added {}", plus, enqueued_text), ctx);
        }
        Err(e) => match e {
            LoadTrackProcessManyError::Query(query) => match query {
                QueryError::LoadFailed(LoadFailedError(query)) => {
                    crit!(format!("Failed to load tracks for query: `{}`", query), ctx);
                }
                QueryError::NoMatches(query) => {
                    what!(format!("No matches found for query: `{}`", query), ctx);
                }
                QueryError::SearchResult(query) => {
                    bad!(
                        format!(
                            "Given query is not a URL: `{}`. Try using the command's autocomplete to search for tracks.",
                            query
                        ),
                        ctx
                    );
                }
            },
            LoadTrackProcessManyError::Lavalink(e) => Err(e)?,
        },
    }
}

#[derive(CommandOption, CreateOption, Default)]
enum PlaySource {
    #[default]
    #[option(name = "Deezer", value = "dz")]
    Deezer,
    #[option(name = "Youtube", value = "yt")]
    Youtube,
    #[option(name = "Youtube Music", value = "ytm")]
    YoutubeMusic,
    #[option(name = "SoundCloud", value = "sc")]
    SoundCloud,
    #[option(name = "Spotify", value = "sp")]
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
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
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
    async fn run(self, mut ctx: SlashCtx) -> CommandResult {
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
    pub fn create_command() -> Command {
        CommandBuilder::new("➕ Add to queue", "", CommandType::Message).build()
    }
}

impl BotMessageCommand for AddToQueue {
    async fn run(mut ctx: MessageCtx) -> CommandResult {
        let message = ctx.target_message();

        let queries = extract_queries(message);
        if queries.is_empty() {
            bad!("No audio files or URLs found in this message.", ctx);
        };
        Ok(play(&mut ctx, queries).await?)
    }
}
