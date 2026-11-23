use std::time::{Duration, SystemTime};

use time::{Date, Month, Time, UtcDateTime};

use crate::error::DomainError;

pub trait Clock {
    fn today(&self) -> SystemTime;
}

pub fn date(year: i32, month: i32, day: i32) -> Result<SystemTime, DomainError> {
    let month = calendar_month(month)?;
    let day = u8::try_from(day).map_err(|_| DomainError::InvalidDate)?;
    let date = Date::from_calendar_date(year, month, day).map_err(|_| DomainError::InvalidDate)?;
    midnight_utc(date).ok_or(DomainError::InvalidDate)
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

fn calendar_month(month: i32) -> Result<Month, DomainError> {
    let month = u8::try_from(month).map_err(|_| DomainError::InvalidDate)?;
    Month::try_from(month).map_err(|_| DomainError::InvalidDate)
}

fn midnight_utc(date: Date) -> Option<SystemTime> {
    let seconds = UtcDateTime::new(date, Time::MIDNIGHT).unix_timestamp();
    let shift = Duration::from_secs(seconds.unsigned_abs());
    if seconds < 0 {
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
