//! Bar timeframes and timestamp alignment.
//!
//! A [`Timeframe`] is a `(count, unit)` pair such as "5 minutes" or
//! "3 months". Use [`open_time`](Timeframe::open_time) to snap a
//! Unix-μs [`Timestamp`] down to the start of its containing bar, and
//! [`close_time`](Timeframe::close_time) for the last μs before the
//! next bar starts.

use std::{fmt, num::NonZero};

use crate::Timestamp;

/// Time-period unit. Building block for [`Timeframe`].
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// A bar duration, expressed as `count` × [`TimeUnit`].
///
/// Aligns timestamps to bar boundaries:
/// - **Sub-day** units ([`Second`](TimeUnit::Second), [`Minute`](TimeUnit::Minute),
///   [`Hour`](TimeUnit::Hour)) align to the Unix epoch (1970-01-01).
/// - [`Day`](TimeUnit::Day) and [`Week`](TimeUnit::Week) align to Monday.
/// - [`Month`](TimeUnit::Month) and [`Year`](TimeUnit::Year) align to calendar
///   months and years. Multi-month periods are epoch-anchored from January
///   1970, matching calendar quarters/halves for any N dividing 12.
///
/// `close_time(t) + 1 == open_time` of the next period, bars form a contiguous,
/// non-overlapping cover of the timeline.
///
/// # Example
///
/// ```
/// use quantedge_core::Timeframe;
///
/// // Mon Apr 28 2025 00:05:30.123 UTC
/// let ts = 1_745_798_730_123_000;
/// assert_eq!(Timeframe::HOUR_1.open_time(ts),  1_745_798_400_000_000); // 00:00:00
/// assert_eq!(Timeframe::HOUR_1.close_time(ts), 1_745_801_999_999_999); // 00:59:59.999999
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Timeframe {
    unit: TimeUnit,
    count: NonZero<u64>,
    // Pre-computed dispatch value, set in `new`:
    // - Fixed units (Second..Week): period in μs (`count * unit_micros`).
    // - Month: `count`.
    // - Year:  `count * 12` (total months).
    // Dual-purpose to keep the struct at one cache line and remove
    // the per-call multiplication in the hot path.
    period: u64,
}

const SEC_IN_MICROS: u64 = 1_000_000;
const MIN_IN_MICROS: u64 = 60 * SEC_IN_MICROS;
const HOUR_IN_MICROS: u64 = 60 * MIN_IN_MICROS;
const DAY_IN_MICROS: u64 = 24 * HOUR_IN_MICROS;
const WEEK_IN_MICROS: u64 = 7 * DAY_IN_MICROS;

/// Unix epoch (1970-01-01) starts on Thursday, for proper open and close time calulations
/// for Day and Week based timeframes we need to remove 4 day from the timestamp value.
const EPOCH_TO_MONDAY_OFFSET: u64 = 4 * DAY_IN_MICROS;

impl Timeframe {
    /// 1-second bars.
    pub const SEC_1: Self = Self::new(NonZero::new(1).unwrap(), TimeUnit::Second);
    /// 5-second bars.
    pub const SEC_5: Self = Self::new(NonZero::new(5).unwrap(), TimeUnit::Second);
    /// 10-second bars.
    pub const SEC_10: Self = Self::new(NonZero::new(10).unwrap(), TimeUnit::Second);
    /// 15-second bars.
    pub const SEC_15: Self = Self::new(NonZero::new(15).unwrap(), TimeUnit::Second);
    /// 1-minute bars.
    pub const MIN_1: Self = Self::new(NonZero::new(1).unwrap(), TimeUnit::Minute);
    /// 3-minute bars.
    pub const MIN_3: Self = Self::new(NonZero::new(3).unwrap(), TimeUnit::Minute);
    /// 5-minute bars.
    pub const MIN_5: Self = Self::new(NonZero::new(5).unwrap(), TimeUnit::Minute);
    /// 15-minute bars.
    pub const MIN_15: Self = Self::new(NonZero::new(15).unwrap(), TimeUnit::Minute);
    /// 30-minute bars.
    pub const MIN_30: Self = Self::new(NonZero::new(30).unwrap(), TimeUnit::Minute);
    /// 1-hour bars.
    pub const HOUR_1: Self = Self::new(NonZero::new(1).unwrap(), TimeUnit::Hour);
    /// 2-hour bars.
    pub const HOUR_2: Self = Self::new(NonZero::new(2).unwrap(), TimeUnit::Hour);
    /// 4-hour bars.
    pub const HOUR_4: Self = Self::new(NonZero::new(4).unwrap(), TimeUnit::Hour);
    /// 6-hour bars.
    pub const HOUR_6: Self = Self::new(NonZero::new(6).unwrap(), TimeUnit::Hour);
    /// 8-hour bars.
    pub const HOUR_8: Self = Self::new(NonZero::new(8).unwrap(), TimeUnit::Hour);
    /// 12-hour bars.
    pub const HOUR_12: Self = Self::new(NonZero::new(12).unwrap(), TimeUnit::Hour);
    /// Daily bars (Monday-aligned).
    pub const DAY_1: Self = Self::new(NonZero::new(1).unwrap(), TimeUnit::Day);
    /// 3-day bars (Monday-aligned).
    pub const DAY_3: Self = Self::new(NonZero::new(3).unwrap(), TimeUnit::Day);
    /// 5-day bars (Monday-aligned).
    pub const DAY_5: Self = Self::new(NonZero::new(5).unwrap(), TimeUnit::Day);
    /// Weekly bars (Monday-aligned).
    pub const WEEK_1: Self = Self::new(NonZero::new(1).unwrap(), TimeUnit::Week);
    /// Monthly bars.
    pub const MONTH_1: Self = Self::new(NonZero::new(1).unwrap(), TimeUnit::Month);
    /// Bi-monthly bars (Jan-Feb, Mar-Apr, ...).
    pub const MONTH_2: Self = Self::new(NonZero::new(2).unwrap(), TimeUnit::Month);
    /// Quarterly bars (Q1=Jan-Mar, Q2=Apr-Jun, Q3=Jul-Sep, Q4=Oct-Dec).
    pub const MONTH_3: Self = Self::new(NonZero::new(3).unwrap(), TimeUnit::Month);
    /// Semi-annual bars (H1=Jan-Jun, H2=Jul-Dec).
    pub const MONTH_6: Self = Self::new(NonZero::new(6).unwrap(), TimeUnit::Month);
    /// Yearly bars (calendar year).
    pub const YEAR_1: Self = Self::new(NonZero::new(1).unwrap(), TimeUnit::Year);

    /// Constructs a [`Timeframe`] from a `count` and `unit`, canonicalizing
    /// where possible: `60s -> 1 minute`, `60min -> 1 hour`, `24h -> 1 day`,
    /// `7d -> 1 week`. Rules apply recursively.
    // Each `NonZero::new(n / k).expect("always positive")` is guarded by the
    // preceding `n.is_multiple_of(k)` arm, which for `n >= 1` and `k >= 2`
    // guarantees `n / k >= 1`, the `.expect` is unreachable.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub const fn new(count: NonZero<u64>, unit: TimeUnit) -> Self {
        let n = count.get();

        match unit {
            TimeUnit::Second if n.is_multiple_of(60) => Self::new(
                NonZero::new(n / 60).expect("always positive"),
                TimeUnit::Minute,
            ),
            TimeUnit::Minute if n.is_multiple_of(60) => Self::new(
                NonZero::new(n / 60).expect("always positive"),
                TimeUnit::Hour,
            ),
            TimeUnit::Hour if n.is_multiple_of(24) => Self::new(
                NonZero::new(n / 24).expect("always positive"),
                TimeUnit::Day,
            ),
            TimeUnit::Day if n.is_multiple_of(7) => Self::new(
                NonZero::new(n / 7).expect("always positive"),
                TimeUnit::Week,
            ),
            TimeUnit::Second => Self {
                count,
                unit,
                period: n * SEC_IN_MICROS,
            },
            TimeUnit::Minute => Self {
                count,
                unit,
                period: n * MIN_IN_MICROS,
            },
            TimeUnit::Hour => Self {
                count,
                unit,
                period: n * HOUR_IN_MICROS,
            },
            TimeUnit::Day => Self {
                count,
                unit,
                period: n * DAY_IN_MICROS,
            },
            TimeUnit::Week => Self {
                count,
                unit,
                period: n * WEEK_IN_MICROS,
            },
            TimeUnit::Month => Self {
                count,
                unit,
                period: n,
            },
            TimeUnit::Year => Self {
                count,
                unit,
                period: n * 12,
            },
        }
    }

    /// Number of [`unit`](Self::unit)s in this timeframe.
    ///
    /// Always non-zero. Reflects the canonicalization done by
    /// [`new`](Self::new): `Timeframe::new(NonZero::new(120).unwrap(),
    /// TimeUnit::Second)` reports `count() == 2`, `unit() == Minute`.
    #[must_use]
    pub fn count(&self) -> NonZero<u64> {
        self.count
    }

    /// The [`TimeUnit`] of this timeframe.
    ///
    /// May differ from the unit passed to [`new`](Self::new) if the count
    /// canonicalized into a larger unit (e.g. `7 days -> 1 week`).
    #[must_use]
    pub fn unit(&self) -> TimeUnit {
        self.unit
    }

    /// Start of the bar period containing `timestamp` (inclusive).
    ///
    /// Idempotent: `open_time(open_time(t)) == open_time(t)`.
    ///
    /// If you need both [`open_time`](Self::open_time) and
    /// [`close_time`](Self::close_time) for the same `timestamp`, prefer
    /// [`bounds`](Self::bounds), it shares work between the two.
    ///
    /// # Precondition
    /// Day/Week paths require `timestamp >= EPOCH_TO_MONDAY_OFFSET`
    /// (Jan 5 1970 00:00 UTC). Earlier timestamps underflow `u64`;
    /// checked in debug, wraps in release.
    #[must_use]
    pub fn open_time(&self, timestamp: Timestamp) -> Timestamp {
        match self.unit {
            TimeUnit::Second | TimeUnit::Minute | TimeUnit::Hour => {
                timestamp - timestamp % self.period
            }
            TimeUnit::Day | TimeUnit::Week => {
                debug_assert!(timestamp >= EPOCH_TO_MONDAY_OFFSET);
                let shifted = timestamp - EPOCH_TO_MONDAY_OFFSET;
                shifted - shifted % self.period + EPOCH_TO_MONDAY_OFFSET
            }
            TimeUnit::Month if self.period == 1 => month_start_micros(timestamp),
            TimeUnit::Month | TimeUnit::Year => {
                n_month_start_micros(timestamp, self.period_months())
            }
        }
    }

    /// Last μs of the bar period containing `timestamp`, the moment just
    /// before the next period begins.
    ///
    /// `close_time(t) + 1` equals the [`open_time`](Self::open_time) of the
    /// next period, so adjacent bars form a contiguous, non-overlapping cover
    /// of the timeline.
    ///
    /// If you need both [`open_time`](Self::open_time) and
    /// [`close_time`](Self::close_time) for the same `timestamp`, prefer
    /// [`bounds`](Self::bounds), it shares work between the two.
    ///
    /// # Precondition
    /// Same as [`open_time`](Self::open_time).
    #[must_use]
    pub fn close_time(&self, timestamp: Timestamp) -> Timestamp {
        match self.unit {
            TimeUnit::Second | TimeUnit::Minute | TimeUnit::Hour => {
                timestamp - timestamp % self.period + self.period - 1
            }
            TimeUnit::Day | TimeUnit::Week => {
                debug_assert!(timestamp >= EPOCH_TO_MONDAY_OFFSET);
                let shifted = timestamp - EPOCH_TO_MONDAY_OFFSET;
                shifted - shifted % self.period + self.period - 1 + EPOCH_TO_MONDAY_OFFSET
            }
            TimeUnit::Month if self.period == 1 => month_close_micros(timestamp),
            TimeUnit::Month | TimeUnit::Year => {
                n_month_close_micros(timestamp, self.period_months())
            }
        }
    }

    /// Returns `(open_time, close_time)` for the period containing `timestamp`,
    /// sharing computation between the two.
    ///
    /// For uniform units (Second..Week) the modulo is computed once. For
    /// [`Month`](TimeUnit::Month) and [`Year`](TimeUnit::Year) the
    /// `civil_from_days` forward pass is shared, meaningful when the same
    /// `timestamp` is mapped to many timeframes per tick.
    ///
    /// # Precondition
    /// Same as [`open_time`](Self::open_time).
    #[must_use]
    pub fn bounds(&self, timestamp: Timestamp) -> (Timestamp, Timestamp) {
        match self.unit {
            TimeUnit::Second | TimeUnit::Minute | TimeUnit::Hour => {
                let open = timestamp - timestamp % self.period;
                (open, open + self.period - 1)
            }
            TimeUnit::Day | TimeUnit::Week => {
                debug_assert!(timestamp >= EPOCH_TO_MONDAY_OFFSET);
                let shifted = timestamp - EPOCH_TO_MONDAY_OFFSET;
                let open = shifted - shifted % self.period + EPOCH_TO_MONDAY_OFFSET;
                (open, open + self.period - 1)
            }
            TimeUnit::Month if self.period == 1 => month_bounds_micros(timestamp),
            TimeUnit::Month | TimeUnit::Year => {
                n_month_bounds_micros(timestamp, self.period_months())
            }
        }
    }

    /// Narrows `self.period` (total months for Month/Year) to `u32` for the
    /// Hinnant helpers. Realistic counts are small (<= 12 per unit); any
    /// pathological caller passing `count > u32::MAX / 12` would get garbage,
    /// but never UB.
    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn period_months(&self) -> u32 {
        self.period as u32
    }
}

/// Binance-style compact notation: `5m`, `1h`, `1d`, `1w`, `3M`, `1Y`.
/// Uppercase `M`/`Y` disambiguate month/year from minute.
impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = match self.unit {
            TimeUnit::Second => 's',
            TimeUnit::Minute => 'm',
            TimeUnit::Hour => 'h',
            TimeUnit::Day => 'd',
            TimeUnit::Week => 'w',
            TimeUnit::Month => 'M',
            TimeUnit::Year => 'Y',
        };
        write!(f, "{}{}", self.count.get(), suffix)
    }
}

/// Days in the month, indexed by Hinnant's `mp`
/// (0=Mar, 1=Apr, 2=May, 3=Jun, 4=Jul, 5=Aug, 6=Sep, 7=Oct, 8=Nov, 9=Dec, 10=Jan, 11=Feb).
/// Feb entry is a placeholder — callers add the leap-year bit for `mp == 11`.
const MONTH_LEN: [u32; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 28];

/// Hinnant's `civil_from_days` forward pass: Unix-μs -> `(era, year-of-era, mp)`.
/// `mp` is Hinnant's 0-indexed shifted month (0=Mar ... 11=Feb). Callers derive
/// whatever fields they need from these three values.
///
/// Adapted from <https://howardhinnant.github.io/date_algorithms.html>. Unix
/// timestamps are non-negative, so the sign branch is omitted.
#[inline]
fn civil_from_days_core(ts: Timestamp) -> (u32, u32, u32) {
    let z = (ts / DAY_IN_MICROS) as u32 + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;

    (era, yoe, mp)
}

/// Decompose a Unix-μs timestamp into `(start-of-month serial day,
/// Hinnant month-index, civil year of the Feb month)`.
///
/// `civil_year_feb` is only meaningful when `mp == 11` (February) and
/// is used for the leap-year check in `month_length`.
///
/// The reverse `days_from_civil` pass is skipped: `(153*mp + 2) / 5` is
/// exactly the `doy − d + 1` term baked into `civil_from_days`, giving
/// start-of-month day-of-year directly.
// `som_doe` (day-of-era) vs `som_doy` (day-of-year) are Hinnant's canonical
// names — renaming them away from the paper harms readability.
#[allow(clippy::similar_names)]
#[inline]
fn month_start_parts(ts: Timestamp) -> (u32, u32, u32) {
    let (era, yoe, mp) = civil_from_days_core(ts);
    let som_doy = (153 * mp + 2) / 5;
    let som_doe = 365 * yoe + yoe / 4 - yoe / 100 + som_doy;
    let som_z = era * 146_097 + som_doe - 719_468;
    // Matches Hinnant's civil-year adjustment `y + (m <= 2)` for Feb only.
    let civil_year_feb = yoe + era * 400 + 1;

    (som_z, mp, civil_year_feb)
}

#[inline]
fn is_leap(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[inline]
fn month_length(mp: u32, civil_year_feb: u32) -> u32 {
    MONTH_LEN[mp as usize] + u32::from(mp == 11 && is_leap(civil_year_feb))
}

#[inline]
fn month_start_micros(ts: Timestamp) -> Timestamp {
    let (som_z, _, _) = month_start_parts(ts);

    u64::from(som_z) * DAY_IN_MICROS
}

#[inline]
fn month_close_micros(ts: Timestamp) -> Timestamp {
    let (som_z, mp, civil_year_feb) = month_start_parts(ts);
    let len = month_length(mp, civil_year_feb);

    (u64::from(som_z) + u64::from(len)) * DAY_IN_MICROS - 1
}

/// Combined `(open, close)` for the calendar month containing `ts`.
/// Shares the `civil_from_days` forward pass between both halves.
#[inline]
fn month_bounds_micros(ts: Timestamp) -> (Timestamp, Timestamp) {
    let (som_z, mp, civil_year_feb) = month_start_parts(ts);
    let len = month_length(mp, civil_year_feb);
    let open = u64::from(som_z) * DAY_IN_MICROS;
    let close = (u64::from(som_z) + u64::from(len)) * DAY_IN_MICROS - 1;

    (open, close)
}

/// Civil `(year, month_1_indexed)` containing `ts`.
#[inline]
fn civil_year_month(ts: Timestamp) -> (u32, u32) {
    let (era, yoe, mp) = civil_from_days_core(ts);
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + u32::from(m <= 2);

    (y, m)
}

/// Days since Unix epoch for the 1st of `(year, month)`. Hinnant's
/// `days_from_civil` with `d = 1` folded into the formula. Requires
/// `year >= 1970`.
#[inline]
fn days_from_civil_month_start(y: u32, m: u32) -> u32 {
    let ay = y - u32::from(m <= 2);
    let era = ay / 400;
    let yoe = ay - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;

    era * 146_097 + doe - 719_468
}

/// Epoch-anchored N-month alignment: returns `(start_year, start_month)`
/// and `(next_year, next_month)` of the N-month period containing `ts`.
/// Aligned on months-since-1970-01 mod N. For N dividing 12, matches
/// calendar-year-anchored quarters/halves.
#[inline]
fn n_month_period(ts: Timestamp, n: u32) -> (u32, u32, u32, u32) {
    let (y, m) = civil_year_month(ts);
    let mse = (y - 1970) * 12 + (m - 1);
    let aligned = (mse / n) * n;
    let next = aligned + n;
    let sy = 1970 + aligned / 12;
    let sm = aligned % 12 + 1;
    let ny = 1970 + next / 12;
    let nm = next % 12 + 1;

    (sy, sm, ny, nm)
}

#[inline]
fn n_month_start_micros(ts: Timestamp, n: u32) -> Timestamp {
    let (sy, sm, _, _) = n_month_period(ts, n);

    u64::from(days_from_civil_month_start(sy, sm)) * DAY_IN_MICROS
}

#[inline]
fn n_month_close_micros(ts: Timestamp, n: u32) -> Timestamp {
    let (_, _, ny, nm) = n_month_period(ts, n);

    u64::from(days_from_civil_month_start(ny, nm)) * DAY_IN_MICROS - 1
}

/// Combined `(open, close)` for the N-month period containing `ts`. Shares
/// the forward Hinnant pass that `n_month_period` runs internally — saves
/// one `civil_from_days` pass vs. calling `_start` and `_close` separately.
#[inline]
fn n_month_bounds_micros(ts: Timestamp, n: u32) -> (Timestamp, Timestamp) {
    let (sy, sm, ny, nm) = n_month_period(ts, n);
    let open = u64::from(days_from_civil_month_start(sy, sm)) * DAY_IN_MICROS;
    let close = u64::from(days_from_civil_month_start(ny, nm)) * DAY_IN_MICROS - 1;

    (open, close)
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    /// `(timeframe, input, expected_open, expected_close)`. Each row drives
    /// `boundaries` (exact-value check) and `period_invariants`
    /// (containment, idempotence, contiguity) for that input.
    #[rustfmt::skip]
    const CASES: &[(Timeframe, &str, &str, &str)] = &[
        // Sub-day epoch-aligned
        (Timeframe::MIN_1,   "2025-05-18T19:27:27.196+00:00", "2025-05-18T19:27:00+00:00", "2025-05-18T19:27:59.999999+00:00"),
        (Timeframe::MIN_3,   "2025-05-18T19:27:27.196+00:00", "2025-05-18T19:27:00+00:00", "2025-05-18T19:29:59.999999+00:00"),
        (Timeframe::MIN_5,   "2025-05-18T19:27:27.196+00:00", "2025-05-18T19:25:00+00:00", "2025-05-18T19:29:59.999999+00:00"),
        (Timeframe::MIN_15,  "2025-05-18T19:27:27.196+00:00", "2025-05-18T19:15:00+00:00", "2025-05-18T19:29:59.999999+00:00"),
        (Timeframe::MIN_30,  "2025-05-18T19:27:27.196+00:00", "2025-05-18T19:00:00+00:00", "2025-05-18T19:29:59.999999+00:00"),
        (Timeframe::HOUR_1,  "2025-05-18T19:27:27.196+00:00", "2025-05-18T19:00:00+00:00", "2025-05-18T19:59:59.999999+00:00"),
        (Timeframe::HOUR_2,  "2025-05-18T19:27:27.196+00:00", "2025-05-18T18:00:00+00:00", "2025-05-18T19:59:59.999999+00:00"),
        (Timeframe::HOUR_4,  "2025-05-18T19:27:27.196+00:00", "2025-05-18T16:00:00+00:00", "2025-05-18T19:59:59.999999+00:00"),
        (Timeframe::HOUR_6,  "2025-05-18T19:27:27.196+00:00", "2025-05-18T18:00:00+00:00", "2025-05-18T23:59:59.999999+00:00"),
        (Timeframe::HOUR_8,  "2025-05-18T19:27:27.196+00:00", "2025-05-18T16:00:00+00:00", "2025-05-18T23:59:59.999999+00:00"),
        (Timeframe::HOUR_12, "2025-05-18T19:27:27.196+00:00", "2025-05-18T12:00:00+00:00", "2025-05-18T23:59:59.999999+00:00"),
        (Timeframe::DAY_1,   "2025-05-18T11:27:27.196+00:00", "2025-05-18T00:00:00+00:00", "2025-05-18T23:59:59.999999+00:00"),
        (Timeframe::DAY_3,   "2025-05-05T19:01:11.796+00:00", "2025-05-04T00:00:00+00:00", "2025-05-06T23:59:59.999999+00:00"),
        // Monday-aligned weeks
        (Timeframe::WEEK_1,  "2025-04-30T19:27:27.196+00:00", "2025-04-28T00:00:00+00:00", "2025-05-04T23:59:59.999999+00:00"),
        (Timeframe::WEEK_1,  "2025-05-18T19:27:27.196+00:00", "2025-05-12T00:00:00+00:00", "2025-05-18T23:59:59.999999+00:00"),
        // Monthly
        (Timeframe::MONTH_1, "2025-05-18T19:27:27.196+00:00",    "2025-05-01T00:00:00+00:00", "2025-05-31T23:59:59.999999+00:00"),
        (Timeframe::MONTH_1, "2025-01-15T12:34:56.789+00:00",    "2025-01-01T00:00:00+00:00", "2025-01-31T23:59:59.999999+00:00"),
        (Timeframe::MONTH_1, "2025-12-15T12:34:56.789+00:00",    "2025-12-01T00:00:00+00:00", "2025-12-31T23:59:59.999999+00:00"),
        (Timeframe::MONTH_1, "2025-04-01T00:00:00+00:00",        "2025-04-01T00:00:00+00:00", "2025-04-30T23:59:59.999999+00:00"),
        (Timeframe::MONTH_1, "2025-04-30T23:59:59.999999+00:00", "2025-04-01T00:00:00+00:00", "2025-04-30T23:59:59.999999+00:00"),
        // Feb leap-year matrix
        (Timeframe::MONTH_1, "2024-02-20T12:00:00+00:00", "2024-02-01T00:00:00+00:00", "2024-02-29T23:59:59.999999+00:00"),
        (Timeframe::MONTH_1, "2025-02-20T12:00:00+00:00", "2025-02-01T00:00:00+00:00", "2025-02-28T23:59:59.999999+00:00"),
        (Timeframe::MONTH_1, "2100-02-15T00:00:00+00:00", "2100-02-01T00:00:00+00:00", "2100-02-28T23:59:59.999999+00:00"),
        (Timeframe::MONTH_1, "2000-02-15T00:00:00+00:00", "2000-02-01T00:00:00+00:00", "2000-02-29T23:59:59.999999+00:00"),
        // Bi-monthly (Jan-Feb, Mar-Apr, May-Jun, Jul-Aug, Sep-Oct, Nov-Dec)
        (Timeframe::MONTH_2, "2025-02-10T00:00:00+00:00", "2025-01-01T00:00:00+00:00", "2025-02-28T23:59:59.999999+00:00"),
        (Timeframe::MONTH_2, "2025-06-15T12:00:00+00:00", "2025-05-01T00:00:00+00:00", "2025-06-30T23:59:59.999999+00:00"),
        (Timeframe::MONTH_2, "2024-01-15T00:00:00+00:00", "2024-01-01T00:00:00+00:00", "2024-02-29T23:59:59.999999+00:00"),
        // Quarterly (Q1=Jan-Mar, Q2=Apr-Jun, Q3=Jul-Sep, Q4=Oct-Dec)
        (Timeframe::MONTH_3, "2025-02-15T00:00:00+00:00",        "2025-01-01T00:00:00+00:00", "2025-03-31T23:59:59.999999+00:00"),
        (Timeframe::MONTH_3, "2025-04-28T00:00:00+00:00",        "2025-04-01T00:00:00+00:00", "2025-06-30T23:59:59.999999+00:00"),
        (Timeframe::MONTH_3, "2025-12-31T23:59:59.999999+00:00", "2025-10-01T00:00:00+00:00", "2025-12-31T23:59:59.999999+00:00"),
        (Timeframe::MONTH_3, "2024-02-29T12:00:00+00:00",        "2024-01-01T00:00:00+00:00", "2024-03-31T23:59:59.999999+00:00"),
        // Semi-annual (H1=Jan-Jun, H2=Jul-Dec)
        (Timeframe::MONTH_6, "2025-04-15T00:00:00+00:00", "2025-01-01T00:00:00+00:00", "2025-06-30T23:59:59.999999+00:00"),
        (Timeframe::MONTH_6, "2025-11-30T00:00:00+00:00", "2025-07-01T00:00:00+00:00", "2025-12-31T23:59:59.999999+00:00"),
        // Yearly
        (Timeframe::YEAR_1,  "2025-07-15T12:00:00+00:00",        "2025-01-01T00:00:00+00:00", "2025-12-31T23:59:59.999999+00:00"),
        (Timeframe::YEAR_1,  "2024-12-31T23:59:59.999999+00:00", "2024-01-01T00:00:00+00:00", "2024-12-31T23:59:59.999999+00:00"),
        (Timeframe::YEAR_1,  "2024-03-01T00:00:00+00:00",        "2024-01-01T00:00:00+00:00", "2024-12-31T23:59:59.999999+00:00"),
    ];

    #[test]
    fn boundaries() {
        for &(tf, input, open, close) in CASES {
            let t = parse(input);
            let expected = (parse(open), parse(close));
            assert_eq!(
                (tf.open_time(t), tf.close_time(t)),
                expected,
                "open/close mismatch: tf={tf:?} input={input}"
            );
            assert_eq!(
                tf.bounds(t),
                expected,
                "bounds mismatch: tf={tf:?} input={input}"
            );
        }
    }

    #[test]
    fn period_invariants() {
        for &(tf, input, _, _) in CASES {
            let t = parse(input);
            let open = tf.open_time(t);
            let close = tf.close_time(t);

            assert!(
                open <= t && t <= close,
                "containment broken: tf={tf:?} input={input} open={open} close={close}"
            );
            assert_eq!(
                tf.open_time(open),
                open,
                "open_time not idempotent: tf={tf:?} input={input}"
            );
            assert_eq!(
                tf.close_time(open),
                close,
                "close_time inconsistent within period: tf={tf:?} input={input}"
            );
            assert_eq!(
                tf.open_time(close + 1),
                close + 1,
                "no contiguity at close+1: tf={tf:?} input={input}"
            );
        }
    }

    #[test]
    fn user_example_literal_micros() {
        // 1_745_798_400_000_000 μs = Mon Apr 28 2025 00:00:00 UTC
        // 1_743_465_600_000_000 μs = Tue Apr 01 2025 00:00:00 UTC
        assert_eq!(
            Timeframe::MONTH_1.open_time(1_745_798_400_000_000),
            1_743_465_600_000_000
        );
    }

    #[test]
    fn display_covers_every_unit() {
        assert_eq!(Timeframe::SEC_5.to_string(), "5s");
        assert_eq!(Timeframe::MIN_1.to_string(), "1m");
        assert_eq!(Timeframe::MIN_15.to_string(), "15m");
        assert_eq!(Timeframe::HOUR_4.to_string(), "4h");
        assert_eq!(Timeframe::DAY_1.to_string(), "1d");
        assert_eq!(Timeframe::WEEK_1.to_string(), "1w");
        assert_eq!(Timeframe::MONTH_1.to_string(), "1M");
        assert_eq!(Timeframe::MONTH_3.to_string(), "3M");
        assert_eq!(Timeframe::YEAR_1.to_string(), "1Y");
    }

    #[test]
    fn display_reflects_canonicalization() {
        // 120 seconds canonicalizes to 2 minutes.
        let tf = Timeframe::new(NonZero::new(120).unwrap(), TimeUnit::Second);
        assert_eq!(tf.to_string(), "2m");
        // 168 hours canonicalizes to 1 week.
        let tf = Timeframe::new(NonZero::new(168).unwrap(), TimeUnit::Hour);
        assert_eq!(tf.to_string(), "1w");
    }

    fn parse(s: &str) -> u64 {
        DateTime::parse_from_rfc3339(s)
            .unwrap()
            .timestamp_micros()
            .cast_unsigned()
    }
}
