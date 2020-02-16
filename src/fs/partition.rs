use alloc::vec::Vec;
use crate::driver::StorageDriver;
use alloc::boxed::Box;
use crate::util::UUID;
use alloc::string::String;

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
    pub media: Box<dyn StorageDriver>,
    pub first_sector: u32,
    pub last_sector: u32,
    pub partition_type: PartitionType,
}

pub struct GptPartition {
    pub media: Box<dyn StorageDriver>,
    pub first_sector: u32,
    pub last_sector: u32,
    pub partition_type: PartitionType,
    pub uuid: UUID,
    pub first_lba: u64,
    pub last_lba: u64,
    pub flags: u64,
    pub name: String,
}
