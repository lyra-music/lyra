use aho_corasick::AhoCorasick;

/// Prints the bot's banner.
///
/// This includes the Λύρα icon, cargo package information and build-specific
/// metadata.
pub fn banner() -> String {
    // we can afford to initialise the entire banner string without any memoisation,
    // as this will only be called once, in `runner::start()`.

    let metadata_patterns = [
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

    let metadata_replacements = [
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_LICENSE"),
        env!("VERGEN_BUILD_TIMESTAMP"),
        env!("VERGEN_GIT_DESCRIBE"),
        env!("VERGEN_GIT_SHA"),
        env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        env!("VERGEN_GIT_BRANCH"),
        env!("VERGEN_RUSTC_SEMVER"),
        env!("VERGEN_RUSTC_CHANNEL"),
        env!("VERGEN_RUSTC_HOST_TRIPLE"),
        env!("VERGEN_RUSTC_COMMIT_HASH"),
        env!("VERGEN_CARGO_TARGET_TRIPLE"),
        env!("VERGEN_CARGO_OPT_LEVEL"),
    ];

    let rdr = include_str!("../../../assets/lyra2-ascii.ans");
    let mut wtr = Vec::new();

    let ac = AhoCorasick::new(metadata_patterns).expect("METADATA_PATTERNS must be valid");
    ac.try_stream_replace_all(rdr.as_bytes(), &mut wtr, &metadata_replacements)
        .expect("searching must be infallible");
    String::from_utf8(wtr).expect("interpolated banner must be utf-8")
}
