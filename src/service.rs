// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

use spin::Mutex;
use crate::driver::ata::{AtaDrive, ide_identify};
use hashbrown::HashMap;
use crate::fs::ext2::Ext2Filesystem;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref DISK_SERVICE: Mutex<DiskService> = Mutex::new(DiskService::new());
    //pub static ref FS_SERVICE: Mutex<FsService> = Mutex::new(FsService::new());
}

pub struct DiskService {
    drives: HashMap<u32, AtaDrive>,
    next_id: u32,
}
impl DiskService {
    pub fn new() -> Self {
        Self {
            drives: HashMap::new(),
            next_id: 1,
        }
    }
    pub fn init(&mut self) {
        for bus in 0..2 {
            for device in 0..2 {
                unsafe {
                    if let Some(info) = ide_identify(bus, device) {
                        self.drives.insert(self.next_id, AtaDrive::from_identify(info, bus, device));
                        self.next_id += 1;
                    }
                }
            }
        }
    }
    pub fn get(&self, id: u32) -> Option<&AtaDrive> {
        match self.drives.get(&id) {
            None => None,
            Some(dt) => Some(&dt)
        }
    }
    pub fn iter(&self) -> hashbrown::hash_map::Iter<u32, AtaDrive> {
        self.drives.iter()
    }
}

pub enum FsType {
    Ext2(Ext2Filesystem)
}
impl FsType {
    pub fn type_as_str(&self) -> &'static str {
        match self {
            FsType::Ext2(_) => "Ext2"
        }
    }
}
//pub struct FsService {
//    filesystems: HashMap<UUID, (u32, FsType)>,
//}
//impl FsService {
//    pub fn new() -> Self {
//        Self { filesystems: HashMap::new() }
//    }
//    pub fn init(&mut self) {
//        unsafe {
//            for (disk_id, disk) in DISK_SERVICE.lock().iter() {
//                match disk {
//                    DiskType::ATA(ref drive) => {
//                        match Ext2Filesystem::read_from(drive) {
//                            Err(_) => {},
//                            Ok(fs) => {
//                                let id = fs.filesystem_id;
//                                self.filesystems.insert(id, (*disk_id, FsType::Ext2(fs)));
//                            }
//                        }
//                    }
//                }
//            }
//        }
//    }
//    pub fn get(&self, id: UUID) -> Option<&FsType> {
//        match self.filesystems.get(&id) {
//            None => None,
//            Some(fs) => Some(&fs.1)
//        }
//    }
//    pub fn iter(&self) -> hashbrown::hash_map::Iter<UUID, (u32, FsType)> {
//        self.filesystems.iter()
//    }
//}
