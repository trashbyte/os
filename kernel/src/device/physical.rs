///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use alloc::sync::Arc;
use spin::Mutex;

pub trait Disk {
    /// Returns the ID for this disk
    fn id(&self) -> usize;
    /// Returns the type of disk this is
    fn kind(&self) -> PhysicalDeviceType;
    /// Returns the size of the disk in bytes, or `None` if the size is unknown
    fn size(&self) -> Option<u64>;
    /// Read data from the disk into the given buffer starting from block number `block`
    fn read(&mut self, block: u64, buffer: &mut [u8]) -> Result<Option<usize>, anyhow::Error>;
    /// Write data to the disk from the given buffer starting at block number `block`
    fn write(&mut self, block: u64, buffer: &[u8]) -> Result<Option<usize>, anyhow::Error>;
    /// Return this disk's block length in bytes
    fn block_length(&mut self) -> Result<u32, anyhow::Error>;
}

#[derive(Clone)]
pub struct SyncDisk {
    disk: Arc<Mutex<Arc<dyn Disk>>>
}
impl SyncDisk {
    pub fn new(disk: Arc<dyn Disk>) -> Self {
        Self { disk: Arc::new(Mutex::new(disk)) }
    }
}
unsafe impl Send for SyncDisk {}
unsafe impl Sync for SyncDisk {}

#[derive(Debug)]
pub enum PhysicalDeviceType {
    FloppyDrive,
    IdeDrive,
    SataDrive,
    SatapiDrive,
    NVMeDrive,
    Unknown
}

#[derive(Clone, Copy, Debug)]
pub struct PhysicalDeviceId(pub u32);
impl PhysicalDeviceId {
    pub fn as_u32(&self) -> u32 { self.0 }
}

#[derive(Debug, Clone, Copy)]
pub struct PhysicalDevice {
    id: PhysicalDeviceId,
    dev_type: PhysicalDeviceType,
}

