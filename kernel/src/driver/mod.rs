///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

// TODO:
// lots more cleanup
// add helper/safety functions around memory-mapped structs
// shell list disks command

use alloc::vec::Vec;

pub mod ahci;

#[derive(Clone)]
enum Handle {
    List(Vec<u8>, usize), // Dir items, position
    Disk(usize, usize), // Disk index, position
    Partition(usize, u32, usize), // Disk index, partition index, position
}
