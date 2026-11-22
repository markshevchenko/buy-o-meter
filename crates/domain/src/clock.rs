use crate::date::Date;

pub trait Clock {
    fn today(&self) -> Date;
}

/// Часы с фиксированной датой для тестов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedClock {
    today: Date,
}

impl FixedClock {
    #[must_use]
    pub const fn new(today: Date) -> Self {
        Self { today }
    }
}

impl Clock for FixedClock {
    fn today(&self) -> Date {
        self.today
    }
}
