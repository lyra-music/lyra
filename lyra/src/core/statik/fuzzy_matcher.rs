use std::sync::LazyLock;

use fuzzy_matcher::skim::SkimMatcherV2;

// we cannot afford to initialise the entire matcher object without any memoisation,
// as this will be called more than once: it will be called on every command autocomplete
// where the choices are tracks as queue positions during fuzzy title matching.
pub static FUZZY_MATCHER: LazyLock<SkimMatcherV2> = LazyLock::new(SkimMatcherV2::default);
