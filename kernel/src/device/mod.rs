///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

/// Physical devices (drives, etc)
pub mod physical;
/// Virtual devices (like /dev/null)
pub mod virt;
/// Block devices (read or write 4kiB blocks)
pub mod block;
/// Serial devices (for printing output or receiving input from a physical terminal)
pub mod serial;
