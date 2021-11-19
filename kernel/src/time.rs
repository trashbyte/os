///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use spin::Mutex;
use chrono::{DateTime, TimeZone, LocalResult};
use chrono::offset::Utc;
use core::sync::atomic::{AtomicU64, Ordering};
use core::ops::Add;
use core::time::Duration;

pub static TIME_START_SECS: Mutex<u64> = Mutex::new(0);

#[derive(Debug, Clone, Copy)]
pub enum DateTimeError {
    RtcInvalid(u64),
    AmbiguousTime(DateTime<Utc>, DateTime<Utc>)
}

pub static NANOS: AtomicU64 = AtomicU64::new(0);

// This is called <PIT freq> times per second in the PIT timer interrupt handler.
// This needs to be as fast as possible
pub(crate) fn pit_tick() {
    const NS_IN_TEN_MS: u64 = 10_000_000;
    NANOS.fetch_add(NS_IN_TEN_MS, Ordering::Relaxed);
}

pub fn get_current_time() -> Result<DateTime<Utc>, DateTimeError> {
    let current_time_secs = crate::arch::rtc::Rtc::new().time();
    match Utc.timestamp_opt(current_time_secs as i64, 0) {
        LocalResult::None => Err(DateTimeError::RtcInvalid(current_time_secs)),
        LocalResult::Ambiguous(a, b) => Err(DateTimeError::AmbiguousTime(a, b)),
        LocalResult::Single(t) => Ok(t),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Instant {
    timestamp_nanos: u64,
}

impl Instant {
    /// Construct a new `Instant` from the given
    /// seconds (absolute since epoch) and
    /// nanoseconds (of the current second) values.
    pub const fn new(timestamp_nanos: u64) -> Self {
        Self { timestamp_nanos }
    }

    /// Returns an `Instant` representing the current time.
    ///
    /// NOTE: In the current implementation, this instant is
    /// only accurate to 10 milliseconds.
    pub fn now() -> Self {
        Instant::new(NANOS.load(Ordering::Relaxed))
    }

    /// Returns a duration representing the time between `self` and `other`.
    /// This difference is directional; it's the time *until* `other`.
    /// This means if `other` is before or equal to `self` the result will be zero.
    pub fn until(&self, other: Instant) -> Duration {
        let diff = (other.timestamp_nanos as i128) - (self.timestamp_nanos as i128);
        Duration::from_nanos(diff.max(0) as u64)
    }
}
impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, dur: Duration) -> Self::Output {
        Instant {
            timestamp_nanos: self.timestamp_nanos + dur.as_nanos() as u64
        }
    }
}
