use std::env;

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

#[derive(Default)]
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

impl CreateOption for PlaySource {
    fn create_option(data: CreateOptionData) -> ModelCommandOption {
        // we can afford to parse the env var without any memoisation, as
        // this will only be called once, in `command::declare::COMMANDS`.
        let (deezer_enabled, spotify_enabled) = (
            env::var("PLUGINS_LAVASRC_SOURCES_DEEZER")
                .is_ok_and(|v| v.parse::<bool>().is_ok_and(|b| b)),
            env::var("PLUGINS_LAVASRC_SOURCES_SPOTIFY")
                .is_ok_and(|v| v.parse::<bool>().is_ok_and(|b| b)),
        );

        let mut choices = Vec::with_capacity(6);
        choices.extend_from_slice(&[
            create_choice("Youtube", "ytsearch:"),
            create_choice("Youtube Music", "ytmsearch:"),
            create_choice("SoundCloud", "scsearch:"),
        ]);

        if deezer_enabled {
            choices.extend_from_slice(&[
                create_choice("Deezer (Search Query)", "dzsearch:"),
                create_choice("Deezer (ISRC)", "dzisrc:"),
            ]);
        }

        if spotify_enabled {
            choices.push(create_choice("Spotify", "spsearch:"));
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

impl PlaySource {
    /// Get the value corresponding to the current variant.
    pub const fn value(&self) -> &'static str {
        match self {
            Self::Youtube => "ytsearch:",
            Self::YoutubeMusic => "ytmsearch:",
            Self::SoundCloud => "scsearch:",
            Self::DeezerQuery => "dzsearch:",
            Self::DeezerIsrc => "dzisrc:",
            Self::Spotify => "spsearch:",
        }
    }

    /// Create a `PlaySource` from its string value.
    pub fn from_value(value: &str) -> Option<Self> {
        match value {
            "ytsearch:" => Some(Self::Youtube),
            "ytmsearch:" => Some(Self::YoutubeMusic),
            "scsearch:" => Some(Self::SoundCloud),
            "dzsearch:" => Some(Self::DeezerQuery),
            "dzisrc:" => Some(Self::DeezerIsrc),
            "spsearch:" => Some(Self::Spotify),
            _ => None,
        }
    }

    /// Returns the display names for the music services.
    ///
    /// There are only 5 names for 6 variants since both Deezer variants
    /// share the same display name.
    pub const fn display_names() -> [&'static str; 5usize] {
        [
            "Youtube",
            "Youtube Music",
            "SoundCloud",
            "Deezer",
            "Spotify",
        ]
    }
}

// Helper function to reduce repetition in choice creation
fn create_choice(name: &str, value: &str) -> CommandOptionChoice {
    let choice_name = IntoLocalizationsInternal::into_localizations((name, None));
    CommandOptionChoice {
        name: choice_name.fallback,
        name_localizations: choice_name.localizations,
        value: CommandOptionChoiceValue::String(value.into()),
    }
}
