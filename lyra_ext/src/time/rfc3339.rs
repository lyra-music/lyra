use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// # Panics
/// This function panics when writing RFC 3339 datetime to string fails
#[must_use]
pub fn rfc3339_time() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("writing rfc3339 datetime to string should never fail")
}
