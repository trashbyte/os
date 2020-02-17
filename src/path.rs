#![allow(dead_code)]

use alloc::string::{String, ToString};
use core::ops::Div;
use core::iter::Iterator;
use core::fmt::{Display, Formatter, Error};

#[derive(Clone)]
pub struct Path(String);

impl Path {
    pub fn new() -> Self { Self(String::new()) }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.split("/").filter(|e| e.len() > 0)
    }

    pub fn sanitize(input: &str) -> String {
        // TODO
        input.to_string()
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
        s.push_str(rhs.0.as_str());
        Self(s)
    }
}
impl Div<String> for Path {
    type Output = Path;

    fn div(self, rhs: String) -> Self::Output {
        let mut s = self.0;
        s.push_str(&rhs);
        Self(s)
    }
}
impl Div<&String> for Path {
    type Output = Path;

    fn div(self, rhs: &String) -> Self::Output {
        let mut s = self.0;
        s.push_str(rhs.as_str());
        Self(s)
    }
}
impl Div<&str> for Path {
    type Output = Path;

    fn div(self, rhs: &str) -> Self::Output {
        let mut s = self.0;
        s.push_str(rhs);
        Self(s)
    }
}
impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{}", self.0)
    }
}
