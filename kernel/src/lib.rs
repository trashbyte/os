///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

//! The main crate for the \[untitled os] kernel.

#![no_std]

#![warn(absolute_paths_not_starting_with_crate,
        elided_lifetimes_in_paths,
        explicit_outlives_requirements,
        macro_use_extern_crate,
        meta_variable_misuse,
        missing_debug_implementations,
        noop_method_call,
        pointer_structural_match,
        rust_2021_incompatible_closure_captures,
        rust_2021_incompatible_or_patterns,
        rust_2021_prefixes_incompatible_syntax,
        rust_2021_prelude_collisions,
        single_use_lifetimes,
        trivial_numeric_casts,
        unreachable_pub,
        unused_crate_dependencies,
        unused_extern_crates,
        unused_import_braces,
        unused_lifetimes,
        unused_qualifications,
        variant_size_differences)]
//#![warn(missing_docs)] // make sure everything is documented
#![warn(unsafe_op_in_unsafe_fn)] // make unsafety as explicit as possible
#![forbid(non_ascii_idents)] // prevent unicode homoglyph attacks

#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(asm)]
#![feature(const_mut_refs)]
#![feature(const_fn_trait_bound)]

#![cfg_attr(test, no_main)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

pub use chrono;

/// ACPI support
pub mod acpi;
/// Architecture-specific implementations
pub mod arch;
/// Devices that support reading and/or writing, physical or virtual
pub mod device;
/// Drivers for specific hardware
pub mod driver;
/// Functions for decoding byte strings of various encodings to unicode and encoding them back
pub mod encoding;
/// Filesystem types and implementations
pub mod fs;
/// Memory management and allocators
pub mod memory;
/// Utilities for working with filesystem paths
pub mod path;
/// Global services for managing resources
pub mod service;
/// Interactive shell system and parser
pub mod shell;
/// General utilities
pub mod util;
/// Text-mode VGA output
pub mod vga_buffer;
/// Async executor and basic runnable task
pub mod task;
/// Types and functions for dealing with times and dates
pub mod time;

use core::panic::PanicInfo;
use x86_64::{VirtAddr, PhysAddr};

use crate::memory::{BootInfoFrameAllocator, AHCI_MEM_REGION};
use x86_64::structures::paging::OffsetPageTable;
use x86_64::instructions::port::Port;
use tinypci::{PciDeviceInfo, PciClass};
use alloc::vec::Vec;
use crate::vga_buffer::Color;
use bootloader::bootinfo::{MemoryRegionType, MemoryRegion, FrameRange};
use crate::driver::ahci;
use crate::driver::ahci::constants::AHCI_MEMORY_SIZE;
use alloc::boxed::Box;

/// Start address where physical memory is identity mapped in virtual memory
pub const PHYS_MEM_OFFSET: u64 = 0x100000000000;

// Testing stuff ///////////////////////////////////////////////////////////////////////////////////

/// Auto trait for test cases. Wraps them in print calls to output `module::function... [ok]` and whatnot.
pub trait Testable {
    /// Run the test case
    fn run(&self);
}

impl<T> Testable for T
    where
        T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

/// Entry point for test runner. Passed a list of [Testable]s to run.
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

/// Panic handler for tests.
/// Prints `[failed]` along with the error and exits QEMU with an error code.
pub fn test_panic_handler(info: &PanicInfo<'_>) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    unreachable!()
}

/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

/// Scope-based utility for printing kernel startup messages.
/// Draws the message out first, with `[      ]` at the beginning,
/// then adds `ok` or `failed` when dropped depending on the result.
#[derive(Debug)]
pub struct StartupStep {
    /// True if the step succeeded
    ok: bool
}

impl StartupStep {
    /// Begins a new startup step with the given message.
    /// Keep a reference to the returned object, it prints
    /// the result when dropped.
    #[must_use]
    pub fn begin(msg: & str) -> Self {
        serial_print!("{}... ", msg);
        print!("[      ] {}", msg);

        Self { ok: false }
    }

    /// Set the step to have succeeded
    pub fn ok(&mut self) { self.ok = true; }
    /// Set the step to have failed.
    /// This is the default for new steps, so it's usually unnecessary,
    /// but you may want to "un-ok" a step after calling `ok()`.
    pub fn fail(&mut self) { self.ok = false; }

    /// Sets the value of `ok` based on whether the given result was `Ok` or `Err`.
    pub fn result(&mut self, res: &Result<(), anyhow::Error>) {
        self.ok = res.is_ok();
        // do something with error msg?
    }
}

impl Drop for StartupStep {
    fn drop(&mut self) {
        x86_64::instructions::interrupts::without_interrupts(|| {
            if self.ok {
                serial_println!("ok");
                let mut term = vga_buffer::TERMINAL.lock();
                term.set_color(Color::Green, Color::Black);
                term.set_cursor_x(3);
                term.write_string("ok\n");
                term.set_color(Color::White, Color::Black);
            }
            else {
                serial_println!("failed");
                let mut term = vga_buffer::TERMINAL.lock();
                term.set_color(Color::Red, Color::Black);
                term.set_cursor_x(1);
                term.write_string("failed\n");
                term.set_color(Color::White, Color::Black);
            }
        });
    }
}

// Initialization //////////////////////////////////////////////////////////////

#[allow(dead_code)]
#[derive(Debug)]
pub struct MemoryInitResults {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}

pub fn memory_init(phys_mem_offset: VirtAddr) -> MemoryInitResults {
    let mut step = StartupStep::begin("Initializing heap");
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init() };
    memory::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    *memory::HAVE_ALLOC.lock() = true;

    step.ok();
    MemoryInitResults { mapper, frame_allocator }
}

pub fn init_pci() -> Vec<PciDeviceInfo> {
    let pci_infos = {
        let mut step = StartupStep::begin("Scanning for PCI devices");
        let pci_infos = tinypci::brute_force_scan();
        if !pci_infos.is_empty() { step.ok(); }
        pci_infos
    };
    if pci_infos.is_empty() {
        both_println!("  Failed to find any PCI devices.");
    }
    for i in pci_infos.iter() {
        match i.full_class {
            tinypci::PciFullClass::MassStorage_IDE => {
                both_println!("  Found IDE device: bus {} device {}", i.bus, i.device);
            },
            tinypci::PciFullClass::MassStorage_ATA => {
                both_println!("  Found ATA device: bus {} device {}", i.bus, i.device);
            },
            tinypci::PciFullClass::MassStorage_SATA => {
                both_println!("  Found SATA device: bus {} device {}", i.bus, i.device);
            },
            _ => {
                both_println!("  Found unsupported PCI device: bus {} device {} class {:?}", i.bus, i.device, i.full_class);
            }
        }
    }
    pci_infos
}

pub fn build_memory_map(boot_info: &'static bootloader::BootInfo) {
    // search memory map provided by bootloader for a free memory region for AHCI
    let mut found_ahci_mem = None;
    {
        let mut step = StartupStep::begin("Building global memory map");
        let mut mmap_lock = memory::GLOBAL_MEMORY_MAP.lock();
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
                step.ok();
            } else {
                mmap_lock.add_region(*region);
            }
        }
        for region in mmap_lock.iter() {
            crate::serial_println!("{:?}", region);
        }
    }
    let found_ahci_mem = found_ahci_mem
        .expect("Failed to find free space for AHCI memory.");
    *AHCI_MEM_REGION.lock() = Some(found_ahci_mem);
}

pub fn init_services() {
    let mut step = StartupStep::begin("Initializing disk service");
    // let mut disk_srv = DiskService::new();
    // disk_srv.init();
    // let part = MbrPartition {
    //     media: disk_srv.get(2).unwrap(),
    //     first_sector: 0,
    //     last_sector: 0,
    //     partition_type: PartitionType::Filesystem
    // };
    // *crate::service::DISK_SERVICE.lock() = Some(disk_srv);
    // let _block_dev = Arc::new(BlockDevice::new(BlockDeviceMedia::Partition(Partition::MBR(part))));
    //let fs = unsafe { Arc::new(Ext2Filesystem::read_from(&block_dev).unwrap()) };
    //unsafe { *crate::fs::vfs::GLOBAL_VFS.lock() = Some(VFS::init(fs)); }
    step.ok();
}

pub unsafe fn ahci_init(pci_infos: &[PciDeviceInfo]) {
    crate::both_println!("Initializing AHCI controller...");
    let ahci_mem_region = AHCI_MEM_REGION.lock()
        .expect("called ahci_init without AHCI_MEM_REGION initialized")
        .range;
    let ahci_mem_range = ahci_mem_region.start_addr()..ahci_mem_region.end_addr();
    for addr in ahci_mem_range {
        // zero out all AHCI memory
        unsafe { *((addr + PHYS_MEM_OFFSET) as *mut u8) = 0; }
    }
    crate::both_println!("Zeroed AHCI host memory region");

    let ahci_controller_info = pci_infos.iter()
        .find(|x| { x.class() == PciClass::MassStorage })
        .expect("No AHCI controller found.");

    let ahci_hba_addr = PhysAddr::new((ahci_controller_info.bars[5] & 0xFFFFFFF0) as u64);
    let (_hba_mem, mut disks) = ahci::init(ahci_hba_addr);

    let mut buf = Box::new([69u8; 512]); // allocate on the heap
    match disks[0].read(0, buf.as_mut()) { _ => {} }
    for i in 0..32 {
        crate::serial_println!("{}  {}  {}  {}  {}  {}  {}  {}", buf[i*4],buf[i*4+1],buf[i*4+2],buf[i*4+3],buf[i*4+4],buf[i*4+5],buf[i*4+6],buf[i*4+7]);
    }
}

// QEMU ////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Zero = 0,
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

