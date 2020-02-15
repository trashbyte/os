#![allow(dead_code)]

use crate::driver::ata::AtaDrive;
use num_traits::float::Float;
use core::fmt::{Debug};
use alloc::string::String;
use alloc::{vec};
use crate::encoding::InvalidCharPolicy;
use alloc::vec::Vec;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use crate::path::Path;
use crate::util::UUID;

#[derive(Debug, Clone, Copy)]
pub enum Ext2FsState {
    Clean = 1,
    HasErrors = 2,
}

#[derive(Debug, Clone, Copy)]
pub enum Ext2ErrorHandling {
    /// Ignore the error and retry
    Ignore = 1,
    /// Remount the filesystem as read-only
    RemountReadOnly = 2,
    /// Kernel panic. Unrecoverable
    Panic = 3
}

#[derive(Debug, Clone, Copy)]
pub enum Ext2Error {
    /// Filesystem failed validity checks and is invalid or corrupt
    NotValidFs,
    /// Filesystem uses an unsupported version. Currently we only support major versions >= 1
    VersionNotSupported,
}

#[derive(Debug, Clone, Copy)]
pub enum Ext2OptionalFeature {
    PreallocBlocksForDirs = 0x0001,
    AFSServerInodes = 0x0002,
    Journaling = 0x0004,
    ExtendedInodeAttributes = 0x0008,
    ResizableFS = 0x0010,
    DirectoryHashIndex = 0x0020,
}

#[derive(Debug, Clone, Copy)]
pub enum Ext2RequiredFeature {
    Compression = 0x0001,
    DirectoryTypeField = 0x0002,
    JournalReplayNeeded = 0x0004,
    JournalDevice = 0x0008,
}

#[derive(Debug, Clone, Copy)]
pub enum Ext2ReadOnlyRequiredFeature {
    SparseDescriptors = 0x0001,
    U64FileSize = 0x0002,
    DirectoryBTreeFormat = 0x0004,
}

#[derive(Debug, Clone)]
pub struct Ext2JournalInfo {
    journal_id: [u8; 16],
    journal_inode: u32,
    journal_device: u32,
}

#[derive(Debug, Clone)]
pub struct Ext2Filesystem {
    pub filesystem_id: UUID,
    pub journal_id: UUID,
    pub volume_name: String,
    pub last_mounted_path: String,
    pub total_inodes: u32,
    pub total_blocks: u32,
    pub total_groups: u32,
    pub block_size: u32,
    pub fragment_size: u32,
    pub inode_size: u32,
    pub blocks_per_group: u32,
    pub fragments_per_group: u32,
    pub inodes_per_group: u32,
    pub last_mount_time: u32,
    pub last_written_time: u32,
    pub num_mounts_since_fsck: u16,
    pub num_mounts_allowed_until_fsck: u16,
    pub last_fsck_time: u32,
    pub forced_fsck_interval: u32,
    pub first_non_reserved_inode: u32,
    pub inode_struct_size: u16,
    pub superblock_backup_block: u16,
    pub blocks_to_prealloc_for_files: u8,
    pub blocks_to_prealloc_for_dirs: u8,
    pub optional_features: u32,
    pub required_features: u32,
    pub features_required_for_write: u32,
    pub head_of_orphan_inode_list: u32,
    pub journal_info: Option<Ext2JournalInfo>,
}
impl Ext2Filesystem {
    pub unsafe fn read_from(drive: &AtaDrive) -> Result<Self, Ext2Error> {
        let mut buffer = vec![0u8; 1024];
        drive.read_sector_to_slice(&mut buffer[0..512], 2);
        drive.read_sector_to_slice(&mut buffer[512..1024], 3);
        let header = (*(&buffer[0..0x54] as *const [u8] as *const SuperblockHeader)).clone();
        if header.check_signature != 0xEF53 {
            return Err(Ext2Error::NotValidFs);
        }
        let group_num_from_blocks = (header.total_blocks as f64 / header.blocks_per_group as f64).ceil() as u32;
        let group_num_from_inodes = (header.total_inodes as f64 / header.inodes_per_group as f64).ceil() as u32;

        if group_num_from_blocks != group_num_from_inodes {
            return Err(Ext2Error::NotValidFs);
        }
        let num_groups = group_num_from_blocks;

        if header.version_major < 1 {
            return Err(Ext2Error::VersionNotSupported);
        }
        let header_ext = (*(&buffer[0x54..0xEC] as *const [u8] as *const SuperblockHeaderExtended)).clone();

        let mut volume_name = String::new();
        for b in header_ext.volume_name.iter() {
            if *b == 0 { break; }
            volume_name.push(*b as char);
        }

        let mut last_mounted_path = String::new();
        for b in header_ext.last_mounted_path_1.iter() {
            if *b == 0 { break; }
            last_mounted_path.push(*b as char);
        }
        for b in header_ext.last_mounted_path_2.iter() {
            if *b == 0 { break; }
            last_mounted_path.push(*b as char);
        }

        Ok(Self {
            filesystem_id: UUID(header_ext.filesystem_id),
            journal_id: UUID(header_ext.journal_id),
            volume_name,
            last_mounted_path,
            total_inodes: header.total_inodes,
            total_blocks: header.total_blocks,
            total_groups: num_groups,
            block_size: 1024 << header.block_size,
            fragment_size: 1024 << header.fragment_size,
            inode_size: header_ext.inode_struct_size as u32,
            blocks_per_group: header.blocks_per_group,
            fragments_per_group: header.fragments_per_group,
            inodes_per_group: header.inodes_per_group,
            last_mount_time: header.last_mount_time,
            last_written_time: header.last_written_time,
            num_mounts_since_fsck: header.num_mounts_since_fsck,
            num_mounts_allowed_until_fsck: header.num_mounts_allowed_until_fsck,
            last_fsck_time: header.last_fsck_time,
            forced_fsck_interval: header.forced_fsck_interval,
            first_non_reserved_inode: header_ext.first_non_reserved_inode,
            inode_struct_size: header_ext.inode_struct_size,
            superblock_backup_block: header_ext.superblock_backup_block,
            blocks_to_prealloc_for_files: header_ext.blocks_to_prealloc_for_files,
            blocks_to_prealloc_for_dirs: header_ext.blocks_to_prealloc_for_dirs,
            optional_features: header_ext.optional_features,
            required_features: header_ext.required_features,
            features_required_for_write: header_ext.features_required_for_write,
            head_of_orphan_inode_list: header_ext.head_of_orphan_inode_list,
            journal_info: None
        })
    }

    unsafe fn read_bdt(&self, drive: &AtaDrive) -> BlockGroupDescriptor {
        let bdt_block = if self.block_size == 1024 { 2 } else { 1 };
        let buffer = drive.read_sector_to_vec(AtaDrive::sector_containing_addr(bdt_block * self.block_size as u64));
        let bdt = (*(buffer.as_slice() as *const [u8] as *const u8 as *const BlockGroupDescriptor)).clone();
        bdt
    }

    unsafe fn read_block(&self, block_num: u32, drive: &AtaDrive) -> Vec<u8> {
        if self.block_size != 4096 {
            panic!("Only block sizes of 4096 are currently supported");
            // TODO: add support for other block sizes once ATA convenience functions are done
        }
        let mut vec = Vec::with_capacity(4096);
        for i in 0..8 {
            let mut buffer = drive.read_sector_to_vec(AtaDrive::sector_containing_addr(block_num as u64 * self.block_size as u64 + (i * 512)));
            vec.append(&mut buffer);
        }
        vec
    }

    unsafe fn read_inode(&self, inode_num: u32, drive: &AtaDrive) -> Inode {
        let group = self.block_group_containing_inode(inode_num);
        let bdt = self.read_bdt(drive);
        let inode_index = self.inode_table_entry_index(inode_num);
        let inodes_per_block = self.block_size / self.inode_size;
        let group_block_offset = group * self.blocks_per_group * self.block_size;
        let blocks_offset_in_inode_table = inode_index / inodes_per_block;
        let buffer = self.read_block(bdt.inode_table_start_block + group_block_offset + blocks_offset_in_inode_table, drive);
        let inode_addr = ((buffer.as_slice() as *const [u8] as *const u8) as u64) + (inode_index as u64 * self.inode_size as u64);
        (*(inode_addr as *const Inode)).clone()
    }

    unsafe fn parse_directory_block(&self, block: &[u8]) -> Vec<DirectoryEntry> {
        let mut result = Vec::new();
        let buffer_addr = block as *const [u8] as *const u8 as u64;
        let mut offset = 0;
        loop {
            if offset >= block.len() { return result; }
            let dir_entry = (*((buffer_addr + offset as u64) as *const DirectoryEntryData)).clone();
            if dir_entry.inode == 0 { return result; }
            let file_name = crate::encoding::ISO_8859_1::decode_ptr((buffer_addr + offset as u64 + 8) as *const u8, dir_entry.name_length as usize, Some(InvalidCharPolicy::ReplaceWithUnknownSymbol)).unwrap();
            result.push(DirectoryEntry {
                file_name, type_indicator: DirectoryEntryType::from_u8(dir_entry.type_indicator).unwrap(), inode: dir_entry.inode
            });
            offset += dir_entry.total_entry_size as usize;
        }
    }

    pub unsafe fn list_directory(&self, _path: Path, drive: &AtaDrive) -> Vec<DirectoryEntry> {
        let block = self.read_block(404, drive);
        self.parse_directory_block(block.as_slice())
    }

    fn block_group_containing_block(&self, block_num: u32) -> u32 {
        block_num / self.blocks_per_group
    }
    fn block_group_containing_inode(&self, inode_num: u32) -> u32 {
        (inode_num - 1) / self.inodes_per_group
    }
    fn block_containing_inode(&self, inode_num: u32) -> u32 {
        (inode_num - 1) / self.inodes_per_group
    }
    fn inode_table_entry_index(&self, inode_num: u32) -> u32 {
        (inode_num - 1) % self.inodes_per_group
    }
}

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub file_name: String,
    pub type_indicator: DirectoryEntryType,
    pub inode: u32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct SuperblockHeader {
    total_inodes: u32,
    total_blocks: u32,
    superuser_reserved_blocks: u32,
    total_unallocated_blocks: u32,
    total_unallocated_inodes: u32,
    block_num_for_superblock: u32,
    block_size: u32,
    fragment_size: u32,
    blocks_per_group: u32,
    fragments_per_group: u32,
    inodes_per_group: u32,
    last_mount_time: u32,
    last_written_time: u32,
    num_mounts_since_fsck: u16,
    num_mounts_allowed_until_fsck: u16,
    check_signature: u16,
    file_system_state: u16,
    error_handling: u16,
    version_minor: u16,
    last_fsck_time: u32,
    forced_fsck_interval: u32,
    creator_os_id: u32,
    version_major: u32,
    user_id_for_reserved_blocks: u16,
    group_id_for_reserved_blocks: u16,
}


#[derive(Debug, Clone)]
#[repr(C)]
pub struct SuperblockHeaderExtended {
    /// First non-reserved inode in the filesystem (always 11 in versions < 1.0)
    first_non_reserved_inode: u32,
    /// Size of each inode struct in bytes (always 128 in versions < 1.0)
    inode_struct_size: u16,
    /// Block group that this superblock is part of (if backup copy)
    superblock_backup_block: u16,

    optional_features: u32,
    required_features: u32,
    features_required_for_write: u32,
    filesystem_id: [u8; 16],
    /// Volume name (C-style string: characters terminated by a 0 byte)
    volume_name: [u8; 16],
    /// Path volume was last mounted to (C-style string: characters terminated by a 0 byte)
    /// Split into two vars because types only have std trait impls up to size 32
    last_mounted_path_1: [u8; 32],
    last_mounted_path_2: [u8; 32],
    compression_algorithms_used: u32,
    blocks_to_prealloc_for_files: u8,
    blocks_to_prealloc_for_dirs: u8,
    _unused: u16,
    journal_id: [u8; 16],
    journal_inode: u32,
    journal_device: u32,
    head_of_orphan_inode_list: u32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct BlockGroupDescriptor {
    block_usage_bitmap_block: u32,
    inode_usage_bitmap_block: u32,
    inode_table_start_block: u32,
    unallocated_blocks: u16,
    unallocated_inodes: u16,
    num_directories: u16,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Inode {
    type_and_permissions: u16,
    user_id: u16,
    file_size_lower_half: u32,
    last_access_time: u32,
    creation_time: u32,
    modification_time: u32,
    deletion_time: u32,
    group_id: u16,
    hard_links_pointing_to_this_inode: u16,
    sectors_in_use: u32,
    flags: u32,
    os_specific_value_1: u32,
    direct_block_pointers: [u32; 12],
    singly_indirect_block_pointer: u32,
    doubly_indirect_block_pointer: u32,
    triply_indirect_block_pointer: u32,
    generation_number: u32,
    extended_attr_block: u32,
    file_size_upper_half: u32,
    block_address_of_fragment: u32,
    os_specific_value_2: [u8; 12]
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct DirectoryEntryData {
    inode: u32,
    total_entry_size: u16,
    name_length: u8,
    type_indicator: u8,
    file_name: u8 // variable-size c-str
}

#[derive(Debug, Clone, FromPrimitive)]
#[repr(u8)]
pub enum DirectoryEntryType {
    Unknown = 0,
    File = 1,
    Directory = 2,
    CharacterDevice = 3,
    BlockDevice = 4,
    FIFO = 5,
    Socket = 6,
    SymbolicLink = 7,
}
