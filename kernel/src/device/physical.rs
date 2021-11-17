///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![allow(dead_code)]


use crate::driver::ata::AtaDrive;

#[derive(Debug)]
pub enum PhysicalDeviceType {
    FloppyDrive,
    AtaDrive(AtaDrive),
    SataDrive,
    NVMeDrive,
}

#[derive(Clone, Copy, Debug)]
pub struct PhysicalDeviceId(pub u32);
impl PhysicalDeviceId {
    pub fn as_u32(&self) -> u32 { self.0 }
}

#[derive(Debug)]
pub struct PhysicalDevice {
    id: PhysicalDeviceId,
    dev_type: PhysicalDeviceType,
}

