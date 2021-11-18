///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

/// Architecture specific implementations for x86_64
#[cfg(target_arch = "x86_64")]
#[macro_use]
pub mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use self::x86_64::*;

/// Architecture specific implementations for aarch64
#[cfg(target_arch = "aarch64")]
#[macro_use]
pub mod aarch64;
#[cfg(target_arch = "aarch64")]
pub use self::aarch64::*;
