// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::print;
use crate::fs::VfsNodeType;
use crate::path::Path;
use crate::fs::vfs::VFS;


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
            if s == "ls" {
                unsafe {
                    let vfs: &VFS = &(*crate::fs::vfs::GLOBAL_VFS.as_ref().unwrap().lock());
                    let dir = vfs.list_dir(self.working_directory.clone()).unwrap();
                    for e in dir.iter().filter(|e| e.entry_type == VfsNodeType::Directory) {
                        print!("    {}/\n", e.file_name);
                    }
                    for e in dir.iter().filter(|e| e.entry_type != VfsNodeType::Directory) {
                        print!("    {}\n", e.file_name);
                    }
                }
            }
            else if &s[0..2] == "cd" {
                let path_str = &s[3..s.len()];
                let path = Path::from(path_str);
                // todo: should path .. logic go somewhere else? probably
                if path.as_str() == ".." {
                    if self.working_directory.as_str() == "/" {
                        // cant go up from root, do nothing
                    }
                    else {
                        // don't need to test if folder is there,
                        // we've already validated the path to get here
                        let len = self.working_directory.iter().count() + 1;
                        // we make some assumptions here based on the fact that the
                        // working directory should always be absolute
                        match len {
                            0 => unreachable!(), // ???
                            1 => unreachable!(), // this could only be / which shouldnt be possible
                                                 // here (string contains "..")
                            2 => unreachable!(), // this could only be /.. which should have
                                                 // already been handled above
                            3 => {
                                //   /somedir/..  ->  can go directly to root
                                self.working_directory = Path::from("/");
                                // todo: what about /../somedir?
                            },
                            _ => {
                                let mut path_str = String::from("/");
                                // path looks something like /foo/bar/..
                                // first is root, skip it, also skip the last two
                                // since we're going up a dir
                                for i in 1..len-2 {
                                    path_str.push('/');
                                    path_str.push_str(self.working_directory.iter().nth(i).unwrap());
                                }
                                self.working_directory = Path::from(path_str);
                            }
                        }
                    }
                }
                else if path.is_relative() {
                    unsafe {
                        let vfs: &VFS = &(*crate::fs::vfs::GLOBAL_VFS.as_ref().unwrap().lock());
                        let dir = vfs.list_dir(self.working_directory.clone()).unwrap();
                        let mut matching = dir.iter().filter(|e| &e.file_name == path_str);
                        match matching.next() {
                            Some(p) => {
                                self.working_directory = self.working_directory.clone() / &p.file_name;
                            },
                            None => print!("Couldn't find '{}'.", path_str)
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

    /// Returns true if a character was deleted
    pub fn backspace(&mut self) -> bool {
        self.command_str.pop().is_some()
    }
}
