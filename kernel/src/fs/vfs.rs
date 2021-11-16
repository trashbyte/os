///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use hashbrown::HashMap;
use crate::path::Path;
use alloc::vec::Vec;
use crate::fs::{FsResult, FsError, Filesystem, VfsDirectoryEntry};
use spin::Mutex;
use alloc::sync::Arc;


pub static mut GLOBAL_VFS: Option<Mutex<VFS>> = None;

pub struct VFS {
    mounts: HashMap<Path, Arc<dyn Filesystem>, ahash::RandomState>,
    //root_node: VfsNode,
}
impl VFS {
    pub fn init(root: Arc<dyn Filesystem>) -> Self {
        let mut mounts = HashMap::default();
        mounts.insert("/".into(), root.clone());
        Self { mounts }
    }
    pub fn mount(&mut self, path: Path, fs: Arc<dyn Filesystem>) -> FsResult<()> {
        match self.mounts.get(&path) {
            Some(_) => { Err(FsError::AlreadyMounted) },
            None => {
                self.mounts.insert(path, fs);
                Ok(())
            }
        }
    }

    pub fn unmount(&mut self, path: Path) -> FsResult<()> {
        if self.mounts.contains_key(&path) {
            self.mounts.remove(&path);
            Ok(())
        }
        else {
            Err(FsError::PathNotMounted)
        }
    }

    pub fn fs_for_path(&self, path: &Path) -> FsResult<&Arc<dyn Filesystem>> {
        // store deepest match (we want to match a mount at /foo/bar with higher precedence than /)
        let mut deepest_path = Path::new();
        let mut deepest_mount = None;
        for (mount_path, mount) in self.mounts.iter() {
            if path == mount_path || path.is_subpath_of(mount_path) {
                // found parent path
                // is it deeper than what we've already found?
                if mount_path.as_str().len() > deepest_path.as_str().len() {
                    deepest_path = mount_path.clone();
                    deepest_mount = Some(mount);
                }
            }
        }
        match deepest_mount {
            Some(m) => {
                Ok(m)
            },
            None => {
                Err(FsError::PathNotMounted)
            }
        }
    }

    pub fn list_dir(&self, path: Path) -> FsResult<Vec<VfsDirectoryEntry>> {
        match self.fs_for_path(&path) {
            Ok(fs) => {
                // we have a mount for this path.
                // try to ls the path, forward any errors
                fs.list_directory(&path)
            },
            Err(e) => Err(e)
        }
    }
}
