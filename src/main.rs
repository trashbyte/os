// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(slice_from_raw_parts)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;
use os::{println, MemoryInitResults};
use bootloader::{BootInfo, entry_point};
use bootloader::bootinfo::{MemoryRegionType, MemoryRegion, FrameRange};
use x86_64::{VirtAddr};
use os::driver::ahci::constants::AHCI_MEMORY_SIZE;


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    os::util::halt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os::test_panic_handler(info)
}



entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    let mut mmap_lock = os::memory::GLOBAL_MEMORY_MAP.lock();
    let mut found_ahci_mem = None;
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
        }
        else {
            mmap_lock.add_region(region.clone());
        }
    }
//    for region in mmap_lock.iter() {
//        serial_println!("{:?}", region);
//    }
    drop(mmap_lock);
    if found_ahci_mem.is_none() {
        panic!("Failed to find free space for AHCI memory.");
    }
    let found_ahci_mem = found_ahci_mem.unwrap().range;
    for addr in found_ahci_mem.start_addr()..found_ahci_mem.end_addr() {
        unsafe { *((addr + boot_info.physical_memory_offset) as *mut u8) = 0 }
    }

    os::gdt_idt_init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let MemoryInitResults { mapper: _mapper, frame_allocator: _frame_allocator } = os::memory_init(phys_mem_offset);
    os::init_devices();

//    let mut ahci_driver = unsafe {
//        os::ahci_init(&pci_infos, found_ahci_mem.start_addr()..found_ahci_mem.end_addr())
//    };

//    let mut buf = [0u16; 4096];
//    unsafe {
//        let mut port = ahci_driver.ports[0].as_mut().unwrap();
//        os::driver::ahci::test_read(&mut port, 0, 8, (&mut buf) as *mut [u16] as *mut u16).unwrap();
//    }
//
//    for _ in 0..1000000 {}
//    unsafe {
//        let addr = ahci_driver.ports[0].as_mut().unwrap().cmd_list_addr.as_u64() + phys_mem_offset.as_u64();
//        debug_dump_memory(VirtAddr::new(addr), 0x20);
//    }

    (*os::shell::SHELL.lock()).prompt();

    #[cfg(test)]
    test_main();

    os::util::halt_loop()
}
