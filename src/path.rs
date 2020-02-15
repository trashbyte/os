use alloc::string::{String, ToString};
use core::ops::Div;

pub struct Path(String);

impl Path {
    pub fn new() -> Self { Self(String::new()) }
}
impl From<String> for Path {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl From<&str> for Path {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
impl Div for Path {
    type Output = Path;

    fn div(self, rhs: Self) -> Self::Output {
        let mut s = self.0;
        s.push_str(rhs.0.as_str());
        Self(s)
    }
}
