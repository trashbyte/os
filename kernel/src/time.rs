///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use spin::Mutex;
use chrono::{DateTime, TimeZone, LocalResult};
use chrono::offset::Utc;

#[derive(Debug, Clone, Copy)]
pub enum DateTimeError {
    RtcInvalid(u64),
    AmbiguousTime(DateTime<Utc>, DateTime<Utc>)
}

pub static TIME_START: Mutex<(u64, u64)> = Mutex::new((0, 0));

pub fn get_current_time() -> Result<DateTime<Utc>, DateTimeError> {
    let current_time_secs = crate::arch::rtc::Rtc::new().time();
    match Utc.timestamp_opt(current_time_secs as i64, 0) {
        LocalResult::None => Err(DateTimeError::RtcInvalid(current_time_secs)),
        LocalResult::Ambiguous(a, b) => Err(DateTimeError::AmbiguousTime(a, b)),
        LocalResult::Single(t) => Ok(t),
    }
}
