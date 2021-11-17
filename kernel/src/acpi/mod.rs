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


pub static ACPI_TABLES: OnceCell<AcpiTables<OsAcpiHandler>> = OnceCell::uninit();


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
