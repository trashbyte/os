// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

use acpi_crate::{AcpiTables, AcpiHandler, PhysicalMapping};
use acpi_crate::platform::interrupt::InterruptModel;
use core::ptr::NonNull;
use alloc::sync::Arc;
use crate::{PHYS_MEM_OFFSET};
use crate::arch::apic::APIC;


pub struct OsAcpiHandlerInner {
    offset: u64,
}

#[derive(Clone)]
pub struct OsAcpiHandler(Arc<OsAcpiHandlerInner>);

impl OsAcpiHandler {
    pub fn new(offset: u64) -> Self {
        Self(Arc::new(OsAcpiHandlerInner { offset }))
    }
}

impl AcpiHandler for OsAcpiHandler {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        // all physical memory is already mapped
        PhysicalMapping::new(
            physical_address,
            NonNull::new((physical_address + self.0.offset as usize) as *mut T).unwrap(),
            size,
            size,
            self.clone())
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
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

    let acpi_handler = OsAcpiHandler::new(PHYS_MEM_OFFSET);
    let acpi = unsafe { AcpiTables::from_rsdp(acpi_handler.clone(), rdsp_phys_addr as usize).unwrap() };
    let apic_slot = acpi.platform_info().unwrap().interrupt_model;
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
