use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// # Panics
/// if system clock went backwards
#[must_use]
pub fn unix() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must move forward")
}
