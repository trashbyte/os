use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::print;
use crate::fs::ext2::Ext2Filesystem;
use crate::service::DiskType;


lazy_static! {
    pub static ref SHELL: Mutex<Shell> = Mutex::new(Shell { command_str: String::new() });
}

pub struct Shell {
    command_str: String,
}
impl Shell {
    pub fn add_char(&mut self, c: char) {
        self.command_str.push(c);
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
                    match drive {
                        DiskType::ATA(ref d) => {
                            let ext2_fs = Ext2Filesystem::read_from(d).unwrap();
                            let dir = ext2_fs.list_directory("/".into(), d);
                            for e in dir {
                                print!("    {}\n", e.file_name);
                            }
                        }
                    }
                }
            }
            else if s == "uuid" {
                let lock = crate::service::FS_SERVICE.lock();
                for (uuid, (disk_id, fst)) in (*lock).iter() {
                    print!("    Disk {} Partition 1 [{}]: {}\n", disk_id, fst.type_as_str(), uuid);
                }
            }
            else {
                print!("Command '{}' not found.", self.command_str);
            }
            print!("\n");
        }
        print!("> ");
        self.command_str = String::new();
    }
}
