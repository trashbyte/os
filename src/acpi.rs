use acpi_crate::{AcpiHandler, PhysicalMapping};
use core::ptr::NonNull;

pub const ACPI_START: usize = 0x_5555_5555_0000;

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
