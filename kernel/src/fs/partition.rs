// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

use alloc::vec::Vec;
use crate::driver::StorageDriver;
use crate::util::UUID;
use alloc::string::String;
use alloc::rc::Rc;
use core::ops::Range;

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

pub enum PartitionTable {
    MBR(MbrPartitionTable),
    GPT(GptPartitionTable),
}

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

pub struct MbrPartitionTable {
    pub partitions: Vec<MbrPartition>,
    pub disk_signature: u32,
    pub copy_protected: bool,
}

pub struct GptPartitionTable {
    pub partitions: Vec<GptPartition>,
    pub uuid: UUID,
    pub partition_entry_size: u32,
}

pub struct MbrPartition {
    pub media: Rc<dyn StorageDriver>,
    pub first_sector: u32,
    pub last_sector: u32,
    pub partition_type: PartitionType,
}

pub struct GptPartition {
    pub media: Rc<dyn StorageDriver>,
    pub first_sector: u32,
    pub last_sector: u32,
    pub partition_type: PartitionType,
    pub uuid: UUID,
    pub first_lba: u64,
    pub last_lba: u64,
    pub flags: u64,
    pub name: String,
}
