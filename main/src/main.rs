///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![no_std]
#![no_main]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

#![feature(custom_test_frameworks)]

extern crate alloc;

use core::panic::PanicInfo;
use bootloader::BootInfo;
use x86_64::{VirtAddr};
use kernel::{MemoryInitResults, both_println};
use kernel::time::DateTimeError;
use x86_64::instructions::port::Port;
use kernel::task::Task;
//use pest::Parser;


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    both_println!("\n{}", info);
    kernel::util::halt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

bootloader::entry_point!(kernel_main);
/// Main entry point for the kernel, called by the bootloader
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    kernel::init_memory_map(boot_info);
    kernel::arch::gdt::init();
    kernel::arch::interrupts::early_init_interrupts();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let MemoryInitResults { mapper: _mapper, frame_allocator: _frame_allocator } = kernel::init_memory(phys_mem_offset);

    kernel::acpi::init();
    kernel::arch::interrupts::late_init_interrupts();

    // set PIT interval to 100 Hz
    unsafe {
        // channel 0, low+high byte, mode 2, binary mode
        Port::<u8>::new(0x43).write(0b00110100);
        // set channel 0 interval to 11932 (0x2E9C)
        let mut port = Port::<u8>::new(0x40);
        port.write(0x9C);
        port.write(0x2E);
    }

    kernel::init_pci();
    kernel::driver::ahci::init();

    kernel::arch::rtc::init_rtc();

    match kernel::time::get_current_time() {
        Ok(time) => {
            both_println!("Current time is: {}", time);
        },
        Err(e) => {
            match e {
                DateTimeError::RtcInvalid(timestamp) => {
                    both_println!("ERROR: Failed to get current time - Invalid timestamp: {}", timestamp);
                }
                DateTimeError::AmbiguousTime(a, b) => {
                    both_println!("WARNING: Current time is ambiguous: {} or {}", a, b);
                }
            }
        }
    }

    #[cfg(test)]
    test_main();

    #[cfg(feature = "ci")]
    kernel::shutdown();

    let exec = kernel::task::executor::Executor::init();
    exec.run(kernel::task::Task::new(async_main())) // -> !
}

async fn async_main() {
    let executor = kernel::task::executor::GLOBAL_EXECUTOR.get().unwrap().clone();
    executor.spawn(Task::new(kernel::service::DiskService::init())).await;
    executor.spawn(Task::new(kernel::task::keyboard::process_scancodes())).await;

    both_println!("async_main exit");
    (*kernel::shell::SHELL.lock()).submit();
}
