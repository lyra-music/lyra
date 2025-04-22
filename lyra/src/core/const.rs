pub mod metadata {
    use std::sync::LazyLock;

    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const COPYRIGHT: &str = env!("CARGO_PKG_LICENSE");
    const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
    const BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP");

    const CARGO_TARGET_TRIPLE: &str = env!("VERGEN_CARGO_TARGET_TRIPLE");
    const CARGO_OPT_LEVEL: &str = env!("VERGEN_CARGO_OPT_LEVEL");

    const RUSTC_SEMVER: &str = env!("VERGEN_RUSTC_SEMVER");
    const RUSTC_CHANNEL: &str = env!("VERGEN_RUSTC_CHANNEL");
    const RUSTC_HOST_TRIPLE: &str = env!("VERGEN_RUSTC_HOST_TRIPLE");
    const RUSTC_COMMIT_HASH: &str = env!("VERGEN_RUSTC_COMMIT_HASH");

    const GIT_DESCRIBE: &str = env!("VERGEN_GIT_DESCRIBE");
    const GIT_SHA: &str = env!("VERGEN_GIT_SHA");
    const GIT_BRANCH: &str = env!("VERGEN_GIT_BRANCH");
    const GIT_COMMIT_TIMESTAMP: &str = env!("VERGEN_GIT_COMMIT_TIMESTAMP");

    const METADATA_N: usize = 14;
    const METADATA_REPLACEMENTS: [&str; METADATA_N] = [
        VERSION,
        AUTHORS,
        COPYRIGHT,
        BUILD_TIMESTAMP,
        GIT_DESCRIBE,
        GIT_SHA,
        GIT_COMMIT_TIMESTAMP,
        GIT_BRANCH,
        RUSTC_SEMVER,
        RUSTC_CHANNEL,
        RUSTC_HOST_TRIPLE,
        RUSTC_COMMIT_HASH,
        CARGO_TARGET_TRIPLE,
        CARGO_OPT_LEVEL,
    ];

    const METADATA_PATTERNS: [&str; METADATA_N] = [
        "%version",
        "%authors",
        "%copyright",
        "%build_timestamp",
        "%git_describe",
        "%git_sha",
        "%git_commit_timestamp",
        "%git_branch",
        "%rustc_semver",
        "%rustc_channel",
        "%rustc_host",
        "%rustc_commit_hash",
        "%cargo_target_triple",
        "%cargo_opt_level",
    ];

    pub static BANNER: LazyLock<&'static str> = LazyLock::new(|| {
        use aho_corasick::AhoCorasick;

        let rdr = include_str!("../../../assets/lyra2-ascii.ans");
        let mut wtr = Vec::new();

        let ac = AhoCorasick::new(METADATA_PATTERNS).expect("METADATA_PATTERNS must be valid");
        ac.try_stream_replace_all(rdr.as_bytes(), &mut wtr, &METADATA_REPLACEMENTS)
            .expect("searching must be infallible");
        String::from_utf8(wtr)
            .expect("interpolated banner must be utf-8")
            .leak()
    });
}

pub mod connection {
    use std::time::Duration;

    pub const INACTIVITY_TIMEOUT_SECS: u16 = 600;
    pub const INACTIVITY_TIMEOUT_POLL_N: u8 = 10;

    pub const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(INACTIVITY_TIMEOUT_SECS as u64);
    pub const CHANGED_TIMEOUT: Duration = Duration::from_millis(250);
    pub const GET_LAVALINK_CONNECTION_INFO_TIMEOUT: Duration = Duration::from_millis(2_000);
    pub const INACTIVITY_TIMEOUT_POLL_INTERVAL: Duration =
        Duration::from_secs(INACTIVITY_TIMEOUT_SECS as u64 / INACTIVITY_TIMEOUT_POLL_N as u64);
}

pub mod misc {
    use std::time::Duration;

    pub const ADD_TRACKS_WRAP_LIMIT: usize = 3;

    pub const WAIT_FOR_NOT_SUPPRESSED_TIMEOUT: Duration = Duration::from_secs(30);
    pub const WAIT_FOR_BOT_EVENTS_TIMEOUT: Duration = Duration::from_millis(1_000);
    pub const DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(60);
    pub const QUEUE_ADVANCE_LOCKED_TIMEOUT: Duration = Duration::from_millis(250);
}

pub mod text {
    use std::sync::LazyLock;

    use fuzzy_matcher::skim::SkimMatcherV2;

    pub const UNTITLED_TRACK: &str = "(Untitled Track)";
    pub const UNNAMED_PLAYLIST: &str = "(Unnamed Playlist)";
    pub const UNKNOWN_ARTIST: &str = "(Unknown Artist)";
    pub const EMPTY_EMBED_FIELD: &str = "`-Empty-`";
    pub const NO_ROWS_AFFECTED_MESSAGE: &str = "🔐 No changes were made.";

    pub static FUZZY_MATCHER: LazyLock<SkimMatcherV2> = LazyLock::new(SkimMatcherV2::default);
}

pub mod regex {
    use std::sync::LazyLock;

    use regex::Regex;

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
