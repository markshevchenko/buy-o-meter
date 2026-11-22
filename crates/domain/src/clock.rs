use std::time::{Duration, SystemTime};

use crate::error::DomainError;

pub trait Clock {
    fn today(&self) -> SystemTime;
}

pub fn date(year: i32, month: i32, day: i32) -> Result<SystemTime, DomainError> {
    let days = civil_days(year, month, day).ok_or(DomainError::InvalidDate)?;
    system_time_from_days(days).ok_or(DomainError::InvalidDate)
}

pub fn add_days(date: SystemTime, days: i64) -> Result<SystemTime, DomainError> {
    let seconds = days
        .unsigned_abs()
        .checked_mul(86_400)
        .ok_or(DomainError::InvalidDate)?;
    let shift = Duration::from_secs(seconds);
    let shifted = if days < 0 {
        date.checked_sub(shift)
    } else {
        date.checked_add(shift)
    };
    shifted.ok_or(DomainError::InvalidDate)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedClock {
    today: SystemTime,
}

impl FixedClock {
    #[must_use]
    pub const fn new(today: SystemTime) -> Self {
        Self { today }
    }
}

impl Clock for FixedClock {
    fn today(&self) -> SystemTime {
        self.today
    }
}

fn civil_days(year: i32, month: i32, day: i32) -> Option<i64> {
    let month = u8::try_from(month).ok()?;
    let day = u8::try_from(day).ok()?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    let divides = |n| year.rem_euclid(n) == 0;
    divides(4) && (!divides(100) || divides(400))
}

fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = (year - era * 400) as u64;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * u64::from(month_index) + 2) / 5 + u64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era as i64 - 719_468
}

fn system_time_from_days(days: i64) -> Option<SystemTime> {
    let seconds = days.unsigned_abs().checked_mul(86_400)?;
    let shift = Duration::from_secs(seconds);
    if days < 0 {
        SystemTime::UNIX_EPOCH.checked_sub(shift)
    } else {
        SystemTime::UNIX_EPOCH.checked_add(shift)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use crate::error::DomainError;

    use super::{add_days, date};

    #[test]
    fn date_matches_calendar_day_and_shifts_by_days() -> Result<(), DomainError> {
        assert_eq!(date(1970, 1, 1)?, SystemTime::UNIX_EPOCH);
        assert_eq!(
            date(1969, 12, 31)?,
            SystemTime::UNIX_EPOCH - Duration::from_secs(86_400)
        );
        assert_eq!(
            date(1970, 1, 2)?,
            SystemTime::UNIX_EPOCH + Duration::from_secs(86_400)
        );
        assert_eq!(
            date(2000, 1, 1)?,
            SystemTime::UNIX_EPOCH + Duration::from_secs(10_957 * 86_400)
        );
        assert_eq!(add_days(date(2026, 11, 22)?, 1)?, date(2026, 11, 23)?);
        assert_eq!(add_days(date(2026, 11, 22)?, -1)?, date(2026, 11, 21)?);
        assert_eq!(add_days(date(2024, 2, 28)?, 1)?, date(2024, 2, 29)?);
        assert_eq!(add_days(date(2024, 2, 29)?, 1)?, date(2024, 3, 1)?);
        assert_eq!(add_days(date(2000, 2, 28)?, 1)?, date(2000, 2, 29)?);
        assert_eq!(add_days(date(1900, 2, 28)?, 1)?, date(1900, 3, 1)?);
        Ok(())
    }

    #[test]
    fn impossible_day_is_rejected() {
        assert_eq!(date(2023, 2, 29), Err(DomainError::InvalidDate));
        assert_eq!(date(1900, 2, 29), Err(DomainError::InvalidDate));
        assert_eq!(date(2026, 13, 1), Err(DomainError::InvalidDate));
        assert_eq!(date(2026, 0, 1), Err(DomainError::InvalidDate));
        assert_eq!(date(2026, 11, 0), Err(DomainError::InvalidDate));
        assert_eq!(date(2026, 11, 31), Err(DomainError::InvalidDate));
    }
}
