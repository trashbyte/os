///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use acpi_crate::{AcpiTables, AcpiHandler, PhysicalMapping};
use core::ptr::NonNull;
use alloc::sync::Arc;
use crate::PHYS_MEM_OFFSET;
use conquer_once::spin::OnceCell;
use aml::AmlContext;
use x86_64::instructions::port::Port;
use spin::Mutex;


pub static ACPI_TABLES: OnceCell<AcpiTables<OsAcpiHandler>> = OnceCell::uninit();
pub static AML_CONTEXT: OnceCell<Mutex<AmlContext>> = OnceCell::uninit();

#[derive(Debug)]
pub struct AmlHandler;
impl aml::Handler for AmlHandler {
    fn read_u8(&self, address: usize) -> u8 {
        unsafe { *((address as u64 + PHYS_MEM_OFFSET) as *const u8) }
    }

    fn read_u16(&self, address: usize) -> u16 {
        unsafe { *((address as u64 + PHYS_MEM_OFFSET) as *const u16) }
    }

    fn read_u32(&self, address: usize) -> u32 {
        unsafe { *((address as u64 + PHYS_MEM_OFFSET) as *const u32) }
    }

    fn read_u64(&self, address: usize) -> u64 {
        unsafe { *((address as u64 + PHYS_MEM_OFFSET) as *const u64) }
    }

    fn write_u8(&mut self, address: usize, value: u8) {
        unsafe { *((address as u64 + PHYS_MEM_OFFSET) as *mut u8) = value; }
    }

    fn write_u16(&mut self, address: usize, value: u16) {
        unsafe { *((address as u64 + PHYS_MEM_OFFSET) as *mut u16) = value; }
    }

    fn write_u32(&mut self, address: usize, value: u32) {
        unsafe { *((address as u64 + PHYS_MEM_OFFSET) as *mut u32) = value; }
    }

    fn write_u64(&mut self, address: usize, value: u64) {
        unsafe { *((address as u64 + PHYS_MEM_OFFSET) as *mut u64) = value; }
    }

    fn read_io_u8(&self, port: u16) -> u8 {
        unsafe { Port::<u8>::new(port).read() }
    }

    fn read_io_u16(&self, port: u16) -> u16 {
        unsafe { Port::<u16>::new(port).read() }
    }

    fn read_io_u32(&self, port: u16) -> u32 {
        unsafe { Port::<u32>::new(port).read() }
    }

    fn write_io_u8(&self, port: u16, value: u8) {
        unsafe { Port::<u8>::new(port).write(value) }
    }

    fn write_io_u16(&self, port: u16, value: u16) {
        unsafe { Port::<u16>::new(port).write(value) }
    }

    fn write_io_u32(&self, port: u16, value: u32) {
        unsafe { Port::<u32>::new(port).write(value) }
    }

    // TODO: expose better pci config read/write functions in tinypci

    fn read_pci_u8(&self, _segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u8 {
        let bus = bus as u32;
        let device = device as u32;
        let func = function as u32;
        let offset = offset as u32;
        let address = (bus << 16) | (device << 11) | (func << 8) | (offset & 0xfc) | 0x80000000;

        unsafe {
            Port::<u32>::new(0xCF8).write(address);
            Port::<u32>::new(0xCFC).read() as u8
        }
    }

    fn read_pci_u16(&self, _segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u16 {
        let bus = bus as u32;
        let device = device as u32;
        let func = function as u32;
        let offset = offset as u32;
        let address = (bus << 16) | (device << 11) | (func << 8) | (offset & 0xfc) | 0x80000000;

        unsafe {
            Port::<u32>::new(0xCF8).write(address);
            Port::<u32>::new(0xCFC).read() as u16
        }
    }

    fn read_pci_u32(&self, _segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u32 {
        let bus = bus as u32;
        let device = device as u32;
        let func = function as u32;
        let offset = offset as u32;
        let address = (bus << 16) | (device << 11) | (func << 8) | (offset & 0xfc) | 0x80000000;

        unsafe {
            Port::<u32>::new(0xCF8).write(address);
            Port::<u32>::new(0xCFC).read()
        }
    }

    fn write_pci_u8(&self, _segment: u16, bus: u8, device: u8, function: u8, offset: u16, value: u8) {
        let bus = bus as u32;
        let device = device as u32;
        let func = function as u32;
        let offset = offset as u32;
        let address = (bus << 16) | (device << 11) | (func << 8) | (offset & 0xfc) | 0x80000000;

        unsafe {
            Port::<u32>::new(0xCF8).write(address);
            Port::<u32>::new(0xCFC).write(value as u32);
        }
    }

    fn write_pci_u16(&self, _segment: u16, bus: u8, device: u8, function: u8, offset: u16, value: u16) {
        let bus = bus as u32;
        let device = device as u32;
        let func = function as u32;
        let offset = offset as u32;
        let address = (bus << 16) | (device << 11) | (func << 8) | (offset & 0xfc) | 0x80000000;

        unsafe {
            Port::<u32>::new(0xCF8).write(address);
            Port::<u32>::new(0xCFC).write(value as u32);
        }
    }

    fn write_pci_u32(&self, _segment: u16, bus: u8, device: u8, function: u8, offset: u16, value: u32) {
        let bus = bus as u32;
        let device = device as u32;
        let func = function as u32;
        let offset = offset as u32;
        let address = (bus << 16) | (device << 11) | (func << 8) | (offset & 0xfc) | 0x80000000;

        unsafe {
            Port::<u32>::new(0xCF8).write(address);
            Port::<u32>::new(0xCFC).write(value);
        }
    }
}

#[derive(Debug)]
pub struct OsAcpiHandlerInner {
    offset: u64,
}

#[derive(Clone, Debug)]
pub struct OsAcpiHandler(Arc<OsAcpiHandlerInner>);

impl OsAcpiHandler {
    pub fn new(offset: u64) -> Self {
        Self(Arc::new(OsAcpiHandlerInner { offset }))
    }
}

impl AcpiHandler for OsAcpiHandler {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        // all physical memory is already mapped
        unsafe {
            PhysicalMapping::new(
                physical_address,
                NonNull::new((physical_address + self.0.offset as usize) as *mut T).unwrap(),
                size,
                size,
                self.clone())
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
        // all physical memory is already mapped
    }
}

pub fn init() {
    let rdsp_phys_addr = {
        let mut step = crate::StartupStep::begin("Finding RDSP");
        const RDSP_HEADER: u64 = 0x2052545020445352;
        let mut rdsp_addr = None;
        for i in 0..0x2000-1 {
            unsafe {
                let addr = 0x000E0000 + (i * 16) + PHYS_MEM_OFFSET;
                let section: u64 = *(addr as *mut u64);
                if section == RDSP_HEADER {
                    rdsp_addr = Some(addr);
                }
            }
        }
        if rdsp_addr.is_none() {
            panic!("Couldn't find RDSP");
        }
        step.ok();
        rdsp_addr.unwrap() - PHYS_MEM_OFFSET
    };

    {
        let mut step = crate::StartupStep::begin("Reading ACPI tables");
        let acpi_handler = OsAcpiHandler::new(PHYS_MEM_OFFSET);
        let acpi = unsafe { AcpiTables::from_rsdp(acpi_handler, rdsp_phys_addr as usize).unwrap() };

        //let mut aml_context = AmlContext::new();
        //let dsdt = acpi.dsdt.unwrap();
        //let slice = unsafe { alloc::slice::from_raw_parts((dsdt.address as u64 + PHYS_MEM_OFFSET) as *const u8, dsdt.length as usize) };
        //unsafe { debug_dump_memory(VirtAddr::new(dsdt.address as u64 + PHYS_MEM_OFFSET - 36), dsdt.length + 64); }
        //aml_context.parse_table(slice).unwrap();

        ACPI_TABLES.try_init_once(|| acpi)
            .expect("acpi::init should only be called once");
        step.ok();
    }
}
