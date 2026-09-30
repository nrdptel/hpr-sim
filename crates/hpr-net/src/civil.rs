//! UTC calendar dates for URLs and answers: Hinnant's `civil_from_days` and `days_from_civil`
//! (<https://howardhinnant.github.io/date_algorithms.html>, public domain).

/// Seconds in a day.
const DAY_S: i64 = 86_400;

/// The UTC date and hour of `unix_s`, seconds since the Unix epoch: `(year, month, day, hour)`.
pub(crate) fn date_hour(unix_s: i64) -> (i64, i64, i64, i64) {
    let days = unix_s.div_euclid(DAY_S);
    let hour = unix_s.rem_euclid(DAY_S) / 3_600;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, hour)
}

/// Seconds since the Unix epoch at the start of a UTC date, or `None` for a month outside 1 to
/// 12 or a day outside the month.
pub(crate) fn unix_day_start(year: i64, month: i64, day: i64) -> Option<i64> {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=month_days).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146_097 + doe - 719_468) * DAY_S)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_match_known_days_both_ways() {
        // (seconds, year, month, day, hour): the epoch, a leap day, the last hour of a century
        // that is a leap year, and the recordings' days.
        for (unix_s, y, m, d, h) in [
            (0, 1970, 1, 1, 0),
            (951_782_400, 2000, 2, 29, 0),
            (978_303_600, 2000, 12, 31, 23),
            (1_750_507_200, 2025, 6, 21, 12),
            (1_736_942_400, 2025, 1, 15, 12),
        ] {
            assert_eq!(date_hour(unix_s), (y, m, d, h));
            assert_eq!(unix_day_start(y, m, d), Some(unix_s - h * 3_600));
        }
    }

    #[test]
    fn every_day_of_four_centuries_round_trips() {
        // 1970 to 2370: every leap rule, and each day's start maps back to its own date.
        let mut unix_s = 0;
        while unix_s < 146_097 * DAY_S {
            let (y, m, d, h) = date_hour(unix_s);
            assert_eq!(h, 0);
            assert_eq!(unix_day_start(y, m, d), Some(unix_s));
            unix_s += DAY_S;
        }
    }

    #[test]
    fn impossible_dates_are_refused() {
        assert_eq!(unix_day_start(2025, 2, 29), None);
        assert_eq!(unix_day_start(1900, 2, 29), None);
        assert!(unix_day_start(2000, 2, 29).is_some());
        assert_eq!(unix_day_start(2025, 4, 31), None);
        assert_eq!(unix_day_start(2025, 13, 1), None);
        assert_eq!(unix_day_start(2025, 0, 1), None);
        assert_eq!(unix_day_start(2025, 1, 0), None);
    }
}
