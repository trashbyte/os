///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

//use crate::driver::ata::{AtaDrive, ide_identify};
//use crate::fs::ext2::Ext2Filesystem;
use hashbrown::HashMap;
use spin::Mutex;
use crate::device::physical::SyncDisk;

pub static DISK_SERVICE: Mutex<Option<DiskService>> = Mutex::new(None);
//pub static ref FS_SERVICE: Mutex<FsService> = Mutex::new(FsService::new());


pub struct DiskService {
    disks: HashMap<u32, SyncDisk, ahash::RandomState>,
    next_id: u32,
}
impl DiskService {
    pub async fn init() {
        if DISK_SERVICE.lock().is_some() {
            crate::both_println!("ERROR: Disk service is already initialized");
        }
        // TODO: IDE drives
        // for bus in 0..2 {
        //     for device in 0..2 {
        //         unsafe {
        //             if let Some(info) = ide_identify(bus, device) {
        //                 self.drives.insert(self.next_id, Arc::new(AtaDrive::from_identify(info, bus, device)));
        //                 self.next_id += 1;
        //             }
        //         }
        //     }
        // }

        let mut next_id = 0;
        let mut disks = HashMap::default();
        let scanned_disks = crate::driver::ahci::scan_disks().await;
        for disk in scanned_disks.iter() {
            disks.insert(next_id, disk.clone());
            next_id += 1;
        }

        *DISK_SERVICE.lock() = Some(Self { disks, next_id });
        crate::both_println!("Disk service initialized");
    }
    pub fn get(&self, id: u32) -> Option<SyncDisk> {
        match self.disks.get(&id) {
            None => None,
            Some(dt) => Some(dt.clone())
        }
    }
    pub fn iter(&self) -> hashbrown::hash_map::Iter<'_, u32, SyncDisk> {
        self.disks.iter()
    }
    pub fn iter_mut(&mut self) -> hashbrown::hash_map::IterMut<'_, u32, SyncDisk> {
        self.disks.iter_mut()
    }
}

// #[derive(Debug)]
// pub enum FsType {
//     Ext2(Ext2Filesystem)
// }
// impl FsType {
//     pub fn type_as_str(&self) -> &'static str {
//         match self {
//             FsType::Ext2(_) => "Ext2"
//         }
//     }
// }
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
