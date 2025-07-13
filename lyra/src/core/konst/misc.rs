use std::time::Duration;

pub const ADD_TRACKS_WRAP_LIMIT: usize = 3;

pub const WAIT_FOR_NOT_SUPPRESSED_TIMEOUT: Duration = Duration::from_secs(30);
pub const WAIT_FOR_BOT_EVENTS_TIMEOUT: Duration = Duration::from_millis(1_000);
pub const DESTRUCTIVE_COMMAND_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(60);
pub const QUEUE_ADVANCE_DISABLED_TIMEOUT: Duration = Duration::from_millis(250);
