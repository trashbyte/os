///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

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

pub mod acpi;
pub mod arch;
pub mod device;
pub mod driver;
pub mod encoding;
pub mod fs;
pub mod memory;
pub mod path;
pub mod service;
pub mod shell;
pub mod util;
pub mod vga_buffer;
pub mod task;
pub mod time;

use core::panic::PanicInfo;
use x86_64::{VirtAddr, PhysAddr};

use crate::memory::BootInfoFrameAllocator;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::instructions::port::Port;
use crate::driver::ahci::AhciDriver;
#[cfg(test)]
use bootloader::BootInfo;
use tinypci::{PciDeviceInfo, PciClass};
use alloc::vec::Vec;
use core::ops::Range;
use crate::service::DiskService;
use crate::fs::partition::{MbrPartition, PartitionType, Partition};
use alloc::sync::Arc;
use crate::device::block::{BlockDevice, BlockDeviceMedia};
use crate::vga_buffer::Color;

/// Start address where physical memory is identity mapped in virtual memory
pub const PHYS_MEM_OFFSET: u64 = 0x100000000000;

// Testing stuff ///////////////////////////////////////////////////////////////////////////////////

pub trait Testable {
    fn run(&self) -> ();
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

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo<'_>) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
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

// Startup step printer

#[derive(Debug)]
pub struct StartupStep {
    ok: bool
}

impl StartupStep {
    pub fn begin(msg: & str) -> Self {
        serial_print!("{}... ", msg);
        print!("[      ] {}", msg);

        Self { ok: false }
    }

    pub fn ok(&mut self) { self.ok = true; }
    pub fn fail(&mut self) { self.ok = false; }

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
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init() };
    memory::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    *memory::HAVE_ALLOC.lock() = true;

    MemoryInitResults { mapper, frame_allocator }
}

pub fn init_pci() -> Vec<PciDeviceInfo> {
    let pci_infos = {
        let mut step = StartupStep::begin("Scanning for PCI devices");
        let pci_infos = tinypci::brute_force_scan();
        if pci_infos.len() != 0 { step.ok(); }
        pci_infos
    };
    if pci_infos.len() == 0 {
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

pub fn init_devices() {
    let mut disk_srv = DiskService::new();
    disk_srv.init();
    let part = MbrPartition {
        media: disk_srv.get(2).unwrap(),
        first_sector: 0,
        last_sector: 0,
        partition_type: PartitionType::Filesystem
    };
    *crate::service::DISK_SERVICE.lock() = Some(disk_srv);
    let _block_dev = Arc::new(BlockDevice::new(BlockDeviceMedia::Partition(Partition::MBR(part))));
    //let fs = unsafe { Arc::new(Ext2Filesystem::read_from(&block_dev).unwrap()) };
    //unsafe { *crate::fs::vfs::GLOBAL_VFS.lock() = Some(VFS::init(fs)); }
}

pub unsafe fn ahci_init(pci_infos: &Vec<PciDeviceInfo>, ahci_mem_range: Range<u64>) -> AhciDriver {
    for addr in ahci_mem_range.clone() {
        // zero out all AHCI memory
        unsafe { *((addr + PHYS_MEM_OFFSET) as *mut u8) = 0; }
    }

    let ahci_controller_info = pci_infos.iter()
        .filter(|x| { x.class() == PciClass::MassStorage })
        .next()
        .expect("No AHCI controller found.");

    let ahci_hba_addr = PhysAddr::new((ahci_controller_info.bars[5] & 0xFFFFFFF0) as u64);
    let mut driver = unsafe { AhciDriver::new(ahci_hba_addr, ahci_mem_range) };
    driver.reset();
    driver.set_ahci_enable(true);
    driver.set_interrupt_enable(true);
    driver
}

// QEMU ////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

