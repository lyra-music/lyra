use lavalink_rs::model::events::Events;

pub fn handlers() -> Events {
    Events {
        track_start: Some(super::track::start),
        track_end: Some(super::track::end),
        track_exception: Some(super::track::exception),
        track_stuck: Some(super::track::stuck),
        ..Default::default()
    }
}
