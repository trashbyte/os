// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

use acpi_crate::{AcpiHandler, PhysicalMapping};
use acpi_crate::interrupt::InterruptModel;
use core::ptr::NonNull;
use crate::{PHYS_MEM_OFFSET};
use crate::arch::apic::APIC;


pub struct OsAcpiHandler {
    offset: u64,
}

impl OsAcpiHandler {
    pub fn new(offset: u64) -> Self {
        Self { offset }
    }
}

impl AcpiHandler for OsAcpiHandler {
    fn map_physical_region<T>(&mut self, physical_address: usize, size: usize) -> PhysicalMapping<T> {
        // all physical memory is already mapped
        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new((physical_address + self.offset as usize) as *mut T).unwrap(),
            region_length: size,
            mapped_length: size
        }
    }

    fn unmap_physical_region<T>(&mut self, _region: PhysicalMapping<T>) {
        // all physical memory is already mapped
    }
}

pub fn init() {
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
    //let acpi = parse_rsdp(&mut acpi_handler, rdsp_phys_addr as usize).unwrap();
    let acpi = acpi_crate::parse_rsdp(&mut acpi_handler, rdsp_phys_addr as usize).unwrap();
    let apic_slot = acpi.interrupt_model.as_ref().unwrap();
    if let InterruptModel::Apic(a) = apic_slot {
        *(crate::arch::apic::LOCAL_APIC.lock()) = APIC::new(a.local_apic_address);
    }
    else {
        panic!("No APIC found. Current kernel requires APIC.");
    }
    //let mut aml_context = AmlContext::new();
    //let dsdt = acpi.dsdt.unwrap();
    //let slice = unsafe { alloc::slice::from_raw_parts((dsdt.address as u64 + PHYS_MEM_OFFSET) as *const u8, dsdt.length as usize) };
    //unsafe { debug_dump_memory(VirtAddr::new(dsdt.address as u64 + PHYS_MEM_OFFSET - 36), dsdt.length + 64); }
    //aml_context.parse_table(slice).unwrap();
}
