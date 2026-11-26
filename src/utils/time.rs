//! "5 minutes ago" style times, worked out when shown so they stay current.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::text;

const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const MONTH: i64 = 30 * DAY;
const YEAR: i64 = 365 * DAY;

/// Seconds since 1970 (UTC).
pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_secs().try_into().unwrap_or(0))
}

/// How long before `now` the time `then` was, both in seconds since 1970.
pub fn ago(then: i64, now: i64) -> String {
    let seconds = now.saturating_sub(then);
    if seconds < MINUTE {
        return text::JUST_NOW.to_owned();
    }
    let (count, unit) = [(YEAR, text::YEAR), (MONTH, text::MONTH), (DAY, text::DAY), (HOUR, text::HOUR)]
        .into_iter()
        .find(|(length, _)| seconds >= *length)
        .map_or((seconds / MINUTE, text::MINUTE), |(length, unit)| (seconds / length, unit));
    text::time_ago(count, unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_largest_whole_unit() {
        let now = 10 * YEAR;
        assert_eq!(ago(now - 5, now), "just now");
        assert_eq!(ago(now + 30, now), "just now");
        assert_eq!(ago(now - MINUTE, now), "1 minute ago");
        assert_eq!(ago(now - 3 * HOUR - 59 * MINUTE, now), "3 hours ago");
        assert_eq!(ago(now - DAY, now), "1 day ago");
        assert_eq!(ago(now - 45 * DAY, now), "1 month ago");
        assert_eq!(ago(now - 2 * YEAR, now), "2 years ago");
    }
}
