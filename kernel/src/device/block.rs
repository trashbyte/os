///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L


use core::ops::Range;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::fs::partition::Partition;

#[derive(Debug, Clone, Copy)]
pub enum BlockDeviceError {

}

#[derive(Debug)]
pub enum BlockDeviceMedia {
    Partition(Partition)
}

pub const BLOCK_SIZE: usize = 4096;
pub type Block = [u8; BLOCK_SIZE];

#[derive(Debug)]
pub struct BlockDevice {
    pub media: BlockDeviceMedia,
}
impl BlockDevice {
    pub fn new(media: BlockDeviceMedia) -> Self { Self { media } }

    pub fn open() -> Result<Box<Self>, BlockDeviceError> {
        unimplemented!();
    }
    pub fn close(&self) -> Result<(), BlockDeviceError> {
        unimplemented!();
    }
    pub fn read(&self, block_num: u64) -> Result<Block, BlockDeviceError> {
        match &self.media {
            BlockDeviceMedia::Partition(part) => {
                let mut buffer = [0u8; BLOCK_SIZE];
                part.read_bytes((block_num * BLOCK_SIZE as u64)..((block_num + 1) * BLOCK_SIZE as u64), &mut buffer);
                Ok(buffer)
            }
        }
    }
    pub fn read_range(&self, _block_range: Range<u64>) -> Result<Vec<Block>, BlockDeviceError> {
        unimplemented!();
    }
    pub fn write(&self, _block_num: u64, _block: Block) -> Result<(), BlockDeviceError> {
        unimplemented!();
    }
    pub fn write_range(&self, _block_range: Range<u64>, _block: Block) -> Result<(), BlockDeviceError> {
        unimplemented!();
    }
}
