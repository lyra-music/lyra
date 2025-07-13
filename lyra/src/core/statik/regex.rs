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
