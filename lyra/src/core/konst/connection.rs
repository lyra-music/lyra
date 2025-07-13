use std::time::Duration;

pub const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(600);
pub const CHANGED_TIMEOUT: Duration = Duration::from_millis(250);
pub const GET_LAVALINK_CONNECTION_INFO_TIMEOUT: Duration = Duration::from_millis(2_000);
