// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

#![allow(dead_code)]


use crate::driver::ata::AtaDrive;

pub enum PhysicalDeviceType {
    FloppyDrive,
    AtaDrive(AtaDrive),
    SataDrive,
    NVMeDrive,
}

pub struct PhysicalDeviceId(pub u32);
impl PhysicalDeviceId {
    pub fn as_u32(&self) -> u32 { self.0 }
}

pub struct PhysicalDevice {
    id: PhysicalDeviceId,
    dev_type: PhysicalDeviceType,
}

