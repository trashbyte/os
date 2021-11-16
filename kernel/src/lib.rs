///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![no_std]

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
pub mod time;

use core::panic::PanicInfo;
use x86_64::{VirtAddr, PhysAddr};

use crate::memory::BootInfoFrameAllocator;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::instructions::port::Port;
use crate::driver::ahci::AhciDriver;
//use crate::fs::vfs::VFS;
//use crate::device::block::{BlockDevice, BlockDeviceMedia};
//use crate::fs::partition::{MbrPartition, PartitionType, Partition};
//use crate::service::DiskService;
//use crate::fs::ext2::Ext2Filesystem;
#[cfg(test)]
use bootloader::BootInfo;
use tinypci::{PciDeviceInfo, PciClass};
use alloc::vec::Vec;
use core::ops::Range;
use crate::service::DiskService;
use crate::fs::partition::{MbrPartition, PartitionType, Partition};
use alloc::sync::Arc;
use crate::device::block::{BlockDevice, BlockDeviceMedia};

pub const PHYS_MEM_OFFSET: u64 = 0x100000000000;
pub const KERNEL_STACK_ADDR: u64 = 0xFFFF00000000;

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

pub fn test_panic_handler(info: &PanicInfo) -> ! {
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

// Initialization //////////////////////////////////////////////////////////////

#[allow(dead_code)]
pub struct MemoryInitResults {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}

pub fn memory_init(phys_mem_offset: VirtAddr) -> MemoryInitResults {
    both_println!("Initializing heap");
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init() };
    memory::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    MemoryInitResults { mapper, frame_allocator }
}

pub fn init_devices() -> Vec<PciDeviceInfo> {
    both_println!("Scanning for PCI devices");
    let pci_infos = tinypci::brute_force_scan();
    for i in pci_infos.iter() {
        match i.full_class {
            tinypci::PciFullClass::MassStorage_IDE => {
                both_println!("Found IDE device: bus {} device {}", i.bus, i.device);
            },
            tinypci::PciFullClass::MassStorage_ATA => {
                both_println!("Found ATA device: bus {} device {}", i.bus, i.device);
            },
            tinypci::PciFullClass::MassStorage_SATA => {
                both_println!("Found SATA device: bus {} device {}", i.bus, i.device);
            },
            _ => {
                both_println!("Found unsupported PCI device: bus {} device {} class {:?}", i.bus, i.device, i.full_class);
            }
        }
    }

    let mut disk_srv = unsafe {
        crate::service::DISK_SERVICE = Some(spin::Mutex::new(DiskService::new()));
        crate::service::DISK_SERVICE.as_mut().unwrap().lock()
    };
    (*disk_srv).init();
    let part = MbrPartition {
        media: (*disk_srv).get(2).unwrap(),
        first_sector: 0,
        last_sector: 0,
        partition_type: PartitionType::Filesystem
    };
    let _block_dev = Arc::new(BlockDevice::new(BlockDeviceMedia::Partition(Partition::MBR(part))));
    // let fs = unsafe { Arc::new(Ext2Filesystem::read_from(&block_dev).unwrap()) };
    // unsafe { crate::fs::vfs::GLOBAL_VFS = Some(spin::Mutex::new(VFS::init(fs))); }

    pci_infos
}

pub unsafe fn ahci_init(pci_infos: &Vec<PciDeviceInfo>, ahci_mem_range: Range<u64>) -> AhciDriver {
    both_println!("Initializing AHCI controller");


    for addr in ahci_mem_range.clone() {
        // zero out all AHCI memory
        *((addr + PHYS_MEM_OFFSET) as *mut u8) = 0;
    }
    both_println!("AHCI memory initialized at {:x}..{:x}", ahci_mem_range.start, ahci_mem_range.end);

    let ahci_controller_info = pci_infos.iter()
        .filter(|x| { x.class() == PciClass::MassStorage })
        .next()
        .expect("No AHCI controller found.");

    let ahci_hba_addr = PhysAddr::new((ahci_controller_info.bars[5] & 0xFFFFFFF0) as u64);
    let mut driver = AhciDriver::new(ahci_hba_addr, ahci_mem_range);
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

