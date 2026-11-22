use std::time::SystemTime;

pub trait Clock {
    /// Полночь UTC.
    fn today(&self) -> SystemTime;
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
