use std::{collections::HashMap, env, sync::LazyLock};

use const_str::concat;
use twilight_interactions::command::{
    CommandOption, CreateOption,
    internal::{CommandOptionData, CreateOptionData, IntoLocalizationsInternal},
};
use twilight_interactions::error::ParseOptionErrorType;
use twilight_model::application::command::{
    CommandOption as ModelCommandOption, CommandOptionChoice, CommandOptionChoiceValue,
    CommandOptionType,
};
use twilight_model::application::interaction::{
    InteractionDataResolved, application_command::CommandOptionValue,
};

struct PlaySourceChoice {
    name: &'static str,
    value: &'static str,
}

const YOUTUBE: &str = "Youtube";
const DEEZER: &str = "Deezer";

impl PlaySourceChoice {
    const fn new(name: &'static str, value: &'static str) -> Self {
        Self { name, value }
    }

    const YOUTUBE: Self = Self::new("Youtube", "ytsearch");
    const YOUTUBE_MUSIC: Self = Self::new(concat!(YOUTUBE, " Music"), "ytmsearch");
    const SOUNDCLOUD: Self = Self::new("SoundCloud", "scsearch");
    const DEEZER_QUERY: Self = Self::new(concat!(DEEZER, " (Search Query)"), "dzsearch");
    const DEEZER_ISRC: Self = Self::new(concat!(DEEZER, " (ISRC)"), "dzisrc");
    const SPOTIFY: Self = Self::new("Spotify", "spsearch");

    const DEFAULT_SOURCES: [Self; 3] = [Self::YOUTUBE, Self::YOUTUBE_MUSIC, Self::SOUNDCLOUD];
}

#[derive(Default, Clone, Copy)]
pub enum PlaySource {
    #[default]
    Youtube,
    YoutubeMusic,
    SoundCloud,
    // only accessible if env `PLUGINS_LAVASRC_SOURCES_DEEZER` is `true`
    DeezerQuery,
    // only accessible if env `PLUGINS_LAVASRC_SOURCES_DEEZER` is `true`
    DeezerIsrc,
    // only accessible if env `PLUGINS_LAVASRC_SOURCES_SPOTIFY` is `true`
    Spotify,
}

// we cannot afford to parse the env var without any memoisation, as
// this will be called more than once: exactly two times in
// - `PlaySource::create_option()`
// - `PlaySource::display_names()`
static DEEZER_ENABLED: LazyLock<bool> = LazyLock::new(|| {
    env::var("PLUGINS_LAVASRC_SOURCES_DEEZER").is_ok_and(|v| v.parse::<bool>().is_ok_and(|b| b))
});
static SPOTIFY_ENABLED: LazyLock<bool> = LazyLock::new(|| {
    env::var("PLUGINS_LAVASRC_SOURCES_SPOTIFY").is_ok_and(|v| v.parse::<bool>().is_ok_and(|b| b))
});

// we cannot afford to initialise the entire array object without any memoisation, as
// this will be called more than once: it will be called on every `PLaySource::display_names()`
// calls.
static DISPLAY_NAMES: LazyLock<&'static [&'static str]> = LazyLock::new(|| {
    let mut names = Vec::with_capacity(PlaySource::N);
    names.extend_from_slice(&PlaySourceChoice::DEFAULT_SOURCES.map(|x| x.name));

    if *DEEZER_ENABLED {
        names.push(DEEZER);
    }

    if *SPOTIFY_ENABLED {
        names.push(PlaySourceChoice::SPOTIFY.name);
    }

    // this call is needed because there may not be exactly `PlaySource::N` elements and
    // `Vec::leak()` does not reallocate or shrink the `Vec` as stated in the method
    // documentation.
    names.shrink_to_fit();

    names.leak()
});

static VALUE_TO_PLAY_SOURCE: LazyLock<HashMap<&'static str, PlaySource>> = LazyLock::new(|| {
    HashMap::from([
        (PlaySourceChoice::YOUTUBE.value, PlaySource::Youtube),
        (
            PlaySourceChoice::YOUTUBE_MUSIC.value,
            PlaySource::YoutubeMusic,
        ),
        (PlaySourceChoice::SOUNDCLOUD.value, PlaySource::SoundCloud),
        (
            PlaySourceChoice::DEEZER_QUERY.value,
            PlaySource::DeezerQuery,
        ),
        (PlaySourceChoice::DEEZER_ISRC.value, PlaySource::DeezerIsrc),
        (PlaySourceChoice::SPOTIFY.value, PlaySource::Spotify),
    ])
});

impl PlaySource {
    /// Number of supported play sources.
    const N: usize = 6;

    const fn as_choice(self) -> PlaySourceChoice {
        match self {
            Self::Youtube => PlaySourceChoice::YOUTUBE,
            Self::YoutubeMusic => PlaySourceChoice::YOUTUBE_MUSIC,
            Self::SoundCloud => PlaySourceChoice::SOUNDCLOUD,
            Self::DeezerQuery => PlaySourceChoice::DEEZER_QUERY,
            Self::DeezerIsrc => PlaySourceChoice::DEEZER_ISRC,
            Self::Spotify => PlaySourceChoice::SPOTIFY,
        }
    }

    /// Get the value corresponding to the current variant.
    #[inline]
    pub const fn value(self) -> &'static str {
        self.as_choice().value
    }

    /// Create a `PlaySource` from its string value.
    pub fn from_value(value: &str) -> Option<Self> {
        VALUE_TO_PLAY_SOURCE.get(value).copied()
    }

    /// Returns the display names for the music services.
    ///
    /// There are only up to 5 names for 6 variants since both Deezer
    /// variants share the same display name.
    #[inline]
    pub fn display_names() -> &'static [&'static str] {
        &DISPLAY_NAMES
    }
}

impl CreateOption for PlaySource {
    fn create_option(data: CreateOptionData) -> ModelCommandOption {
        let mut choices = Vec::with_capacity(Self::N);
        choices.extend_from_slice(&[
            create_choice(&PlaySourceChoice::YOUTUBE),
            create_choice(&PlaySourceChoice::YOUTUBE_MUSIC),
            create_choice(&PlaySourceChoice::SOUNDCLOUD),
        ]);

        if *DEEZER_ENABLED {
            choices.extend_from_slice(&[
                create_choice(&PlaySourceChoice::DEEZER_QUERY),
                create_choice(&PlaySourceChoice::DEEZER_ISRC),
            ]);
        }

        if *SPOTIFY_ENABLED {
            choices.push(create_choice(&PlaySourceChoice::SPOTIFY));
        }

        data.builder(CommandOptionType::String)
            .choices(choices)
            .build()
    }
}

impl CommandOption for PlaySource {
    fn from_option(
        value: CommandOptionValue,
        data: CommandOptionData,
        resolved: Option<&InteractionDataResolved>,
    ) -> Result<Self, ParseOptionErrorType> {
        let parsed_string: String = String::from_option(value, data, resolved)?;

        Self::from_value(&parsed_string).ok_or(ParseOptionErrorType::InvalidChoice(parsed_string))
    }
}

// Helper function to reduce repetition in choice creation
fn create_choice(choice: &PlaySourceChoice) -> CommandOptionChoice {
    let choice_name = IntoLocalizationsInternal::into_localizations((choice.name, None));
    CommandOptionChoice {
        name: choice_name.fallback,
        name_localizations: choice_name.localizations,
        value: CommandOptionChoiceValue::String(choice.value.into()),
    }
}
