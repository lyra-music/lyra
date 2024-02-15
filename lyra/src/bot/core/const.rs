pub mod metadata {
    use version_check::Version;

    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const COPYRIGHT: &str = env!("CARGO_PKG_LICENSE");
    pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
    pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
    pub const SUPPORT: &str = "https://discord.com/invite/d4UerJpvTp";

    lazy_static::lazy_static! {
        pub static ref RUST_VERSION: Box<str> = Version::read().expect("rustc version must exist").to_string().into();
        pub static ref OS_INFO: Box<str> = os_info::get().to_string().into();
    }
}

pub mod connection {
    pub const INACTIVITY_TIMEOUT: u16 = 600;
    pub const INACTIVITY_TIMEOUT_POLL_N: u8 = 10;
    pub const INACTIVITY_TIMEOUT_POLL_INTERVAL: u16 =
        INACTIVITY_TIMEOUT / INACTIVITY_TIMEOUT_POLL_N as u16;
}

pub mod misc {
    pub const WAIT_FOR_BOT_EVENTS_TIMEOUT: u16 = 250;
    pub const ADD_TRACKS_WRAP_LIMIT: usize = 3;
    pub const WAIT_FOR_NOT_SUPPRESSED_TIMEOUT: u8 = 30;
    pub const DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT: u8 = 60;
}

pub mod text {
    use fuzzy_matcher::skim::SkimMatcherV2;

    pub const UNTITLED_TRACK: &str = "(Untitled Track)";
    pub const UNNAMED_PLAYLIST: &str = "(Unnamed Playlist)";
    pub const UNKNOWN_ARTIST: &str = "(Unknown Artist)";
    pub const EMPTY_EMBED_FIELD: &str = "`-Empty-`";
    pub const NO_ROWS_AFFECTED_MESSAGE: &str = "🔐 No changes were made.";

    lazy_static::lazy_static! {
        pub static ref FUZZY_MATCHER: SkimMatcherV2 = SkimMatcherV2::default();
    }
}

pub mod regex {
    use regex::Regex;

    lazy_static::lazy_static! {
        pub static ref URL: Regex =
            Regex::new(r"(https://www\.|http://www\.|https://|http://)?[a-zA-Z]{2,}(\.[a-zA-Z]{2,})(\.[a-zA-Z]{2,})?/[a-zA-Z0-9]{2,}|((https://www\.|http://www\.|https://|http://)?[a-zA-Z]{2,}(\.[a-zA-Z]{2,})(\.[a-zA-Z]{2,})?)|(https://www\.|http://www\.|https://|http://)?[a-zA-Z0-9]{2,}\.[a-zA-Z0-9]{2,}\.[a-zA-Z0-9]{2,}(\.[a-zA-Z0-9]{2,})?")
                .expect("regex must be valid");
        pub static ref TIMESTAMP: Regex =
            Regex::new(r"^(((?<h>[1-9]\d*):(?<m1>[0-5]\d))|(?<m2>[0-5]?\d)):(?<s>[0-5]\d)(\.(?<ms>\d{3}))?$")
                .expect("regex must be valild");
        pub static ref TIMESTAMP_2: Regex =
            Regex::new(r"^((?<h>[1-9]\d*)\s?hr?)?\s*((?<m>[1-9]|[1-5]\d)\s?m(in)?)?\s*((?<s>[1-9]|[1-5]\d)\s?s(ec)?)?\s*((?<ms>[1-9]\d{0,2})\s?ms(ec)?)?$")
                .expect("regex must be valid");
    }
}

pub mod exit_code {
    /// A harmless notice, confirming something the user might have meant to do
    pub const NOTICE: &str = "❕";
    /// A suspicious notice, implying something the user might not have meant to do
    pub const DUBIOUS: &str = "❔";
    /// A harmless warning
    pub const WARNING: &str = "❗";
    /// Needed information was not found, implying user given an incorrect query
    pub const NOT_FOUND: &str = "❓";
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
    pub const UPVOTE: &str = "🟪";
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
