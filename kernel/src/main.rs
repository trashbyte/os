///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![no_std]
#![no_main]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

#![feature(custom_test_frameworks)]

//#[macro_use]
//extern crate pest_derive;

extern crate alloc;

use core::panic::PanicInfo;
use os::{MemoryInitResults, println};
use bootloader::{BootInfo, entry_point};
use bootloader::bootinfo::{MemoryRegionType, MemoryRegion, FrameRange};
use x86_64::{VirtAddr};
use os::driver::ahci::constants::AHCI_MEMORY_SIZE;
use chrono::{Utc, TimeZone, LocalResult};
//use pest::Parser;


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os::serial_println!("{}", info);
    os::println!("{}", info);
    os::util::halt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os::test_panic_handler(info)
}


entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    let mut mmap_lock = os::memory::GLOBAL_MEMORY_MAP.lock();
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
//    for region in mmap_lock.iter() {
//        serial_println!("{:?}", region);
//    }
    drop(mmap_lock);
    if found_ahci_mem.is_none() {
        panic!("Failed to find free space for AHCI memory.");
    }
    let found_ahci_mem = found_ahci_mem.unwrap().range;
    for addr in found_ahci_mem.start_addr()..found_ahci_mem.end_addr() {
        unsafe { *((addr + boot_info.physical_memory_offset) as *mut u8) = 0 }
    }

    os::gdt_idt_init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let MemoryInitResults { mapper: _mapper, frame_allocator: _frame_allocator } = os::memory_init(phys_mem_offset);
    os::init_devices();
    os::apic_init();
    os::arch::rtc::init_rtc();

    let current_time_secs = os::arch::rtc::Rtc::new().time();
    let current_time = match Utc.timestamp_opt(current_time_secs as i64, 0) {
        LocalResult::None => {
            println!("ERROR: Failed to get current time - Invalid timestamp: {}", current_time_secs);
            Utc.timestamp(1577836800, 0) // fallback to 01/01/2020
        }
        LocalResult::Single(t) => t,
        LocalResult::Ambiguous(a, b) => {
            println!("WARNING: Current time is ambiguous: {} or {}", a, b);
            a
        }
    };
    println!("Current time is: {}", current_time);

   // let mut ahci_driver = unsafe {
   //     os::ahci_init(&pci_infos, found_ahci_mem.start_addr()..found_ahci_mem.end_addr())
   // };

//    let mut buf = [0u16; 4096];
//    unsafe {
//        let mut port = ahci_driver.ports[0].as_mut().unwrap();
//        os::driver::ahci::test_read(&mut port, 0, 8, (&mut buf) as *mut [u16] as *mut u16).unwrap();
//    }
//
//    for _ in 0..1000000 {}
//    unsafe {
//        let addr = ahci_driver.ports[0].as_mut().unwrap().cmd_list_addr.as_u64() + phys_mem_offset.as_u64();
//        debug_dump_memory(VirtAddr::new(addr), 0x20);
//    }

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

    (*os::shell::SHELL.lock()).submit();

    #[cfg(test)]
    test_main();

    os::util::halt_loop()
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
