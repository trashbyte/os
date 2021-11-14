// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

#[cfg(target_arch = "x86_64")]
#[macro_use]
pub mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use self::x86_64::*;
