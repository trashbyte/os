// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

#![no_std]

#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_in_array_repeat_expressions)]
#![feature(const_fn)]

#![cfg_attr(test, no_main)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
#[macro_use] extern crate num_derive;

pub mod acpi;
pub mod allocator;
pub mod device;
pub mod driver;
pub mod encoding;
pub mod fs;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod path;
pub mod pci;
pub mod serial;
pub mod service;
pub mod shell;
pub mod util;
pub mod vga_buffer;

use core::panic::PanicInfo;
use x86_64::{VirtAddr, PhysAddr};

#[cfg(test)]
use bootloader::entry_point;
use memory::BootInfoFrameAllocator;
use x86_64::structures::paging::OffsetPageTable;
use pci::{PciDeviceInfo, PciClass};
use alloc::vec::Vec;
use acpi::OsAcpiHandler;
use acpi_crate::parse_rsdp;
use acpi_crate::interrupt::InterruptModel;
use aml::AmlContext;
use x86_64::instructions::port::Port;
use core::ops::Range;
use crate::driver::ahci::AhciDriver;
use crate::util::halt_loop;
use crate::fs::vfs::VFS;
use crate::device::block::{BlockDevice, BlockDeviceMedia};
use crate::fs::partition::{MbrPartition, PartitionType, Partition};
use alloc::rc::Rc;
use crate::service::DiskService;
use crate::fs::ext2::Ext2Filesystem;

pub const PHYS_MEM_OFFSET: u64 = 0x100000000000;
pub const KERNEL_STACK_ADDR: u64 = 0xFFFF00000000;

// Test runner /////////////////////////////////////////////////////////////////

pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    exit_qemu(QemuExitCode::Success);
}

// Handlers ////////////////////////////////////////////////////////////////////

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    halt_loop()
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

// Testing entry point /////////////////////////////////////////////////////////

/// Entry point for `cargo xtest`
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(_boot_info: &'static BootInfo) -> ! {
    serial_println!("[failed]\n");
    gdt_idt_init();
    test_main();
    util::halt_loop()
}

// Initialization //////////////////////////////////////////////////////////////

/// Basic kernel initialization
///
/// NOTE: We do NOT have a valid heap yet, so nothing here can use `alloc` types.
pub fn gdt_idt_init() {
    gdt::init();
    interrupts::init_idt();
    let mut wait_port: Port<u8> = Port::new(0x80);
    let mut wait = || unsafe { wait_port.write(0); };
    let mut pic1_command: Port<u8> = Port::new(0x20);
    let mut pic1_data: Port<u8> = Port::new(0x21);
    let mut pic2_command: Port<u8> = Port::new(0xA0);
    let mut pic2_data: Port<u8> = Port::new(0xA1);

    unsafe {
        // init command (3 data bytes)
        pic1_command.write(0x11);
        wait();
        pic2_command.write(0x11);
        wait();

        // interrupt offsets
        pic1_data.write(0x20);
        wait();
        pic2_data.write(0x28);
        wait();

        // chaining
        pic1_data.write(4);
        wait();
        pic2_data.write(2);
        wait();

        // mode
        pic1_data.write(0x01);
        wait();
        pic2_data.write(0x01);
        wait();

        // after init command, mask all interrupts
//        pic1_data.write(0xFF);
//        wait();
//        pic2_data.write(0xFF);
    }

    // set PIT interval to ~200 Hz
    unsafe {
        // channel 0, low+high byte, mode 2, binary mode
        Port::<u8>::new(0x43).write(0b00110100);
        // set channel 0 interval to 5966 (0x174e)
        let mut port = Port::<u8>::new(0x40);
        port.write(0x4e);
        port.write(0x17);
    }

    //unsafe {
        // enable APIC
        //let mut port: Port<u8> = Port::new(0xF0);
        //let old = port.read();
        //port.write(old | 0x100);
    //}

    x86_64::instructions::interrupts::enable();
}

#[allow(dead_code)]
pub struct MemoryInitResults {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}

pub fn memory_init(phys_mem_offset: VirtAddr) -> MemoryInitResults {
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init() };
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    MemoryInitResults { mapper, frame_allocator }
}

pub fn init_devices() {
    crate::acpi_init();
    crate::pci::scan_devices();
    unsafe {
        crate::service::DISK_SERVICE = Some(spin::Mutex::new(DiskService::new()));
        (*crate::service::DISK_SERVICE.as_mut().unwrap().lock()).init();
    }
    let mut disk_srv = unsafe { crate::service::DISK_SERVICE.as_ref().unwrap().lock() };
    (*disk_srv).init();
    let part = MbrPartition {
        media: (*disk_srv).get(2).unwrap(),
        first_sector: 0,
        last_sector: 0,
        partition_type: PartitionType::Filesystem
    };
    let block_dev = Rc::new(BlockDevice::new(BlockDeviceMedia::Partition(Partition::MBR(part))));
    let fs = unsafe { Rc::new(Ext2Filesystem::read_from(block_dev.clone()).unwrap()) };
    unsafe { crate::fs::vfs::GLOBAL_VFS = Some(spin::Mutex::new(VFS::init(fs))); }
}

pub unsafe fn ahci_init(pci_infos: &Vec<PciDeviceInfo>, ahci_mem_range: Range<u64>) -> AhciDriver {
    let ahci_controller_info = pci_infos.iter()
        .filter(|x| { x.class() == PciClass::MassStorage })
        .next()
        .expect("No AHCI controller found.");

    let ahci_hba_addr = PhysAddr::new((ahci_controller_info.bars[5] & 0xFFFFFFF0) as u64);
    let mut driver = AhciDriver::new(ahci_hba_addr, ahci_mem_range);
    driver.reset();
    //driver.set_ahci_enable(true);
    driver.set_interrupt_enable(true);
    driver
}

pub fn acpi_init() {
    const RDSP_HEADER: u64 = 0x2052545020445352;
    let mut rdsp_addr = None;
    for i in 0..0x2000-1 {
        unsafe {
            let addr = 0x000E0000 + (i * 16) + PHYS_MEM_OFFSET;
            let section = *(addr as *mut u64) as u64;
            if section == RDSP_HEADER {
                rdsp_addr = Some(addr);
            }
        }
    }
    if rdsp_addr.is_none() {
        panic!("Couldn't find RDSP");
    }
    let rdsp_phys_addr = rdsp_addr.unwrap() - PHYS_MEM_OFFSET;

    let mut acpi_handler = OsAcpiHandler::new(PHYS_MEM_OFFSET);
    let acpi = parse_rsdp(&mut acpi_handler, rdsp_phys_addr as usize).unwrap();
    let apic_slot = acpi.interrupt_model.as_ref().unwrap();
    let _apic;
    if let InterruptModel::Apic(a) = apic_slot {
        _apic = a;
    }
    else {
        panic!("No APIC found. Current kernel requires APIC.");
    }
    let mut aml_context = AmlContext::new();
    for ssdt in acpi.ssdts.iter() {
        aml_context.parse_table(unsafe { alloc::slice::from_raw_parts((ssdt.address as u64 + PHYS_MEM_OFFSET) as *const u8, ssdt.length as usize) }).unwrap();
    }
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

