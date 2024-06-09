use std::{fmt::Display, sync::OnceLock, time::Duration};

use regex::Regex;

fn timestamp() -> &'static Regex {
    static TIMESTAMP: OnceLock<Regex> = OnceLock::new();
    TIMESTAMP.get_or_init(|| {
        Regex::new(
            r"^(((?<h>[1-9]\d*):(?<m1>[0-5]\d))|(?<m2>[0-5]?\d)):(?<s>[0-5]\d)(\.(?<ms>\d{3}))?$",
        )
        .expect("regex is valid")
    })
}

fn timestamp_2() -> &'static Regex {
    static TIMESTAMP_2: OnceLock<Regex> = OnceLock::new();
    TIMESTAMP_2.get_or_init(|| {
            Regex::new(
                r"^((?<h>[1-9]\d*)\s?hr?)?\s*((?<m>[1-9]|[1-5]\d)\s?m(in)?)?\s*((?<s>[1-9]|[1-5]\d)\s?s(ec)?)?\s*((?<ms>[1-9]\d{0,2})\s?ms(ec)?)?$"
            )
            .expect("regex is valid")
        })
}

pub struct PrettyDurationRefDisplayer<'a>(&'a Duration);

pub trait PrettyDurationDisplay {
    fn pretty_display(&self) -> PrettyDurationRefDisplayer;
}

impl PrettyDurationDisplay for Duration {
    fn pretty_display(&self) -> PrettyDurationRefDisplayer {
        PrettyDurationRefDisplayer(self)
    }
}

trait FromPrettyStr {
    fn from_pretty_str(value: &str) -> Result<Self, &str>
    where
        Self: Sized;
}

impl FromPrettyStr for Duration {
    fn from_pretty_str(value: &str) -> Result<Self, &str>
    where
        Self: Sized,
    {
        let captures = if let Some(captures) = timestamp().captures(value) {
            captures
        } else if let Some(captures) = timestamp_2().captures(value) {
            captures
        } else {
            return Err(value);
        };

        let ms = captures
            .name("ms")
            .and_then(|c| c.as_str().parse().ok())
            .unwrap_or(0);
        let s = captures
            .name("s")
            .and_then(|c| c.as_str().parse().ok())
            .unwrap_or(0);
        let m = captures
            .name("m")
            .or_else(|| captures.name("m1"))
            .or_else(|| captures.name("m2"))
            .and_then(|c| c.as_str().parse().ok())
            .unwrap_or(0);
        let h = captures
            .name("h")
            .and_then(|c| c.as_str().parse().ok())
            .unwrap_or(0);

        let total_ms = (((h * 60 + m) * 60 + s) * 1000) + ms;
        Ok(Self::from_millis(total_ms))
    }
}

fn fmt(millis: u128, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let divrem = |x, y| (x / y, x % y);

    let (s, ms) = divrem(millis, 1000);
    let (m, s) = divrem(s, 60);
    let (h, m) = divrem(m, 60);

    match (h, m, s) {
        (0, 0, 0) => write!(f, "0:00.{ms:03}"),
        (0, m, s) => write!(f, "{m}:{s:02}"),
        (h, m, s) => write!(f, "{h}:{m:02}:{s:02}"),
    }
}

impl Display for PrettyDurationRefDisplayer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt(self.0.as_millis(), f)
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use rstest::rstest;

    use super::{FromPrettyStr, PrettyDurationDisplay};

    #[rstest]
    #[case(Duration::ZERO, "0:00.000")]
    #[case(Duration::from_millis(999), "0:00.999")]
    #[case(Duration::from_secs(1), "0:01")]
    #[case(Duration::from_secs(59), "0:59")]
    #[case(Duration::from_secs(60), "1:00")]
    #[case(Duration::from_secs(61), "1:01")]
    #[case(Duration::from_secs(59*60 + 59),"59:59")]
    #[case(Duration::from_secs(60*60), "1:00:00")]
    #[case(Duration::from_secs(60*60 + 1), "1:00:01")]
    #[case(Duration::from_secs(60*60 + 59),"1:00:59")]
    #[case(Duration::from_secs(61*60), "1:01:00")]
    #[case(Duration::from_secs(60*60 + 61), "1:01:01")]
    #[case(Duration::from_secs(999*60*60 + 59*60 + 59), "999:59:59")]
    fn pretty_duration_display_to_string(#[case] input: Duration, #[case] expected: &str) {
        assert_eq!(input.pretty_display().to_string(), expected);
    }

    #[rstest]
    #[case("0:0", None)]
    #[case("0:00", Some(Duration::ZERO))]
    #[case("0:00.0", None)]
    #[case("0:00.999", Some(Duration::from_millis(999)))]
    #[case("0:00.9999", None)]
    #[case("0:01", Some(Duration::from_secs(1)))]
    #[case("0:59.999", Some(Duration::from_millis(59_999)))]
    #[case("0:99.999", None)]
    #[case("1:00", Some(Duration::from_secs(60)))]
    #[case("1:00.999", Some(Duration::from_millis(60_999)))]
    #[case("1:01", Some(Duration::from_secs(61)))]
    #[case("59:59.999", Some(Duration::from_millis(59*60_000 + 59_999)))]
    #[case("99:59.999", None)]
    #[case("0:0:00", None)]
    #[case("0:00:00", None)]
    #[case("1:00:00", Some(Duration::from_secs(60*60)))]
    #[case("1:00:00.999", Some(Duration::from_millis(60*60_000 + 999)))]
    #[case("1:00:01", Some(Duration::from_secs(60*60 + 1)))]
    #[case("1:00:59.999", Some(Duration::from_millis(60*60_000 + 59_999)))]
    #[case("1:01:00", Some(Duration::from_secs(61*60)))]
    #[case("1:01:00.999", Some(Duration::from_millis(60*60_000 + 60_999)))]
    #[case("1:01:01", Some(Duration::from_secs(60*60 + 61)))]
    #[case("999:59:59.999", Some(Duration::from_millis(999*60*60_000 + 59*60_000 + 59_999)))]
    fn duration_from_pretty_str_1(#[case] input: &str, #[case] expected: Option<Duration>) {
        assert_eq!(Duration::from_pretty_str(input).ok(), expected);
    }

    #[rstest]
    #[case("", Some(Duration::ZERO))]
    #[case("0ms", None)]
    #[case("01ms", None)]
    #[case("999ms", Some(Duration::from_millis(999)))]
    #[case("9999ms", None)]
    #[case("0s", None)]
    #[case("1s", Some(Duration::from_secs(1)))]
    #[case("01s", None)]
    #[case("59 sec 999 msec", Some(Duration::from_millis(59_999)))]
    #[case("99s999ms", None)]
    #[case("0m", None)]
    #[case("1m", Some(Duration::from_secs(60)))]
    #[case("01m", None)]
    #[case("1m 999ms", Some(Duration::from_millis(60_999)))]
    #[case("1m1s", Some(Duration::from_secs(61)))]
    #[case("59 min 59 sec 999 msec", Some(Duration::from_millis(59*60_000 + 59_999)))]
    #[case("99m59s999ms", None)]
    #[case("0h", None)]
    #[case("1h", Some(Duration::from_secs(60*60)))]
    #[case("01h", None)]
    #[case("1h 999ms", Some(Duration::from_millis(60*60_000 + 999)))]
    #[case("1h 1s", Some(Duration::from_secs(60*60 + 1)))]
    #[case("1h 59s 999ms", Some(Duration::from_millis(60*60_000 + 59_999)))]
    #[case("1h1m", Some(Duration::from_secs(61*60)))]
    #[case("1h1m 999ms", Some(Duration::from_millis(60*60_000 + 60_999)))]
    #[case("1h1m1s", Some(Duration::from_secs(60*60 + 61)))]
    #[case("999 hr 59 min 59 sec 999 msec", Some(Duration::from_millis(999*60*60_000 + 59*60_000 + 59_999)))]
    fn duration_from_pretty_str_2(#[case] input: &str, #[case] expected: Option<Duration>) {
        assert_eq!(Duration::from_pretty_str(input).ok(), expected);
    }
}
