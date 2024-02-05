use std::{mem, sync::Arc};

use chrono::Duration;
use futures::future;
use hyper::{Body, Request};
use itertools::{Either, Itertools};
use linkify::{LinkFinder, LinkKind};
use twilight_interactions::command::{
    AutocompleteValue, CommandModel, CommandOption, CreateCommand, CreateOption,
};
use twilight_lavalink::{
    http::{LoadType, LoadedTracks, PlaylistInfo, Track},
    player::Player,
};
use twilight_model::{
    application::command::{Command, CommandOptionChoice, CommandOptionChoiceValue, CommandType},
    channel::{Attachment, Message},
};
use twilight_util::builder::command::CommandBuilder;

use crate::bot::{
    command::{
        macros::{bad, crit, hid, out_or_fol, what},
        model::{
            AutocompleteCtx, BotAutocomplete, BotMessageCommand, BotSlashCommand, MessageCommand,
            RespondViaMessage, SlashCommand,
        },
        util::auto_join_or_check_in_voice_with_user_and_check_not_suppressed,
        Ctx,
    },
    core::{
        model::{HyperAware, HyperClient, OwnedBotState, OwnedBotStateAware},
        r#const::{
            discord::COMMAND_CHOICES_LIMIT,
            misc::ADD_TRACKS_WRAP_LIMIT,
            regex,
            text::{UNKNOWN_ARTIST, UNNAMED_PLAYLIST, UNTITLED_TRACK},
        },
    },
    error::{
        command::{AutocompleteResult, Result as CommandResult},
        component::queue::play::{
            self, LoadTrackProcessError, LoadTrackProcessManyError, QueryError,
            UnknownLoadTypeError,
        },
        LoadFailed as LoadFailedError,
    },
    ext::util::{PrettifiedTimestamp, PrettyJoiner, PrettyTruncator, ViaGrapheme},
    gateway::ExpectedGuildIdAware,
    lavalink::ClientAware,
};

struct LoadTrackContext {
    player: Arc<Player>,
    inner: OwnedBotState,
}

impl LoadTrackContext {
    fn new(player: Arc<Player>, ctx: &impl OwnedBotStateAware) -> Self {
        Self {
            inner: ctx.bot_owned(),
            player,
        }
    }

    fn hyper(&self) -> &HyperClient {
        self.inner.hyper()
    }

    async fn process(&self, query: &str) -> Result<LoadedTracks, LoadTrackProcessError> {
        let node_config = self.player.node().config();
        let (parts, body) = twilight_lavalink::http::load_track(
            node_config.address,
            query,
            node_config.authorization.as_str(),
        )?
        .into_parts();
        let req = Request::from_parts(parts, Body::from(body));
        let resp = self.hyper().request(req).await?;
        let response_bytes = hyper::body::to_bytes(resp.into_body()).await?;

        let loaded = serde_json::from_slice::<LoadedTracks>(&response_bytes)?;
        Ok(loaded)
    }

    async fn process_many(
        &self,
        queries: impl IntoIterator<Item = Box<str>> + Send,
    ) -> Result<LoadTrackResults, LoadTrackProcessManyError> {
        let queries = queries.into_iter().map(|query| async move {
            let mut loaded = self.process(&query).await?;
            match loaded.load_type {
                LoadType::TrackLoaded => {
                    let track = loaded.tracks.swap_remove(0);
                    Ok(LoadTrackResult::Track(track))
                }
                LoadType::PlaylistLoaded => {
                    Ok(LoadTrackResult::Playlist(Playlist::new(loaded, query)))
                }
                LoadType::NoMatches => Err(LoadTrackProcessManyError::Query(
                    QueryError::NoMatches(query),
                )),
                LoadType::SearchResult => Err(LoadTrackProcessManyError::Query(
                    QueryError::SearchResult(query),
                )),
                LoadType::LoadFailed => Err(LoadTrackProcessManyError::Query(
                    QueryError::LoadFailed(LoadFailedError(query)),
                )),
                unknown => Err(UnknownLoadTypeError(unknown))?,
            }
        });

        let results = future::try_join_all(queries).await?;
        Ok(LoadTrackResults(results.into()))
    }
}

struct Playlist {
    uri: Box<str>,
    info: PlaylistInfo,
    tracks: Box<[Track]>,
}

impl Playlist {
    fn new(loaded: LoadedTracks, uri: Box<str>) -> Self {
        match loaded.load_type {
            LoadType::PlaylistLoaded => Self {
                uri,
                info: loaded.playlist_info,
                tracks: loaded.tracks.into(),
            },
            _ => panic!("`loaded.load_type` must be `LoadType::PlaylistLoaded`"),
        }
    }
}

#[must_use]
enum LoadTrackResult {
    Track(Track),
    Playlist(Playlist),
}

#[must_use]
struct LoadTrackResults(Box<[LoadTrackResult]>);

impl LoadTrackResults {
    fn split(&self) -> (Vec<&Track>, Vec<&Playlist>) {
        let (tracks, playlists): (Vec<_>, Vec<_>) =
            self.0.iter().partition_map(|result| match result {
                LoadTrackResult::Track(track) => Either::Left(track),
                LoadTrackResult::Playlist(playlist) => Either::Right(playlist),
            });

        (tracks, playlists)
    }
}

impl From<LoadTrackResults> for Vec<Track> {
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

impl AutocompleteResultPrettify for Track {
    fn prettify(&mut self) -> String {
        let track_info = &mut self.info;

        let track_length =
            PrettifiedTimestamp::from(Duration::milliseconds(track_info.length as i64));
        let title = mem::take(&mut track_info.title);
        let author = mem::take(&mut track_info.author);

        format!(
            "⌛{} 👤{} 🎵{}",
            track_length,
            author
                .as_deref()
                .unwrap_or(UNKNOWN_ARTIST)
                .pretty_truncate(15),
            title
                .as_deref()
                .unwrap_or(UNTITLED_TRACK)
                .pretty_truncate(55)
        )
    }
}

impl AutocompleteResultPrettify for LoadedTracks {
    fn prettify(&mut self) -> String {
        let name = mem::take(&mut self.playlist_info.name);
        let track_length = PrettifiedTimestamp::from(Duration::milliseconds(
            self.tracks.iter().map(|t| t.info.length as i64).sum(),
        ));
        let track_count = self.tracks.len();

        format!(
            "📚{} tracks ⌛{} 🎵{}",
            track_count,
            track_length,
            name.as_deref()
                .unwrap_or(UNNAMED_PLAYLIST)
                .pretty_truncate(80)
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
        .expect("at least one option must be focused");

        let guild_id = ctx.guild_id_expected();
        let player = ctx.lavalink().player(guild_id).await?;

        let load_ctx = LoadTrackContext::new(player, &ctx);

        let mut loaded = load_ctx.process(&query).await?;
        let choices = match loaded.load_type {
            LoadType::SearchResult => loaded
                .tracks
                .into_iter()
                .map(|mut t| CommandOptionChoice {
                    name: t.prettify(),
                    name_localizations: None,
                    value: CommandOptionChoiceValue::String(t.info.uri),
                })
                .take(COMMAND_CHOICES_LIMIT)
                .collect(),
            LoadType::TrackLoaded => {
                let mut track = loaded.tracks.swap_remove(0);

                vec![CommandOptionChoice {
                    name: track.prettify(),
                    name_localizations: None,
                    value: CommandOptionChoiceValue::String(track.info.uri),
                }]
            }
            LoadType::PlaylistLoaded => {
                let mut choices = vec![CommandOptionChoice {
                    name: loaded.prettify(),
                    name_localizations: None,
                    value: CommandOptionChoiceValue::String(
                        query.grapheme_truncate(100).to_string(),
                    ),
                }];

                if let Some(selected) = loaded.playlist_info.selected_track {
                    let mut track = loaded.tracks.swap_remove(selected as usize);
                    choices.push(CommandOptionChoice {
                        name: track.prettify(),
                        name_localizations: None,
                        value: CommandOptionChoiceValue::String(track.info.uri),
                    });
                }

                choices
            }
            LoadType::LoadFailed => Err(LoadFailedError(query))?,
            LoadType::NoMatches => Vec::new(),
            unknown => Err(UnknownLoadTypeError(unknown))?,
        };

        Ok(ctx.autocomplete(choices).await?)
    }
}

async fn play(
    ctx: &mut Ctx<impl RespondViaMessage>,
    queries: impl IntoIterator<Item = Box<str>> + Send,
) -> Result<(), play::Error> {
    let guild_id = ctx.guild_id_expected();
    let player = ctx.lavalink().player(guild_id).await?;

    let load_ctx = LoadTrackContext::new(player.clone(), ctx);
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
                            t.info.title.as_deref().unwrap_or(UNTITLED_TRACK),
                            t.info.uri
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
                            p.info.name.as_deref().unwrap_or(UNNAMED_PLAYLIST),
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

            auto_join_or_check_in_voice_with_user_and_check_not_suppressed(ctx).await?;

            let total_tracks = Vec::from(results);
            let first_track = total_tracks
                .first()
                .expect("first track must exist")
                .track
                .clone();
            ctx.lavalink()
                .connection_mut(guild_id)
                .queue_mut()
                .enqueue(total_tracks, ctx.author_id());
            player.send(twilight_lavalink::model::Play::from((
                guild_id,
                first_track,
            )))?;
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
            LoadTrackProcessManyError::Process(e) => Err(e)?,
            LoadTrackProcessManyError::UnknownLoadType(e) => Err(e)?,
        },
    }
}

#[derive(CommandOption, CreateOption, Default)]
enum PlaySource {
    #[default]
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
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
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
    async fn run(self, mut ctx: Ctx<SlashCommand>) -> CommandResult {
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
    async fn run(mut ctx: Ctx<MessageCommand>) -> CommandResult {
        let message = ctx.target_message();

        let queries = extract_queries(message);
        if queries.is_empty() {
            bad!("No audio files or URLs found in this message.", ctx);
        };
        Ok(play(&mut ctx, queries).await?)
    }
}
