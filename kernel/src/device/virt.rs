///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![allow(dead_code)]


#[derive(Clone, Copy, Debug)]
pub enum VirtualDeviceType {
    Character,
    Block
}

#[derive(Clone, Copy, Debug)]
pub struct VirtualDeviceId(pub u32);
impl VirtualDeviceId {
    pub fn as_u32(&self) -> u32 { self.0 }
}

#[derive(Clone, Copy, Debug)]
pub struct VirtualDevice {
    id: VirtualDeviceId,
    dev_type: VirtualDeviceType,
}
