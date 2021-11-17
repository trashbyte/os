///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![allow(dead_code)]

use alloc::string::{String, ToString};
use core::ops::Div;
use core::iter::Iterator;
use core::fmt::{Display, Formatter, Error};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(String);

impl Path {
    pub fn new() -> Self { Self(String::new()) }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        // necessary to keep the compiler from complaining about mismatched closure types
        let closure = |s: &&str| { s.len() > 0 };
        match self.is_absolute() {
            true => { ["/"].iter().cloned().chain(self.0.split("/").filter(closure)) },
            false => { [].iter().cloned().chain(self.0.split("/").filter(closure)) }
        }
    }

    /// Accumulating iter, e.g. / -> /foo -> /foo/bar
    pub fn iter_accum(&self) -> PathAccumIter<'_> {
        PathAccumIter::new(self)
    }

    pub fn sanitize(input: &str) -> String {
        // TODO
        let mut s = input.to_string();
        while s.contains("//") {
            s = s.replace("//", "/");
        }
        s
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() < 1
    }
    pub fn is_absolute(&self) -> bool {
        !self.is_empty() && self.0.chars().nth(0).unwrap() == '/'
    }
    pub fn is_relative(&self) -> bool {
        !self.is_empty() && self.0.chars().nth(0).unwrap() != '/'
    }
    pub fn is_root(&self) -> bool {
        !self.is_empty() && self.iter().count() == 0
    }
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if this path resolves to a location that is a child of the other path.
    /// e.g. /some/thing/nice is a subpath of /some
    ///
    /// NOTE: currently does not handle symlinks etc, only plain old paths
    /// (its basically a plain string comparison per segment)
    // TODO: implement is_subpath_of for symlinks etc
    pub fn is_subpath_of(&self, other: &Path) -> bool {
        if *self == *other {
            // identical paths aren't subpaths of each other
            return false;
        }
        if self.as_str().len() < other.as_str().len() {
            // assuming we aren't checking symlinks, if our path is shorter it cant be a subpath
            return false;
        }
        for (a, b) in self.iter().zip(other.iter()) {
            if a != b {
                // found differing segments at the same depth, paths diverge
                return false;
            }
        }
        // we already know this path is longer than the other, and all the segments up until
        // now have matched, so this is a valid supbath
        true
    }
}
impl From<String> for Path {
    fn from(s: String) -> Self {
        Self(Path::sanitize(&s))
    }
}
impl From<&str> for Path {
    fn from(s: &str) -> Self {
        Self(Path::sanitize(s))
    }
}
impl Div<Path> for Path {
    type Output = Path;

    fn div(self, rhs: Path) -> Self::Output {
        let mut s = self.0;
        s.push('/');
        s.push_str(rhs.0.as_str());
        Path::from(s)
    }
}
impl Div<String> for Path {
    type Output = Path;

    fn div(self, rhs: String) -> Self::Output {
        let mut s = self.0;
        s.push('/');
        s.push_str(&rhs);
        Path::from(s)
    }
}
impl Div<&String> for Path {
    type Output = Path;

    fn div(self, rhs: &String) -> Self::Output {
        let mut s = self.0;
        s.push('/');
        s.push_str(rhs.as_str());
        Path::from(s)
    }
}
impl Div<&str> for Path {
    type Output = Path;

    fn div(self, rhs: &str) -> Self::Output {
        let mut s = self.0;
        s.push('/');
        s.push_str(rhs);
        Path::from(s)
    }
}
impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct PathAccumIter<'a> {
    path: &'a Path,
    accum_str: String,
    idx: usize,
}
impl<'a> PathAccumIter<'a> {
    pub fn new(p: &'a Path) -> Self {
        Self {
            path: p,
            accum_str: String::new(),
            idx: 0,
        }
    }
}
impl Iterator for PathAccumIter<'_> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.path.iter().nth(self.idx) {
            Some(s) => {
                self.accum_str.push_str(s);
                self.idx += 1;
                Some(self.accum_str.clone())
            },
            None => None
        }
    }
}
