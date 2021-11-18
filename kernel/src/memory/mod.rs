///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

pub mod allocator;

use x86_64::{structures::paging::PageTable, VirtAddr, PhysAddr};
use x86_64::structures::paging::{OffsetPageTable, FrameAllocator, Size4KiB, PhysFrame};
use bootloader::bootinfo::{MemoryMap, MemoryRegionType, MemoryRegion};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref GLOBAL_MEMORY_MAP: Mutex<MemoryMap> = Mutex::new(MemoryMap::new());
}

pub static HAVE_ALLOC: Mutex<bool> = Mutex::new(false);
pub static AHCI_MEM_REGION: Mutex<Option<MemoryRegion>> = Mutex::new(None);


/// Initialize a new OffsetPageTable.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    let level_4_table = unsafe { &mut *page_table_ptr };
    unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
#[derive(Debug)]
pub struct BootInfoFrameAllocator {
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the global memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the global
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init() -> Self {
        BootInfoFrameAllocator { next: 0 }
    }
}

// TODO: deallocate frames
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = GLOBAL_MEMORY_MAP.lock().iter()
            .filter(|r| r.region_type == MemoryRegionType::Usable)
            .map(|r| r.range.start_addr()..r.range.end_addr())
            .flat_map(|r| r.step_by(4096))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
            .nth(self.next);
        self.next += 1;
        frame
    }
}
