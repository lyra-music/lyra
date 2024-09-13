use time::{format_description::well_known::Iso8601, OffsetDateTime};

/// # Panics
/// This function panics when writing ISO 8601 datetime to string fails
#[must_use]
pub fn iso8601() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::DEFAULT)
        .expect("writing iso8601 datetime to string should never fail")
}
