///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use self::ata::AtaDisk;
use self::atapi::AtapiDisk;
use self::hba::HbaMemory;
use self::constants::HbaPortType;
use x86_64::PhysAddr;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::PHYS_MEM_OFFSET;

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

pub trait Disk {
    /// Returns the ID for this disk
    fn id(&self) -> usize;
    /// Returns the size of the disk in bytes, or `None` if the size is unknown
    fn size(&mut self) -> Option<u64>;
    /// Read data from the disk into the given buffer starting from block number `block`
    fn read(&mut self, block: u64, buffer: &mut [u8]) -> Result<Option<usize>, anyhow::Error>;
    /// Write data to the disk from the given buffer starting at block number `block`
    fn write(&mut self, block: u64, buffer: &[u8]) -> Result<Option<usize>, anyhow::Error>;
    /// Return this disk's block length in bytes
    fn block_length(&mut self) -> Result<u32, anyhow::Error>;
}

/// Initialize the HBA and scan for disks
pub fn init(hba_mem_base: PhysAddr) -> (&'static mut HbaMemory, Vec<Box<dyn Disk>>) {
    let hba_mem = unsafe { &mut *((hba_mem_base.as_u64() + PHYS_MEM_OFFSET) as *mut HbaMemory) };
    hba_mem.init();
    let pi = hba_mem.ports_impl.read();
    let disks: Vec<Box<dyn Disk>> = (0..hba_mem.ports.len())
          .filter(|&i| pi & 1 << i as i32 == 1 << i as i32)
          .filter_map(|i| {
              let port = unsafe { &mut *hba_mem.ports.as_mut_ptr().add(i) };
              let port_type = port.probe();
              crate::serial_println!("disk-{}: {:?}", i, port_type);

              let disk: Option<Box<dyn Disk>> = match port_type {
                  HbaPortType::SATA => {
                      match AtaDisk::new(i, port) {
                          Ok(disk) => Some(Box::new(disk)),
                          Err(err) => {
                              crate::serial_println!("{}: {}", i, err);
                              None
                          }
                      }
                  }
                  HbaPortType::SATAPI => {
                      match AtapiDisk::new(i, port) {
                          Ok(disk) => Some(Box::new(disk)),
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
          .collect();

    (hba_mem, disks)
}
