use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[must_use]
pub fn unix_time() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}
