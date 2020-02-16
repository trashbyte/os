// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license


pub struct FsHandle;

pub enum VfsError {
    FileNotFound,
    FileInUse,
    InvalidHandle
}

pub trait VFS {
    fn mount() -> Result<FsHandle, VfsError>;
    fn release(handle: FsHandle) -> Result<(), VfsError>;
}
