///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![allow(dead_code)]

use alloc::string::String;
use core::fmt::{Display, Formatter, Error};
use alloc::vec::Vec;
use alloc::format;
use x86_64::VirtAddr;
use crate::serial_print;

/// Simple infinite loop using the x86 `hlt` instruction.
pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Format a 32-bit integer in binary, spaced for readability, like so:
/// 0000 0000 0000 0000 0000 0000 0000 0000
pub fn format_u32_as_bin_spaced(i: u32) -> String {
    let mut string = String::new();
    string += &format!("{:04b} ", (i >> 28) & 0xF);
    string += &format!("{:04b} ", (i >> 24) & 0xF);
    string += &format!("{:04b} ", (i >> 20) & 0xF);
    string += &format!("{:04b} ", (i >> 16) & 0xF);
    string += &format!("{:04b} ", (i >> 12) & 0xF);
    string += &format!("{:04b} ", (i >>  8) & 0xF);
    string += &format!("{:04b} ", (i >>  4) & 0xF);
    string += &format!("{:04b}",  (i      ) & 0xF);
    string
}

/// Trait for types that can be read from byte buffers.
pub trait BufferRead {
    fn read_from_buffer(buffer: &Vec<u8>) -> Self;
}

/// Trait for types that can be written to byte buffers.
pub trait BufferWrite {
    fn write_to_buffer(&self, buffer: &mut Vec<u8>);
}

/// Trait for types that can be read from and written to byte buffers.
pub trait BufferReadWrite: BufferRead + BufferWrite {}

/// Trait for types that can be read from raw virtual addresses.
pub trait MemoryRead {
    /// Read the contents of this struct from the specified virtual address.
    ///
    /// ## Unsafety
    ///
    /// Caller must ensure that the provided address points to a valid memory location
    /// that contains the correct data for this type.
    unsafe fn read_from_addr(addr: VirtAddr) -> Self;
}

/// Trait for types that can be written to raw virtual addresses.
pub trait MemoryWrite {
    /// Write the contents of this struct to the specified virtual address.
    ///
    /// ## Unsafety
    ///
    /// Caller must ensure that the provided address points to a valid memory
    /// location of the proper size, as this function will blindly overwrite
    /// its data to that address, possibly overwriting other memory or causing
    /// access violations if the destination region is too small.
    unsafe fn write_to_addr(&self, addr: VirtAddr);
}

/// Trait for types that can be read from and written to raw virtual addresses.
pub trait MemoryReadWrite: MemoryRead + MemoryWrite {}

pub unsafe fn debug_dump_memory(addr: VirtAddr, size: u32) {
    // arbitrary fixed number (need a const value for the raw ptr cast, and the actual
    // value doesn't matter since it only reads `size` bytes in, so even if addr+0x1000
    // is invalid memory, if you only read 0x20 bytes, it'll be fine.
    assert!(size <= 65536);
    let data = unsafe { &*((addr.as_u64()) as *const [u8; 65536]) };
    for i in 0..size {
        if i % 16 == 0 {
            serial_print!("\n{:#06X}   ", i);
        }
        serial_print!("{:02X} ", data[i as usize]);
    }
    serial_print!("\n");
}

pub unsafe fn read_c_str(addr: VirtAddr) -> String {
    let mut string = String::new();
    let mut i = 0;
    loop {
        let ptr = (addr.as_u64() + i) as *const char;
        if unsafe { *ptr } == '\0' { break; }
        string.push(unsafe { *ptr });
        i += 1;
    }
    string
}

pub unsafe fn read_c_str_with_len(addr: VirtAddr, len: usize) -> String {
    let mut string = String::new();
    let mut i = 0;
    loop {
        let ptr = unsafe { *((addr.as_u64() + i) as *const u8) as char };
        if ptr == '\0' || i == len as u64 { break; }
        string.push(ptr);
        i += 1;
    }
    string
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
#[repr(transparent)]
pub struct UUID(pub [u8; 16]);
impl UUID {
    pub fn parse(_s: String) -> Self {
        unimplemented!();
    }
    pub fn to_string(&self) -> String {
        let mut uuid_str = String::new();
        for (i, b) in self.0.iter().enumerate() {
            uuid_str.push_str(&format!("{:02X}", b));
            if i == 3 || i == 5 || i == 7 || i == 9 {
                uuid_str.push('-');
            }
        }
        uuid_str
    }
    pub fn version(&self) -> u32 {
        ((self.0[6] >> 4) & 0xF) as u32
    }
    pub fn variant(&self) -> u8 { (self.0[8] >> 5) & 0b111 }
}
impl Display for UUID {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.to_string())
    }
}

pub fn sleep(ms: u32) {
    let target_ticks = crate::arch::interrupts::ticks() + ms as u64;
    loop {
        for _ in 0..1000 {}
        if crate::arch::interrupts::ticks() > target_ticks { return; }
    }
}
