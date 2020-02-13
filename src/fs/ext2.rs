#![allow(dead_code)]

use crate::driver::ata::AtaDrive;
use crate::util::debug_dump_memory;
use crate::serial_println;
use x86_64::VirtAddr;
use num_traits::float::Float;
use core::fmt::{Debug, Formatter, Error};
use alloc::string::String;
use alloc::format;

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
    filesystem_id_str: String,
    journal_id_str: String,
    volume_name: String,
    last_mounted_path: String,
    total_inodes: u32,
    total_blocks: u32,
    block_size: u32,
    blocks_per_group: u32,
    inodes_per_group: u32,
    last_mount_time: u32,
    last_written_time: u32,
    num_mounts_since_fsck: u16,
    num_mounts_allowed_until_fsck: u16,
    last_fsck_time: u32,
    forced_fsck_interval: u32,
    first_non_reserved_inode: u32,
    inode_struct_size: u16,
    superblock_backup_block: u16,
    blocks_to_prealloc_for_files: u8,
    blocks_to_prealloc_for_dirs: u8,
    optional_features: u32,
    required_features: u32,
    features_required_for_write: u32,
    head_of_orphan_inode_list: u32,
    journal_info: Option<Ext2JournalInfo>,
}
impl Ext2Filesystem {
    pub unsafe fn read_from(drive: &AtaDrive) -> Result<Self, Ext2Error> {
        let mut buffer = [0u8; 1024];
        drive.read_sector(&mut *(buffer[0..512].as_mut_ptr() as *mut [u8; 512]), 2);
        drive.read_sector(&mut *(buffer[512..1024].as_mut_ptr() as *mut [u8; 512]), 3);
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

        let mut filesystem_id_str = String::new();
        for (i, b) in header_ext.filesystem_id.iter().enumerate() {
            filesystem_id_str.push_str(&format!("{:02X}", b));
            if i == 3 || i == 5 || i == 7 || i == 9 {
                filesystem_id_str.push('-');
            }
        }

        let mut journal_id_str = String::new();
        for (i, b) in header_ext.journal_id.iter().enumerate() {
            journal_id_str.push_str(&format!("{:02X}", b));
            if i == 3 || i == 5 || i == 7 || i == 9 {
                journal_id_str.push('-');
            }
        }

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
            filesystem_id_str, journal_id_str, volume_name, last_mounted_path,
            total_inodes: header.total_inodes,
            total_blocks: header.total_blocks,
            block_size: header.block_size,
            blocks_per_group: header.blocks_per_group,
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
