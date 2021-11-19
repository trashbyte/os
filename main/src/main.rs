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
use kernel::memory::AHCI_MEM_REGION;
use x86_64::instructions::port::Port;
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
    kernel::build_memory_map(boot_info);
    kernel::arch::gdt::init();
    kernel::arch::interrupts::early_init_interrupts();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let MemoryInitResults { mapper: _mapper, frame_allocator: _frame_allocator } = kernel::memory_init(phys_mem_offset);

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

    let pci_infos = kernel::init_pci();
    kernel::init_services();
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

    unsafe { kernel::ahci_init(&pci_infos) };
    let found_ahci_mem = AHCI_MEM_REGION.lock().unwrap().range;
    both_println!("AHCI memory initialized at {:x}..{:x}", found_ahci_mem.start_addr(), found_ahci_mem.end_addr());

   //
   //  // TODO: [HACK] there's gotta be a better way to do a wait here
   //  for _ in 0..1000000 {}
   //  unsafe {
   //     let addr = ahci_driver.ports[0].as_mut().unwrap().cmd_list_addr.as_u64() + phys_mem_offset.as_u64();
   //     debug_dump_memory(VirtAddr::new(addr), 0x20);
   // }

    kernel::parse_aml();

    both_println!("Boot complete!\n");

    #[cfg(test)]
    test_main();

    #[cfg(feature = "ci")]
    kernel::shutdown();

    let exec = kernel::task::executor::Executor::init();
    exec.run(kernel::task::Task::new(async_main())) // -> !
}

async fn async_main() {
    let executor = kernel::task::executor::GLOBAL_EXECUTOR.get().unwrap().clone();
    executor.spawn(kernel::task::Task::new(kernel::task::keyboard::process_scancodes())).await;

    both_println!("async_main exit");
    (*kernel::shell::SHELL.lock()).submit();
}

// #[derive(Parser)]
// #[grammar = r###"
// alpha = { 'a'..'z' | 'A'..'Z' }
// digit = { '0'..'9' }
// underscore = { "_" }
// WHITESPACE = _{ " " | "\t" | "\r" | "\n" }
// lparen = { "(" }
// rparen = { ")" }
// plus = { "+" }
// minus = { "-" }
// star = { "*" }
// slash = { "/" }
// equal = { "=" }
// semicolon = { ";" }
// period = { "." }
//
// lit_true = { "true" }
// lit_false = { "false" }
// lit_int = @{ digit+ }
// lit_float = @{ digit+ ~ period ~ digit+ }
// lit_bool = { lit_true | lit_false }
// literal = { lit_int | lit_float | lit_bool }
//
// op = { plus | minus | star | slash }
//
// ident = @{ (alpha | underscore) ~ (alpha | digit | underscore)* }
//
// func_params = _{ lparen ~ ident* ~ rparen }
// func_call = { ident ~ func_params }
//
// term = { ident | func_call | literal }
// expr_right = _{ op ~ term }
// expr = _{ term ~ op ~ term }
//
// //assign_statement = _{ "let" ~ ident ~ equal ~ expr ~ semicolon  }
//
// "###]
// struct IdentParser;
