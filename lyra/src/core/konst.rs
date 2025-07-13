pub mod connection {
    use std::time::Duration;

    pub const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(600);
    pub const CHANGED_TIMEOUT: Duration = Duration::from_millis(250);
    pub const GET_LAVALINK_CONNECTION_INFO_TIMEOUT: Duration = Duration::from_millis(2_000);
}

pub mod misc {
    use std::time::Duration;

    pub const ADD_TRACKS_WRAP_LIMIT: usize = 3;

    pub const WAIT_FOR_NOT_SUPPRESSED_TIMEOUT: Duration = Duration::from_secs(30);
    pub const WAIT_FOR_BOT_EVENTS_TIMEOUT: Duration = Duration::from_millis(1_000);
    pub const DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(60);
    pub const QUEUE_ADVANCE_DISABLED_TIMEOUT: Duration = Duration::from_millis(250);
}

pub mod text {
    use std::sync::LazyLock;

    use fuzzy_matcher::skim::SkimMatcherV2;

    pub const UNTITLED_TRACK: &str = "(Untitled Track)";
    pub const UNNAMED_PLAYLIST: &str = "(Unnamed Playlist)";
    pub const UNKNOWN_ARTIST: &str = "(Unknown Artist)";
    pub const EMPTY_EMBED_FIELD: &str = "`-Empty-`";
    pub const NO_ROWS_AFFECTED_MESSAGE: &str = "🔐 No changes were made.";

    // we cannot afford to initialise the entire matcher object without any memoisation,
    // as this will be called more than once: it will be called on every command autocomplete
    // where the choices are tracks as queue positions during fuzzy title matching.
    pub static FUZZY_MATCHER: LazyLock<SkimMatcherV2> = LazyLock::new(SkimMatcherV2::default);
}

pub mod regex {
    use std::sync::LazyLock;

    use regex::Regex;

    // we cannot afford to initialise the entire regex object without any memoisation,
    // as this will be called more than once: it will be called on every `/play` command
    // autocomplete.
    pub static URL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(https://www\.|http://www\.|https://|http://)?[a-zA-Z]{2,}(\.[a-zA-Z]{2,})(\.[a-zA-Z]{2,})?/[a-zA-Z0-9]{2,}|((https://www\.|http://www\.|https://|http://)?[a-zA-Z]{2,}(\.[a-zA-Z]{2,})(\.[a-zA-Z]{2,})?)|(https://www\.|http://www\.|https://|http://)?[a-zA-Z0-9]{2,}\.[a-zA-Z0-9]{2,}\.[a-zA-Z0-9]{2,}(\.[a-zA-Z0-9]{2,})?"
        )
        .expect("regex is valid")
    });
}

pub mod exit_code {
    /// A harmless notice, confirming something the user might have meant to do
    pub const NOTICE: &str = "❕";
    /// A suspicious notice, implying something the user might not have meant to do
    pub const DUBIOUS: &str = "❔";
    /// A harmless warning
    pub const WARNING: &str = "❗";
    /// Needed information was not found, implying user given an incorrect query
    // pub const NOT_FOUND: &str = "❓";
    /// Invalid command usage, implying unmet conditions
    pub const INVALID: &str = "❌";
    /// User lacked sufficient permissions
    pub const PROHIBITED: &str = "🚫";
    /// Bot lacked sufficient permissions
    pub const FORBIDDEN: &str = "⛔";
    /// Other known errors
    pub const KNOWN_ERROR: &str = "‼️";
    /// Unknown errors
    pub const UNKNOWN_ERROR: &str = "⁉️";
}

pub mod discord {
    pub const BASE_URL: &str = "https://discord.com";
    pub const CDN_URL: &str = "https://cdn.discordapp.com";
    pub const COMMAND_CHOICES_LIMIT: usize = 25;
}

pub mod colours {
    pub const EMBED_DEFAULT: u32 = 0x82_6b_d6;
    pub const DOWNVOTE: u32 = 0xdd_2e_44;
    pub const UPVOTE: u32 = 0x58_65_f2;
    pub const POLL_BASE: u32 = 0x00_00_00;
}

pub mod poll {
    pub const UPVOTE: &str = "🟦";
    pub const DOWNVOTE: &str = "🟥";
    pub const BASE: &str = "⬛";
    pub const RATIO_BAR_SIZE: usize = 16;
}

/// Source:
/// [1](https://github.com/lavalink-devs/lavaplayer/blob/e684e603f0f783d5fcbe1eef9a939b6e9e1cb0e9/main/src/main/java/com/sedmelluq/discord/lavaplayer/track/info/AudioTrackInfoBuilder.java#L14)
/// [2](https://github.com/lavalink-devs/lavaplayer/blob/e684e603f0f783d5fcbe1eef9a939b6e9e1cb0e9/main/src/main/java/com/sedmelluq/discord/lavaplayer/container/MediaContainerDetection.java#L22)
pub mod lavaplayer {
    pub const UNKNOWN_TITLE: &str = "Unknown title";
    pub const UNKNOWN_ARTIST: &str = "Unknown artist";
}
