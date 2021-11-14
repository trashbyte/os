///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use crate::device::block::BlockDeviceError;
use crate::path::Path;
use alloc::vec::Vec;
use alloc::string::String;
use crate::fs::ext2::FsHandle;

pub mod fat32;
pub mod ext2;
pub mod vfs;
pub mod partition;

pub type FsResult<T> = Result<T, FsError>;

#[derive(Debug, Clone, Copy)]
pub enum FsError {
    /// Filesystem failed validity checks and is invalid or corrupt
    NotValidFs,
    /// Filesystem uses an unsupported version. Currently we only support major versions >= 1
    VersionNotSupported,
    /// Forwarding error from the block device
    BlockDeviceError(BlockDeviceError),
    /// Parameters were out of bounds (could be block number, group number, sector number, etc)
    OutOfBounds,
    /// Path does not exist
    FileNotFound,
    /// Requested file is already in use
    FileInUse,
    /// Provided handle is invalid
    InvalidHandle,
    /// Path is already mounted
    AlreadyMounted,
    /// Path is not mounted
    PathNotMounted,
    /// Tried to ls a file (e.g. `ls /a/b/c` where `b` is a file
    PathContainsFileAsDirectory
}
impl From<BlockDeviceError> for FsError {
    fn from(e: BlockDeviceError) -> Self {
        FsError::BlockDeviceError(e)
    }
}

pub struct FilesystemStatic;
impl FilesystemStatic {
    pub fn mount() -> FsResult<FsHandle> { Ok(0) }
    pub fn release(_handle: FsHandle) -> FsResult<()> { Ok(()) }
}

/// Generic filesystem interface
pub trait Filesystem {
    fn list_directory(&self, path: &Path) -> FsResult<Vec<VfsDirectoryEntry>>;
}
#[derive(Debug, Clone)]
pub struct VfsDirectoryEntry {
    pub file_name: String,
    pub full_path: Path,
    pub entry_type: VfsNodeType,
    pub inode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsNodeType {
    Unknown,
    File,
    Directory,
    CharDevice,
    BlockDevice,
    FIFO,
    Socket,
    SymbolicLink,
}
