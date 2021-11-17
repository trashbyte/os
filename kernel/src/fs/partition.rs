///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use alloc::vec::Vec;
use crate::driver::StorageDriver;
use crate::util::UUID;
use alloc::string::String;
use alloc::sync::Arc;
use core::ops::Range;
use core::fmt::{Debug, Formatter};

#[derive(Debug, Clone, Copy)]
pub enum PartitionType {
    FreeSpace,
    Swap,
    Filesystem,
    HiddenFilesystem,
    SecuredFilesystem,
    Container,
    HiddenContainer,
    SecuredContainer,
    Recovery,
    Hibernation,
    Blocker,
    Service,
    Unknown,
}

#[derive(Debug)]
pub enum PartitionTable {
    MBR(MbrPartitionTable),
    GPT(GptPartitionTable),
}

#[derive(Debug)]
pub enum Partition {
    MBR(MbrPartition),
    GPT(GptPartition),
}
impl Partition {
    pub fn read_bytes(&self, addr_range: Range<u64>, buffer: &mut [u8]) {
        // TODO: there has to be a better way to do this
        match &self {
            Partition::GPT(part) => part.media.read_bytes(addr_range, buffer),
            Partition::MBR(part) => part.media.read_bytes(addr_range, buffer)
        }
    }
}

#[derive(Debug)]
pub struct MbrPartitionTable {
    pub partitions: Vec<MbrPartition>,
    pub disk_signature: u32,
    pub copy_protected: bool,
}

#[derive(Debug)]
pub struct GptPartitionTable {
    pub partitions: Vec<GptPartition>,
    pub uuid: UUID,
    pub partition_entry_size: u32,
}

pub struct MbrPartition {
    pub media: Arc<dyn StorageDriver>,
    pub first_sector: u32,
    pub last_sector: u32,
    pub partition_type: PartitionType,
}
impl Debug for MbrPartition {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "MbrPartition {{ media: Arc<dyn StorageDriver>, first_sector: {}, last_sector: {}, partition_type: {:?} }}",
               self.first_sector, self.last_sector, self.partition_type)
    }
}

pub struct GptPartition {
    pub media: Arc<dyn StorageDriver>,
    pub first_sector: u32,
    pub last_sector: u32,
    pub partition_type: PartitionType,
    pub uuid: UUID,
    pub first_lba: u64,
    pub last_lba: u64,
    pub flags: u64,
    pub name: String,
}
impl Debug for GptPartition {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "GptPartition {{ media: Arc<dyn StorageDriver>, first_sector: {}, last_sector: {}, partition_type: {:?} }}",
               self.first_sector, self.last_sector, self.partition_type)
    }
}
