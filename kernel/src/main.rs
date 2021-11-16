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

//#[macro_use]
//extern crate pest_derive;

extern crate alloc;

use core::panic::PanicInfo;
use bootloader::BootInfo;
use bootloader::bootinfo::{MemoryRegionType, MemoryRegion, FrameRange};
use x86_64::{VirtAddr};
use kernel::{MemoryInitResults, both_println};
use kernel::driver::ahci::constants::AHCI_MEMORY_SIZE;
use chrono::{Utc, TimeZone, LocalResult};
//use pest::Parser;


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    both_println!("{}", info);
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
    // search memory map provided by bootloader for a free memory region for AHCI
    both_println!("Building global memory map");
    let mut mmap_lock = kernel::memory::GLOBAL_MEMORY_MAP.lock();
    let mut found_ahci_mem = None;
    for region in boot_info.memory_map.iter() {
        if found_ahci_mem.is_none() && region.region_type == MemoryRegionType::Usable &&
            region.range.end_addr() - region.range.start_addr() >= AHCI_MEMORY_SIZE {

            let ahci_region = MemoryRegion {
                range: FrameRange::new(region.range.start_addr(), region.range.start_addr() + AHCI_MEMORY_SIZE),
                region_type: MemoryRegionType::InUse
            };
            let leftover = region.range.end_addr() - region.range.start_addr() - AHCI_MEMORY_SIZE;
            let leftover_region = MemoryRegion {
                range: FrameRange::new(region.range.start_addr() + leftover, region.range.end_addr()),
                region_type: MemoryRegionType::Usable
            };

            mmap_lock.add_region(ahci_region);
            mmap_lock.add_region(leftover_region);

            found_ahci_mem = Some(ahci_region);
        }
        else {
            mmap_lock.add_region(region.clone());
        }
    }
    // for region in mmap_lock.iter() {
    //     os::serial_println!("{:?}", region);
    // }
    drop(mmap_lock);
    if found_ahci_mem.is_none() {
        panic!("Failed to find free space for AHCI memory.");
    }
    let found_ahci_mem = found_ahci_mem.unwrap().range;

    kernel::arch::gdt::init();
    kernel::arch::interrupts::early_init_interrupts();

    // set PIT interval to ~200 Hz
    // unsafe {
    //     // channel 0, low+high byte, mode 2, binary mode
    //     Port::<u8>::new(0x43).write(0b00110100);
    //     // set channel 0 interval to 5966 (0x174e)
    //     let mut port = Port::<u8>::new(0x40);
    //     port.write(0x4e);
    //     port.write(0x17);
    // }

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let MemoryInitResults { mapper: _mapper, frame_allocator: _frame_allocator } = kernel::memory_init(phys_mem_offset);

    kernel::acpi::init();
    kernel::arch::interrupts::late_init_interrupts();

    let pci_infos = kernel::init_devices();

    kernel::arch::rtc::init_rtc();

    let current_time_secs = kernel::arch::rtc::Rtc::new().time();
    let current_time = match Utc.timestamp_opt(current_time_secs as i64, 0) {
        LocalResult::None => {
            both_println!("ERROR: Failed to get current time - Invalid timestamp: {}", current_time_secs);
            Utc.timestamp(1577836800, 0) // fallback to 01/01/2020
        }
        LocalResult::Single(t) => t,
        LocalResult::Ambiguous(a, b) => {
            both_println!("WARNING: Current time is ambiguous: {} or {}", a, b);
            a
        }
    };
    both_println!("Current time is: {}", current_time);

    let _ahci_driver = unsafe {
       kernel::ahci_init(&pci_infos, found_ahci_mem.start_addr()..found_ahci_mem.end_addr())
    };

    // let mut buf = Box::new([0u16; 4096]);
    // unsafe {
    //    let mut port = ahci_driver.ports[0].as_mut().unwrap();
    //    kernel::driver::ahci::test_read(&mut port, 0, 8, buf.as_mut_ptr()).unwrap();
    // }
    // for i in 0..4096 {
    //     serial_println!("{}", buf[i]);
    // }
   //
   //  // TODO: [HACK] there's gotta be a better way to do a wait here
   //  for _ in 0..1000000 {}
   //  unsafe {
   //     let addr = ahci_driver.ports[0].as_mut().unwrap().cmd_list_addr.as_u64() + phys_mem_offset.as_u64();
   //     debug_dump_memory(VirtAddr::new(addr), 0x20);
   // }

    // let pairs = IdentParser::parse(Rule::expr, r##" ls() + a_29 * 3 "##).unwrap_or_else(|e| panic!("{}", e));
    // for pair in pairs {
    //     // A pair is a combination of the rule which matched and a span of input
    //     println!(r#"{:<8} {:>3}{:<3} "{}""#,
    //              alloc::format!("{:?}", pair.as_rule()),
    //              alloc::format!("{:2}..", pair.as_span().start()),
    //              pair.as_span().end(),
    //              pair.as_str());
    //     // A pair can be converted to an iterator of the tokens which make it up:
    //     for inner_pair in pair.into_inner() {
    //         match inner_pair.as_rule() {
    //             Rule::alpha => println!("Letter:  {}", inner_pair.as_str()),
    //             Rule::digit => println!("Digit:   {}", inner_pair.as_str()),
    //             _ => unreachable!()
    //         };
    //     }
    // }

    both_println!("Boot complete!\n");

    (*kernel::shell::SHELL.lock()).submit();

    #[cfg(test)]
    test_main();

    kernel::util::halt_loop()
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
