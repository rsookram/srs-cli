use time::OffsetDateTime;

pub trait Clock {
    fn now(&self) -> OffsetDateTime;
}

pub struct UtcClock;

impl Clock for UtcClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}
