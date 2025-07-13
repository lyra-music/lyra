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
