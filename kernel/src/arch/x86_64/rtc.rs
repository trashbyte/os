///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L
// Adapted from RedoxOS's kernel/src/arch/x86_64/device/rtc.rs (MIT License)

use x86_64::instructions::port::Port;


pub fn init_rtc() {
    let mut rtc = Rtc::new();
    (*crate::time::TIME_START.lock()).0 = rtc.time();
}


#[derive(Debug, Clone, Copy)]
#[repr(u8)]
/// A register in the RTC area of CMOS.
pub enum RtcRegister {
    Seconds = 0x0,
    Minutes = 0x1,
    Hours = 0x4, // 0–23 in 24-hour mode, 1–12 in 12-hour mode, highest bit set if pm
    //Weekday = 0x6, // Unreliable, do not use
    DayOfMonth = 0x7,
    Month = 0x8,
    Year = 0x9,
    StatusA = 0xA,
    StatusB = 0xB,
}


/// 24-hour time (1 = 24-hour time, 0 = 12-hour time)
const STATUS_B_24HR_TIME: u8 = 0x2;
/// Number format (1 = binary, 0 = BCD)
const STATUS_B_FORMAT: u8 = 0x4;


/// Convert BCD value to binary
fn cvt_bcd(value: usize) -> usize {
    (value & 0xF) + ((value / 16) * 10)
}


/// Real Time Clock
///
/// Don't worry about global state or duplication, this struct holds very little state.
/// Just construct a new one wherever you need it. (But remember to set NMI appropriately!)
#[derive(Debug)]
pub struct Rtc {
    /// Address I/O port
    addr: Port<u8>,
    /// Data I/O port
    data: Port<u8>,
    /// NMI enable bit
    nmi: bool,
}
impl Rtc {
    /// Create new RTC struct
    pub fn new() -> Self {
        Self {
            addr: Port::<u8>::new(0x70),
            data: Port::<u8>::new(0x71),
            nmi: false,
        }
    }

    /// Read register
    fn read(&mut self, reg: RtcRegister) -> u8 {
        // using RtcRegister ensures that the register value will
        // always be correct, so this should be completely safe.
        unsafe {
            if self.nmi {
                self.addr.write(reg as u8 & 0x7F);
            } else {
                self.addr.write(reg as u8 | 0x80);
            }
            self.data.read()
        }
    }

    /// Write register
    #[allow(dead_code)]
    fn write(&mut self, reg: RtcRegister, value: u8) {
        // using RtcRegister ensures that the register value will
        // always be correct, so this should be completely safe.
        unsafe {
            if self.nmi {
                self.addr.write(reg as u8 & 0x7F);
            } else {
                self.addr.write(reg as u8 | 0x80);
            }
            self.data.write(value);
        }
    }

    /// Get current time immediately (UNIX timestamp in seconds).
    ///
    /// May yield invalid results if called during an RTC update.
    /// Use `RTC::time()` for a safer (but slower) method.
    ///
    /// Marked unsafe not because it can violate memory safety,
    /// but because it may return invalid values.
    pub unsafe fn time_immediate(&mut self) -> u64 {
        let mut second = self.read(RtcRegister::Seconds) as usize;
        let mut minute = self.read(RtcRegister::Minutes) as usize;
        let mut hour = self.read(RtcRegister::Hours) as usize;
        let mut day = self.read(RtcRegister::DayOfMonth) as usize;
        let mut month = self.read(RtcRegister::Month) as usize;
        let mut year = self.read(RtcRegister::Year) as usize;
        let century = 20; // remember to update this in 80 years lol
        let register_b = self.read(RtcRegister::StatusB);

        // convert BCD values to binary if necessary
        if (register_b & STATUS_B_FORMAT) != 4 {
            second = cvt_bcd(second);
            minute = cvt_bcd(minute);
            hour = cvt_bcd(hour & 0x7F) | (hour & 0x80);
            day = cvt_bcd(day);
            month = cvt_bcd(month);
            year = cvt_bcd(year);
        }

        // correct 12-hour time
        if register_b & STATUS_B_24HR_TIME != 2 || hour & 0x80 == 0x80 {
            hour = ((hour & 0x7F) + 12) % 24;
        }

        year += century * 100;

        // Unix time from clock (years since 1970 * 31,536,000 seconds in a year)
        let mut secs: u64 = (year as u64 - 1970) * 31_536_000;

        // Correct for leap days
        let mut leap_days = (year as u64 - 1972) / 4 + 1;
        if year % 4 == 0 && month <= 2 {
            leap_days -= 1;
        }
        secs += leap_days * 86_400;

        // Add seconds for months
        match month {
            2 => secs += 2_678_400,
            3 => secs += 5_097_600,
            4 => secs += 7_776_000,
            5 => secs += 10_368_000,
            6 => secs += 13_046_400,
            7 => secs += 15_638_400,
            8 => secs += 18_316_800,
            9 => secs += 20_995_200,
            10 => secs += 23_587_200,
            11 => secs += 26_265_600,
            12 => secs += 28_857_600,
            _ => (),
        }

        secs += (day as u64 - 1) * 86_400;
        secs += hour as u64 * 3600;
        secs += minute as u64 * 60;
        secs += second as u64;

        // return UNIX timestamp (in seconds)
        secs
    }

    /// Get the current time, checking twice to make sure we
    /// didn't read invalid values during an update.
    pub fn time(&mut self) -> u64 {
        loop {
            unsafe {
                while self.read(RtcRegister::StatusA) & 0x80 == 0x80 {}
                let time1 = self.time_immediate();

                while self.read(RtcRegister::StatusA) & 0x80 == 0x80 {}
                let time2 = self.time_immediate();

                if time1 == time2 { return time1; }
            }
        }
    }
}
