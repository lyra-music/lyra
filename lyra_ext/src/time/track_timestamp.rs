#[cfg(test)]
use mock_instant::thread_local::Instant;
#[cfg(not(test))]
use std::time::Instant;

use std::time::Duration;

#[derive(Debug, Copy, Clone)]
enum Operation {
    Pause(bool),
    Speed(f64),
    Seek(Duration),
}

pub struct Data {
    most_recent_operation: Instant,
    most_recent_position: Instant,
    paused: bool,
    speed: f64,
}

impl Data {
    #[must_use]
    const fn new(started: Instant) -> Self {
        Self {
            most_recent_operation: started,
            most_recent_position: started,
            paused: false,
            speed: 1.0,
        }
    }

    fn reset(&mut self, started: Instant) {
        self.most_recent_operation = started;
        self.most_recent_position = started;
        self.paused = false;
        self.speed = 1.0;
    }
}

pub struct TrackTimestamp {
    started: Instant,
    data: Data,
    last_operation: Instant,
}

impl TrackTimestamp {
    #[must_use]
    pub fn new() -> Self {
        let started = Instant::now();
        Self {
            started,
            data: Data::new(started),
            last_operation: started,
        }
    }

    pub fn reset(&mut self) {
        let started = Instant::now();
        self.started = started;
        self.data.reset(started);
        self.last_operation = started;
    }

    #[must_use]
    pub fn get(&self) -> Duration {
        let data = &self.data;
        let most_recent_duration = data
            .most_recent_position
            .saturating_duration_since(self.started);
        if data.paused {
            return most_recent_duration;
        }
        let elapsed = Instant::now().saturating_duration_since(data.most_recent_operation);
        most_recent_duration + elapsed.mul_f64(data.speed)
    }

    #[must_use]
    pub const fn paused(&self) -> bool {
        self.data.paused
    }

    fn apply(&mut self, op: Operation) {
        let now = Instant::now();
        let last_operation = &mut self.last_operation;
        let since_prev = now - *last_operation;
        *last_operation = now;

        let data = &mut self.data;
        let most_recent_position = &mut data.most_recent_position;
        let paused = &mut data.paused;
        let speed = &mut data.speed;

        match op {
            Operation::Pause(p) => {
                *paused = p;
                if p {
                    *most_recent_position += since_prev.mul_f64(*speed);
                }
            }
            Operation::Speed(m) => {
                if !*paused {
                    *most_recent_position += since_prev.mul_f64(*speed);
                }
                *speed = m;
            }
            Operation::Seek(d) => {
                *most_recent_position = self.started + d;
            }
        }

        data.most_recent_operation += since_prev;
    }

    pub fn set_pause(&mut self, state: bool) {
        if state == self.data.paused {
            return; // this is no-op
        }

        self.apply(Operation::Pause(state));
    }

    pub fn set_speed(&mut self, multiplier: f64) {
        self.apply(Operation::Speed(multiplier));
    }

    pub fn seek_to(&mut self, timestamp: Duration) {
        self.apply(Operation::Seek(timestamp));
    }

    #[inline]
    pub fn resume(&mut self) {
        self.set_pause(false);
    }

    #[inline]
    pub fn pause(&mut self) {
        self.set_pause(true);
    }

    #[inline]
    pub fn seek_forward(&mut self, duration: Duration) {
        self.seek_to(self.get() + duration.mul_f64(self.data.speed));
    }

    #[inline]
    pub fn seek_backward(&mut self, duration: Duration) {
        self.seek_to(self.get().saturating_sub(duration.mul_f64(self.data.speed)));
    }
}

impl Default for TrackTimestamp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use mock_instant::thread_local::MockClock;
    use rstest::{fixture, rstest};

    use super::TrackTimestamp;

    const SECS_0: Duration = Duration::ZERO;
    const SEC: Duration = Duration::from_secs(1);

    #[fixture]
    fn stamp() -> TrackTimestamp {
        TrackTimestamp::new()
    }

    #[rstest]
    fn then_get(stamp: TrackTimestamp) {
        MockClock::advance(SEC);
        assert_eq!(stamp.get(), SEC);
    }

    #[rstest]
    fn pause(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.pause();
        assert_eq!(stamp.get(), SEC);

        MockClock::advance(SEC); // +1 [ignored]
        assert_eq!(stamp.get(), SEC);
    }

    #[rstest]
    fn pause_resume(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.pause();
        assert_eq!(stamp.get(), SEC);

        stamp.resume();
        assert_eq!(stamp.get(), SEC);
    }

    #[rstest]
    fn pause_then_resume(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.pause();
        assert_eq!(stamp.get(), SEC);

        MockClock::advance(SEC); // +1 [ignored]
        assert_eq!(stamp.get(), SEC);

        stamp.resume();
        assert_eq!(stamp.get(), SEC);

        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), 2 * SEC);
    }

    #[test]
    fn seekf() {
        MockClock::set_time(SEC);
        let mut stamp = TrackTimestamp::new();

        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.seek_forward(SEC); // +1
        assert_eq!(stamp.get(), 2 * SEC);
    }

    #[test]
    fn pause_seekf() {
        MockClock::set_time(SEC);
        let mut stamp = TrackTimestamp::new();

        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.pause();
        assert_eq!(stamp.get(), SEC);

        stamp.seek_forward(SEC); // +1
        assert_eq!(stamp.get(), 2 * SEC);
    }

    #[test]
    fn pause_then_seekf() {
        MockClock::set_time(SEC);
        let mut stamp = TrackTimestamp::new();

        stamp.pause();
        assert_eq!(stamp.get(), SECS_0);

        MockClock::advance(SEC); // +1 [ignored]
        assert_eq!(stamp.get(), SECS_0);

        stamp.seek_forward(SEC); // +1
        assert_eq!(stamp.get(), SEC);
    }

    #[rstest]
    #[case(SEC, SEC)]
    #[case(2 * SEC, SECS_0)]
    #[case(3 * SEC, SECS_0)]
    fn seekb(mut stamp: TrackTimestamp, #[case] input: Duration, #[case] expected: Duration) {
        MockClock::advance(2 * SEC); // +2
        assert_eq!(stamp.get(), 2 * SEC);

        stamp.seek_backward(input); // -input, min 0
        assert_eq!(stamp.get(), expected);
    }

    #[rstest]
    #[case(SEC, SEC)]
    #[case(2 * SEC, SECS_0)]
    #[case(3 * SEC + SEC, SECS_0)]
    fn pause_seekb(mut stamp: TrackTimestamp, #[case] input: Duration, #[case] expected: Duration) {
        MockClock::advance(2 * SEC); // +2
        assert_eq!(stamp.get(), 2 * SEC);

        stamp.pause();
        assert_eq!(stamp.get(), 2 * SEC);

        stamp.seek_backward(input); // -input, min 0
        assert_eq!(stamp.get(), expected);
    }

    #[rstest]
    #[case(SEC, SEC)]
    #[case(2 * SEC, SECS_0)]
    #[case(3 * SEC + SEC, SECS_0)]
    fn pause_then_seekb(
        mut stamp: TrackTimestamp,
        #[case] input: Duration,
        #[case] expected: Duration,
    ) {
        MockClock::advance(2 * SEC); // +2
        assert_eq!(stamp.get(), 2 * SEC);

        stamp.pause();
        assert_eq!(stamp.get(), 2 * SEC);

        MockClock::advance(SEC); // +1 [ignored]
        assert_eq!(stamp.get(), 2 * SEC);

        stamp.seek_backward(input); // -input, min 0
        assert_eq!(stamp.get(), expected);
    }

    #[rstest]
    fn speed(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.set_speed(2.);
        assert_eq!(stamp.get(), SEC);

        MockClock::advance(SEC); // +(1 x2) = +2
        assert_eq!(stamp.get(), 3 * SEC);
    }

    #[rstest]
    fn pause_speed(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.pause();
        assert_eq!(stamp.get(), SEC);

        stamp.set_speed(2.);
        assert_eq!(stamp.get(), SEC);
    }

    #[rstest]
    fn pause_then_speed(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.pause();
        assert_eq!(stamp.get(), SEC);

        stamp.set_speed(2.);
        assert_eq!(stamp.get(), SEC);

        MockClock::advance(SEC); // ignored
        assert_eq!(stamp.get(), SEC);
    }

    #[rstest]
    fn pause_speed_resume(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.pause();
        assert_eq!(stamp.get(), SEC);

        stamp.set_speed(2.);
        assert_eq!(stamp.get(), SEC);

        MockClock::advance(SEC); // +(1 x2) [ignored]
        assert_eq!(stamp.get(), SEC);

        stamp.resume();
        assert_eq!(stamp.get(), SEC);

        MockClock::advance(SEC); // +(1 x2) = +2
        assert_eq!(stamp.get(), 3 * SEC);
    }

    #[rstest]
    fn speed_seekf(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.set_speed(2.);
        assert_eq!(stamp.get(), SEC);

        stamp.seek_forward(SEC); // +(1 x2)
        assert_eq!(stamp.get(), 3 * SEC);
    }

    #[rstest]
    fn speed_seekf_pause(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.set_speed(2.);
        assert_eq!(stamp.get(), SEC);

        stamp.seek_forward(SEC); // +(1 x2)
        assert_eq!(stamp.get(), 3 * SEC);

        stamp.pause();
        assert_eq!(stamp.get(), 3 * SEC);

        MockClock::advance(SEC);
        assert_eq!(stamp.get(), 3 * SEC);
    }

    #[rstest]
    fn speed_seekb(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.set_speed(2.);
        assert_eq!(stamp.get(), SEC);

        stamp.seek_backward(SEC); // -(1 x2), min 0
        assert_eq!(stamp.get(), SECS_0);
    }

    #[rstest]
    fn speed_seekb_pause(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.set_speed(2.);
        assert_eq!(stamp.get(), SEC);

        stamp.seek_backward(SEC); // -(1 x2), min 0
        assert_eq!(stamp.get(), SECS_0);

        stamp.pause();
        assert_eq!(stamp.get(), SECS_0);

        MockClock::advance(SEC); // +(1 x2) [ignored]
        assert_eq!(stamp.get(), SECS_0);
    }

    #[rstest]
    fn pause_pause(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.pause();
        assert_eq!(stamp.get(), SEC);

        MockClock::advance(SEC); // +1 [ignored]
        assert_eq!(stamp.get(), SEC);

        stamp.pause();
        assert_eq!(stamp.get(), SEC);

        MockClock::advance(SEC); // +1 [ignored]
        assert_eq!(stamp.get(), SEC);
    }

    #[rstest]
    fn resume(mut stamp: TrackTimestamp) {
        MockClock::advance(SEC); // +1
        assert_eq!(stamp.get(), SEC);

        stamp.resume();
        assert_eq!(stamp.get(), SEC);
    }
}
