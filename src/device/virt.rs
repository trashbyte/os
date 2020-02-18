// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

#![allow(dead_code)]


pub enum VirtualDeviceType {
    Character,
    Block
}

pub struct VirtualDeviceId(pub u32);
impl VirtualDeviceId {
    pub fn as_u32(&self) -> u32 { self.0 }
}

pub struct VirtualDevice {
    id: VirtualDeviceId,
    dev_type: VirtualDeviceType,
}
