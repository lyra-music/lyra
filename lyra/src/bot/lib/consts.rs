pub mod metadata {
    use once_cell::sync::Lazy;
    use version_check::Version;

    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const COPYRIGHT: &str = env!("CARGO_PKG_LICENSE");
    pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
    pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
    pub const SUPPORT: &str = "https://discord.com/invite/d4UerJpvTp";

    pub static RUST_VERSION: Lazy<String> =
        Lazy::new(|| format!("{}", Version::read().expect("rustc version must exist")));
    pub static OS_INFO: Lazy<String> = Lazy::new(|| format!("{}", os_info::get()));
}

pub mod connections {
    pub const INACTIVITY_TIMEOUT: u16 = 600;
    pub const INACTIVITY_TIMEOUT_POLL_N: u8 = 10;
    pub const INACTIVITY_TIMEOUT_POLL_INTERVAL: u16 =
        INACTIVITY_TIMEOUT / INACTIVITY_TIMEOUT_POLL_N as u16;
}

pub mod misc {
    pub const DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT: u8 = 60;
}

pub mod texts {
    pub const EMPTY_EMBED_FIELD: &str = "`-Empty-`";
    pub const NO_ROWS_AFFECTED_MESSAGE: &str = "🔐 No changes were made.";
}

pub mod exit_codes {
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
