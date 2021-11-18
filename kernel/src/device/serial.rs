///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#[cfg(target_arch = "x86_64")]
use uart_16550::SerialPort;
#[cfg(target_arch = "aarch64")]
use uart_16550::MmioSerialPort;

use spin::Mutex;
use lazy_static::lazy_static;

#[cfg(target_arch = "x86_64")]
lazy_static! {
    /// The UART serial port mapped at port 0x3F8
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

#[cfg(target_arch = "aarch64")]
lazy_static! {
    pub static ref SERIAL1: Mutex<MmioSerialPort> = {
        let mut serial_port = unsafe { MmioSerialPort::new(0) };
        serial_port.init();
        Mutex::new(serial_port);
        unimplemented!();
    };
}


#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments<'_>) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}


/// Print a string to SERIAL1 (without adding a newline)
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::device::serial::_print(format_args!($($arg)*));
    };
}


/// Print a string to SERIAL1 with a newline after it
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
