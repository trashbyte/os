///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

// TODO: check for redundancy with DiskService

use alloc::sync::Arc;
use self::ata::AtaDisk;
use self::atapi::AtapiDisk;
use self::hba::HbaMemory;
use self::constants::HbaPortType;
use x86_64::PhysAddr;
use alloc::vec::Vec;
use crate::{AHCI_MEM_REGION, PCI_DEVICES, PHYS_MEM_OFFSET};
use spin::Mutex;
use tinypci::PciClass;
use crate::device::physical::SyncDisk;

/// Types related to the AHCI HBA (Host Bus Adapter)
pub mod hba;
/// Types related to the various types of FIS (Frame Information Structure)
pub mod fis;
/// Constants and enums for AHCI values
pub mod constants;
/// `Disk` implementation for (S)ATA disks
pub mod ata;
/// `Disk` implementation for (S)ATAPI disks
pub mod atapi;

static HBA: Mutex<Option<&'static mut HbaMemory>> = Mutex::new(None);

/// Initialize the HBA and scan for disks
pub fn init() {
    crate::both_println!("Initializing AHCI controller...");
    let ahci_mem_region = AHCI_MEM_REGION.lock()
        .expect("called ahci_init without AHCI_MEM_REGION initialized")
        .range;
    let ahci_mem_range = ahci_mem_region.start_addr()..ahci_mem_region.end_addr();
    for addr in ahci_mem_range {
        // zero out all AHCI memory
        unsafe { *((addr + PHYS_MEM_OFFSET) as *mut u8) = 0; }
    }
    crate::both_println!("AHCI memory initialized at {:x}..{:x}", ahci_mem_region.start_addr(), ahci_mem_region.end_addr());

    let pci_lock = PCI_DEVICES.lock();
    let ahci_controller_info = pci_lock.iter()
        .find(|x| { x.class() == PciClass::MassStorage })
        .expect("No AHCI controller found.");

    let hba_mem_base = PhysAddr::new((ahci_controller_info.bars[5] & 0xFFFFFFF0) as u64);

    let hba_mem = unsafe { &mut *((hba_mem_base.as_u64() + PHYS_MEM_OFFSET) as *mut HbaMemory) };
    hba_mem.init();
    *HBA.lock() = Some(hba_mem);
    crate::both_println!("HBA initialized.");
}

pub async fn scan_disks() -> Vec<SyncDisk> {
    crate::both_println!("Scanning for disks...");
    let disks: Vec<SyncDisk> =  {
        let mut lock = HBA.lock();
        let hba_mem = lock.as_mut().unwrap();
        let pi = hba_mem.ports_impl.read();
        (0..hba_mem.ports.len())
            .filter(|&i| pi & 1 << i as i32 == 1 << i as i32)
            .filter_map(|i| {
                let port = unsafe { &mut *hba_mem.ports.as_mut_ptr().add(i) };
                let port_type = port.probe();
                crate::serial_println!("disk-{}: {:?}", i, port_type);

                let disk: Option<SyncDisk> = match port_type {
                    HbaPortType::SATA => {
                        match AtaDisk::new(i, port) {
                            Ok(disk) => Some(SyncDisk::new(Arc::new(disk))),
                            Err(err) => {
                                crate::serial_println!("{}: {}", i, err);
                                None
                            }
                        }
                    }
                    HbaPortType::SATAPI => {
                        match AtapiDisk::new(i, port) {
                            Ok(disk) => Some(SyncDisk::new(Arc::new(disk))),
                            Err(err) => {
                                crate::serial_println!("{}: {}", i, err);
                                None
                            }
                        }
                    }
                    _ => None,
                };
                disk
            })
            .collect()
    };
    disks
}
