use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// # Panics
/// if system clock went backwards
#[must_use]
pub fn unix() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| panic!("system clock went backwards"))
}
