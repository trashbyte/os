//// The MIT License (MIT)
//// Copyright (c) 2020 trashbyte
//// See LICENSE.txt for full license
//
//
//use crate::fs::ext2::Ext2Filesystem;
//use hashbrown::HashMap;
//use crate::path::Path;
//use alloc::boxed::Box;
//use alloc::vec::Vec;
//use alloc::string::String;
//
//pub enum VfsError {
//    FileNotFound,
//    FileInUse,
//    InvalidHandle,
//    AlreadyMounted,
//    PathNotMounted
//}
//
//pub trait Filesystem {
//    fn mount() -> Result<FsHandle, VfsError>;
//    fn release(handle: FsHandle) -> Result<(), VfsError>;
//}
//
//pub struct VFS {
//    mounts: HashMap<Path, Box<dyn Filesystem>>,
//    root_node: VfsNode,
//}
//impl VFS {
//    pub fn mount(&mut self, path: Path, fs: Box<dyn Filesystem>) -> Result<(), VfsError> {
//        match self.mounts.get(&path) {
//            Some(_) => { Err(VfsError::AlreadyMounted) },
//            None => {
//                self.mounts.insert(path, fs);
//                Ok(())
//            }
//        }
//    }
//    pub fn unmount(&mut self, path: Path, fs: Box<dyn Filesystem>) -> Result<(), VfsError> {
//        if self.mounts.contains_key(&path) {
//            self.mounts.remove(&path);
//            Ok(())
//        }
//        else {
//            Err(VfsError::PathNotMounted)
//        }
//    }
//    pub fn list_dir(&self, path: Path) -> Result<Vec<FileInfo>, VfsError> {
//        let mut result = Vec::new();
//        let mut node = self.root_node.clone();
//        for segment in path.iter() {
//
//        }
//        Ok(result)
//    }
//}
//
//#[derive(Debug, Clone)]
//pub struct VfsNode {
//    pub name: String,
//}
//
//#[derive(Debug, Clone)]
//pub struct FileInfo {
//    pub name: String,
//}
