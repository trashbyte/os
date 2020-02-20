// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

#![allow(dead_code)]

use crate::path::Path;
use core::ops::Range;
use alloc::vec::Vec;

pub mod ahci;
pub mod ata;

#[derive(Debug, Clone, Copy)]
pub enum MountError {
    AlreadyMounted
}
#[derive(Debug, Clone, Copy)]
pub enum ReadError {}
#[derive(Debug, Clone, Copy)]
pub enum WriteError {
    InvalidParams
}

pub type Sector = [u8; 512];

/// A StorageDriver is a driver that sits directly on top of a storage device.
///
/// It is not aware of the structure of the underlying data, it simply serves as a
/// low-level interface to mount/unmount and read/write blocks to/from storage devices.
pub trait StorageDriver {
    fn mount(&mut self, path: Path) -> Result<(), MountError>;
    fn unmount(&mut self) -> Result<(), MountError>;
    fn read_sector(&self, sector_num: u32) -> Result<Sector, ReadError>;
    fn write_sector(&mut self, sector_num: u32, sector_data: &Sector) -> Result<(), WriteError>;
    fn read_sector_range(&self, sector_range: Range<u32>) -> Result<Vec<Sector>, ReadError> {
        let mut result = Vec::new();
        for sect in sector_range {
            result.push(self.read_sector(sect)?);
        }
        Ok(result)
    }
    fn write_sector_range(&mut self, sector_range: Range<u32>, sector_data: &Vec<Sector>) -> Result<(), WriteError> {
        if sector_data.len() as u32 != (sector_range.end - sector_range.start) {
            return Err(WriteError::InvalidParams);
        }
        for (i, sect) in sector_range.enumerate() {
            self.write_sector(sect, &sector_data[i])?;
        }
        Ok(())
    }
    fn read_bytes(&self, addr_range: Range<u64>, buffer: &mut [u8]);
    fn write_bytes(&self, addr_range: Range<u64>, data: &[u8]);
}
