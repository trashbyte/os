///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![allow(dead_code)]

use num_traits::float::Float;
use core::fmt::Debug;
use alloc::string::String;
use crate::encoding::InvalidCharPolicy;
use alloc::vec::Vec;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use crate::path::Path;
use crate::util::UUID;
use crate::device::block::BlockDevice;
use crate::fs::{FsResult, FsError, Filesystem, VfsNodeType, VfsDirectoryEntry};
use core::iter::FromIterator;
use alloc::sync::Arc;

const ROOT_INODE: u64 = 2;

pub type FsHandle = u32;

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

// TODO: is there any point in using 64-bit inode/block addrs here?
#[derive(Debug)]
pub struct Ext2Filesystem {
    pub media: Arc<BlockDevice>,
    pub filesystem_id: UUID,
    pub journal_id: UUID,
    pub volume_name: String,
    pub last_mounted_path: String,
    pub total_inodes: u64,
    pub total_blocks: u64,
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
    pub unsafe fn read_from(media: &Arc<BlockDevice>) -> FsResult<Self> {
        let buffer = media.read(0)?;
        let header = unsafe { (*(&buffer[0x400..0x454] as *const [u8] as *const SuperblockHeader)).clone() };
        if header.check_signature != 0xEF53 {
            return Err(FsError::NotValidFs);
        }
        let group_num_from_blocks = (header.total_blocks as f64 / header.blocks_per_group as f64).ceil() as u32;
        let group_num_from_inodes = (header.total_inodes as f64 / header.inodes_per_group as f64).ceil() as u32;

        if group_num_from_blocks != group_num_from_inodes {
            return Err(FsError::NotValidFs);
        }
        let num_groups = group_num_from_blocks;

        if header.version_major < 1 {
            return Err(FsError::VersionNotSupported);
        }
        let header_ext = unsafe { (*(&buffer[0x454..0x4EC] as *const [u8] as *const SuperblockHeaderExtended)).clone() };

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
            media: media.clone(),
            filesystem_id: UUID(header_ext.filesystem_id),
            journal_id: UUID(header_ext.journal_id),
            volume_name,
            last_mounted_path,
            total_inodes: header.total_inodes as u64,
            total_blocks: header.total_blocks as u64,
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

    /// Reads the Block Group Descriptor for the given group number
    fn read_bgd(&self, group_num: u32) -> FsResult<BlockGroupDescriptor> {
        if group_num >= self.total_groups {
            return Err(FsError::OutOfBounds);
        }
        // figure out which block the BGD table is in
        let bgd_table_block = if self.block_size == 1024 { 2 } else { 1 };
        // offset into the table for the desired group's descriptor (32 bytes each)
        let bgd_byte_offset = group_num * 32;
        // read the block from media (table block + byte offset / block size (blocks rounded down))
        let block = self.media.read(bgd_table_block + (bgd_byte_offset as u64 / self.block_size as u64))?;
        unsafe {
            // raw address for the start of the read block in memory
            let buf_addr = &block as *const [u8] as *const u8 as u64;
            // address of the BGD
            let target_addr = buf_addr + (bgd_byte_offset % self.block_size) as u64;
            // read memory as a BGD
            let bdt = (*(target_addr as *const BlockGroupDescriptor)).clone();
            // TODO: validity check on BDT
            Ok(bdt)
        }
    }

    /// Reads an Ext2 block from the FS (NOT a block device block, although they're often 4k as well)
    fn read_block(&self, block_num: u64) -> FsResult<Vec<u8>> {
        if block_num >= self.total_blocks {
            return Err(FsError::OutOfBounds);
        }
        if self.block_size != 4096 {
            panic!("Only block sizes of 4096 are currently supported");
            // TODO: add support for other block sizes once ATA convenience functions are done
        }
        // assuming the 4k blocks line up is lazy but whatever, its temporary
        let block = self.media.read(block_num)?;
        Ok(Vec::from_iter(block.iter().cloned()))
    }

    fn read_inode(&self, inode_num: u64) -> FsResult<Inode>  {
        if self.block_size != 4096 {
            panic!("Only block sizes of 4096 are currently supported");
            // TODO: add support for other block sizes once ATA convenience functions are done
        }
        let group = self.block_group_containing_inode(inode_num)?;
        let bgd = self.read_bgd(group)?;
        let inode_index = self.inode_table_entry_index(inode_num)?;
        let inodes_per_block = self.block_size / self.inode_size;
        let group_block_offset = group * self.blocks_per_group * self.block_size;
        let block_offset_in_inode_table = inode_index as u32 / inodes_per_block;
        let block_num = bgd.inode_table_start_block + group_block_offset + block_offset_in_inode_table;
        let inode_index = inode_index % inodes_per_block as u64;
        // assuming the 4k blocks line up is lazy but whatever, its temporary
        let block = self.media.read(block_num as u64)?;
        assert!((inode_index * self.inode_size as u64) < self.block_size as u64);
        unsafe {
            let inode_addr = (&block as *const [u8] as *const u8 as u64) + (inode_index * self.inode_size as u64);
            Ok((*(inode_addr as *const Inode)).clone())
        }
    }

    fn parse_directory_block(&self, block: &[u8]) -> FsResult<Vec<Ext2DirectoryEntry>> {
        let mut result = Vec::new();
        let buffer_addr = block as *const [u8] as *const u8 as u64;
        let mut offset = 0;
        loop {
            if offset >= block.len() { return Ok(result); }
            // 10 bytes is the shortest possible valid directory entry (including \0 after name)
            assert!((offset + 10) < block.len());
            let dir_entry = unsafe {
                (*((buffer_addr + (offset as u64)) as *const DirectoryEntryData)).clone()
            };
            if dir_entry.inode == 0 { return Ok(result); }
            // TODO: validity check
            let name_start_ptr = (buffer_addr + offset as u64 + 8) as *const u8;
            // make sure we dont read an invalid string past the end of the buffer
            assert!((offset + 9 + dir_entry.name_length as usize) < block.len());
            let file_name = unsafe {
                crate::encoding::iso_8859_1::decode_ptr(name_start_ptr,
                                                        dir_entry.name_length as usize,
                                                        Some(InvalidCharPolicy::ReplaceWithUnknownSymbol)
                ).unwrap()
            };
            let entry_node = self.read_inode(dir_entry.inode as u64)?;
            result.push(Ext2DirectoryEntry {
                file_name,
                entry_type: DirectoryEntryType::from(InodeType::from_u16(entry_node.type_and_permissions).unwrap()),
                inode: dir_entry.inode
            });
            offset += dir_entry.total_entry_size as usize;
        }
    }

    fn block_group_containing_block(&self, block_num: u64) -> FsResult<u64> {
        if block_num >= self.total_blocks { Err(FsError::OutOfBounds) }
        else { Ok(block_num / self.blocks_per_group as u64) }
    }

    fn block_group_containing_inode(&self, inode_num: u64) -> FsResult<u32> {
        // yes, greater than. inodes are indexed from one
        if inode_num > self.total_inodes { Err(FsError::OutOfBounds) }
        else { Ok(((inode_num - 1) / self.inodes_per_group as u64) as u32) }
    }

    fn block_containing_inode(&self, inode_num: u64) -> FsResult<u64> {
        // yes, greater than. inodes are indexed from one
        if inode_num > self.total_inodes { Err(FsError::OutOfBounds) }
        else { Ok((inode_num - 1) / self.inodes_per_group as u64) }
    }

    fn inode_table_entry_index(&self, inode_num: u64) -> FsResult<u64> {
        // yes, greater than. inodes are indexed from one
        if inode_num > self.total_inodes { Err(FsError::OutOfBounds) }
        else { Ok((inode_num - 1) % self.inodes_per_group as u64) }
    }

    // TODO: support indirect pointers
    fn list_single_directory_internal(&self, node: &Inode) -> FsResult<Vec<Ext2DirectoryEntry>> {
        let mut result = Vec::new();
        for block_num in node.direct_block_pointers.iter() {
            if *block_num != 0 {
                // assuming the 4k blocks line up is lazy but whatever, its temporary
                let block = self.media.read(*block_num as u64)?;
                result.append(&mut self.parse_directory_block(&block)?);
            }
        }
        Ok(result)
    }
}
impl Filesystem for Ext2Filesystem {
    fn list_directory(&self, path: &Path) -> FsResult<Vec<VfsDirectoryEntry>> {
        if self.block_size != 4096 {
            panic!("Only block sizes of 4096 are currently supported");
            // TODO: add support for other block sizes once ATA convenience functions are done
        }

        let mut current_node = self.read_inode(ROOT_INODE).unwrap();
        // skip root
        for segment in path.iter().skip(1) {
            let entries = self.list_single_directory_internal(&current_node)?;
            for e in entries {
                if e.file_name.as_str() == segment {
                    // found matching entry
                    if e.entry_type != DirectoryEntryType::Directory {
                        // tried to ls a file
                        return Err(FsError::PathContainsFileAsDirectory);
                    }
                    // set current node to this one, check next segment
                    current_node = self.read_inode(e.inode as u64).unwrap();
                }
            }
        }
        // if we've made it here, we've traversed the whole path,
        // and `current_node` points to the last segment in the path
        let dir_contents = self.list_single_directory_internal(&current_node)?;
        // need to convert to generic vfs entries
        let mut result = Vec::new();
        for e in dir_contents {
            result.push(VfsDirectoryEntry {
                file_name: e.file_name.clone(),
                full_path: path.clone() / Path::from(e.file_name),
                entry_type: e.entry_type.into(),
                inode: e.inode
            });
        }
        Ok(result)
    }
}
// TODO: NOT ACTUALLY THREAD SAFE
unsafe impl Send for Ext2Filesystem {}
unsafe impl Sync for Ext2Filesystem {}

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

#[derive(Debug, Clone, FromPrimitive, PartialEq, Eq)]
#[repr(u8)]
pub enum InodeType {
    Unknown = 0,
    FIFO = 0x10,
    CharDevice = 0x20,
    Directory = 0x40,
    BlockDevice = 0x60,
    File = 0x80,
    SymbolicLink = 0xA0,
    Socket = 0xC0,
}
impl InodeType {
    /// Parse the inode type from the 16-bit "type and permissions" field
    pub fn from_u16(i: u16) -> Option<Self> {
        Self::from_u8(((i & 0xF000) >> 8) as u8)
    }
}

#[derive(Debug, Clone, FromPrimitive, PartialEq, Eq)]
#[repr(u8)]
pub enum DirectoryEntryType {
    Unknown = 0,
    File = 1,
    Directory = 2,
    CharDevice = 3,
    BlockDevice = 4,
    FIFO = 5,
    Socket = 6,
    SymbolicLink = 7,
}
impl From<InodeType> for DirectoryEntryType {
    fn from(t: InodeType) -> Self {
        match t {
            InodeType::Unknown => DirectoryEntryType::Unknown,
            InodeType::FIFO => DirectoryEntryType::FIFO,
            InodeType::CharDevice => DirectoryEntryType::CharDevice,
            InodeType::Directory => DirectoryEntryType::Directory,
            InodeType::BlockDevice => DirectoryEntryType::BlockDevice,
            InodeType::File => DirectoryEntryType::File,
            InodeType::SymbolicLink => DirectoryEntryType::SymbolicLink,
            InodeType::Socket => DirectoryEntryType::Socket,
        }
    }
}
impl Into<VfsNodeType> for DirectoryEntryType {
    fn into(self) -> VfsNodeType {
        match self {
            DirectoryEntryType::Unknown => VfsNodeType::Unknown,
            DirectoryEntryType::FIFO => VfsNodeType::FIFO,
            DirectoryEntryType::CharDevice => VfsNodeType::CharDevice,
            DirectoryEntryType::Directory => VfsNodeType::Directory,
            DirectoryEntryType::BlockDevice => VfsNodeType::BlockDevice,
            DirectoryEntryType::File => VfsNodeType::File,
            DirectoryEntryType::SymbolicLink => VfsNodeType::SymbolicLink,
            DirectoryEntryType::Socket => VfsNodeType::Socket,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ext2DirectoryEntry {
    pub file_name: String,
    pub entry_type: DirectoryEntryType,
    pub inode: u32,
}
