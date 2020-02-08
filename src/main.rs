#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(slice_from_raw_parts)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;
use os::{println, serial_println, format_u32_as_bin_spaced};
use bootloader::{BootInfo, entry_point};
use os::acpi::OsAcpiHandler;
use acpi::parse_rsdp;
use aml::AmlContext;
use acpi::interrupt::InterruptModel::Apic;
use alloc::vec::Vec;
use os::pci::PciClass;
use os::driver::ahci::HbaMemory;


#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    os::halt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os::test_panic_handler(info)
}

entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use x86_64::VirtAddr;

    os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { os::memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        os::memory::BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    os::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    println!("Brute force scanning for PCI devices...");
    let mut pci_infos = Vec::new();
    os::pci::brute_force_scan(&mut pci_infos);
    let ahci_controller_info = pci_infos.iter()
        .filter(|x| { x.class() == PciClass::MassStorage })
        .next()
        .expect("No AHCI controller found.");
    println!("AHCI Controller PCI Info");
    println!("{}", ahci_controller_info);

    let ahci_hba_mem = unsafe { &mut *(((ahci_controller_info.bars[5] & 0xFFFFFFF0) as u64 + phys_mem_offset.as_u64()) as *mut HbaMemory) };
    println!("Ports implemented: {}", format_u32_as_bin_spaced(ahci_hba_mem.port_implemented));
    let port0 = &ahci_hba_mem.port_registers[0];
    println!("Port 0 device type: {:?}", port0.device_type());

    const RDSP_HEADER: u64 = 0x2052545020445352;
    let mut rdsp_addr = None;
    for i in 0..0x2000-1 {
        unsafe {
            let addr = 0x000E0000 + (i * 16) + phys_mem_offset.as_u64();
            let section = *(addr as *mut u64) as u64;
            if section == RDSP_HEADER {
                rdsp_addr = Some(addr);
            }
        }
    }
    if rdsp_addr.is_none() {
        panic!("Couldn't find RDSP");
    }
    let rdsp_phys_addr = rdsp_addr.unwrap() - phys_mem_offset.as_u64();

    let mut acpi_handler = OsAcpiHandler::new(phys_mem_offset.as_u64());
    let acpi = parse_rsdp(&mut acpi_handler, rdsp_phys_addr as usize).unwrap();
    let apic_slot = acpi.interrupt_model.as_ref().unwrap();
    let apic;
    if let Apic(a) = apic_slot {
        apic = a;
    }
    else {
        panic!("No APIC found. Current kernel requires APIC.");
    }
    let _apic_addr = apic.local_apic_address;
    let mut aml_context = AmlContext::new();
    for ssdt in acpi.ssdts.iter() {
        aml_context.parse_table(unsafe { alloc::slice::from_raw_parts((ssdt.address as u64 + phys_mem_offset.as_u64()) as *const u8, ssdt.length as usize) }).unwrap();
    }
    //serial_print!("{:?}", acpi);

    #[cfg(test)]
    test_main();

    os::halt_loop()
}
