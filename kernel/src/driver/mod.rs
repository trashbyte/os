///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

// TODO:
// lots more cleanup
// add helper/safety functions around memory-mapped structs
// recreate disk service as async task
// allow tasks to spawn other tasks
// shell list disks command

use ahci::Disk;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::fs::partition::PartitionTable;

pub mod ahci;

#[derive(Clone)]
enum Handle {
    List(Vec<u8>, usize), // Dir items, position
    Disk(usize, usize), // Disk index, position
    Partition(usize, u32, usize), // Disk index, partition index, position
}

pub struct DiskWrapper {
    disk: Box<dyn Disk>,
    pt: Option<PartitionTable>,
}

impl DiskWrapper {
    fn new(disk: Box<dyn Disk>) -> Self {
        Self {
            pt: None,
            disk,
        }
    }
}

impl core::ops::Deref for DiskWrapper {
    type Target = dyn Disk;

    fn deref(&self) -> &Self::Target {
        &*self.disk
    }
}
impl core::ops::DerefMut for DiskWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.disk
    }
}
