///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use kernel::{println, serial_print, serial_println};
use core::panic::PanicInfo;
use bootloader::BootInfo;

bootloader::entry_point!(main);
fn main(_boot_info: &'static BootInfo) -> ! {
    kernel::arch::gdt::init();
    kernel::arch::interrupts::early_init_interrupts();
    test_main();
    kernel::exit_qemu(kernel::QemuExitCode::Success);
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

#[test_case]
fn test_println() {
    println!("test_println output");
}
