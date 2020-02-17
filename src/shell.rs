// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::print;
use crate::fs::ext2::{Ext2Filesystem, DirectoryEntryType};
use crate::path::Path;


lazy_static! {
    pub static ref SHELL: Mutex<Shell> = Mutex::new(Shell {
        command_str: String::new(),
        working_directory: Path::from("/")
    });
}

pub struct Shell {
    command_str: String,
    working_directory: Path,
}
impl Shell {
    pub fn add_char(&mut self, c: char) {
        self.command_str.push(c);
    }

    pub fn prompt(&self) {
        print!("[{}] > ", self.working_directory);
    }

    pub fn submit(&mut self) {
        let s = self.command_str.trim();
        if s.len() > 0 {
            if s == "hello" {
                print!("Hi!");
            }
            else if s == "ls" {
                unsafe {
                    let lock = crate::service::DISK_SERVICE.lock();
                    let drive = (*lock).get(2).unwrap();
                    let ext2_fs = Ext2Filesystem::read_from(drive).unwrap();
                    let dir = ext2_fs.list_directory(self.working_directory.clone(), drive);
                    for e in dir.iter().filter(|e| e.type_indicator == DirectoryEntryType::Directory) {
                        print!("    {}/\n", e.file_name);
                    }
                    for e in dir.iter().filter(|e| e.type_indicator != DirectoryEntryType::Directory) {
                        print!("    {}\n", e.file_name);
                    }
                }
            }
            else if &s[0..2] == "cd" {
                let path_str = &s[3..s.len()];
                let path = Path::from(path_str);
                if path.is_relative() {
                    let lock = crate::service::DISK_SERVICE.lock();
                    let drive = (*lock).get(2).unwrap();
                    unsafe {
                        let ext2_fs = Ext2Filesystem::read_from(drive).unwrap();
                        let dir = ext2_fs.list_directory(self.working_directory.clone(), drive);
                        let mut matching = dir.iter().filter(|e| &e.file_name == path_str);
                        match matching.next() {
                            Some(p) => {
                                self.working_directory = self.working_directory.clone() / &p.file_name;
                            },
                            None => print!("No '{}' in current directory.", path_str)
                        }
                    }
                }
                else {
                    unimplemented!()
                }
            }
//            else if s == "uuid" {
//                let lock = crate::service::FS_SERVICE.lock();
//                for (uuid, (disk_id, fst)) in (*lock).iter() {
//                    print!("    Disk {} Partition 1 [{}]: {}\n", disk_id, fst.type_as_str(), uuid);
//                }
//            }
            else {
                print!("Command '{}' not found.", self.command_str);
            }
            print!("\n");
        }
        self.command_str = String::new();
        self.prompt();
    }
}
