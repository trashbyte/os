// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license


use core::ops::Range;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::fs::partition::Partition;

pub enum BlockDeviceError {

}

pub enum BlockDeviceMedia {
    Partition(Partition)
}

pub type Block = [u8; 4096];

pub struct BlockDevice {
    pub media: BlockDeviceMedia,
}
impl BlockDevice {
    fn open() -> Result<Box<Self>, BlockDeviceError> {
        unimplemented!();
    }
    fn close(&self) -> Result<(), BlockDeviceError> {
        unimplemented!();
    }
    fn read(&self, block_num: u64) -> Result<Block, BlockDeviceError> {
        unimplemented!();
    }
    fn read_range(&self, block_range: Range<u64>) -> Result<Vec<Block>, BlockDeviceError> {
        unimplemented!();
    }
    fn write(&self, block_num: u64, block: Block) -> Result<(), BlockDeviceError> {
        unimplemented!();
    }
    fn write_range(&self, block_range: Range<u64>, block: Block) -> Result<(), BlockDeviceError> {
        unimplemented!();
    }
}
