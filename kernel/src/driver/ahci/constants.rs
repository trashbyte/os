///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use num_derive::FromPrimitive;


pub const COMMAND_LIST_SIZE: u64 = 1024;
pub const COMMAND_FIS_SIZE: u64 = 64;
pub const RECEIVING_FIS_SIZE: u64 = 256;
pub const COMMAND_HEADER_SIZE: u64 = 32;
pub const PRDT_OFFSET_IN_TABLE: u64 = 0x80;
pub const PRDT_SIZE: u64 = 16;
pub const PRDT_LIST_TOTAL_SIZE: u64 = PRDT_SIZE * 256;
pub const COMMAND_TABLE_SIZE: u64 = PRDT_LIST_TOTAL_SIZE + PRDT_OFFSET_IN_TABLE; // 4224 bytes

/// Total size of AHCI memory region in bytes.
pub const AHCI_MEMORY_SIZE: u64 = (COMMAND_LIST_SIZE + RECEIVING_FIS_SIZE + PRDT_LIST_TOTAL_SIZE) * 32;

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
/// ATA command codes
pub enum AtaCommand {
    ReadDma = 0xC8,
    ReadDmaExt = 0x25,
    WriteDma = 0xCA,
    WriteDmaExt = 0x35,
}
impl AtaCommand { pub fn as_u8(self) -> u8 { self as u8 } }

pub const ATA_DEV_BUSY: u32 = 0x80;
pub const ATA_DEV_DRQ: u32 = 0x08;
pub const HBA_PxIS_TASK_FILE_ERR: u32 = 0x40000000;

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u32)]
/// Bitmasks for the per-port PxCMD register (Command and Status)
pub enum HbaPxCMDBit {
    Start             = 0x0001,
    FisReceiveEnable  = 0x0010,
    FisReceiveRunning = 0x4000,
    CmdListRunning    = 0x8000,
}
impl HbaPxCMDBit { pub fn as_u32(self) -> u32 { self as u32 } }

#[derive(Debug, Copy, Clone, PartialEq, FromPrimitive)]
#[repr(u32)]
/// Valid signatures for various SATA devlices
pub enum SataSignature {
    ATA      = 0x00000101, // SATA drive
    ATAPI    = 0xEB140101, // SATAPI drive
    SEMB     = 0xC33C0101, // Enclosure management bridge
    PortMult = 0x96690101, // Port multiplier
}
impl SataSignature {
    pub fn as_u32(self) -> u32 { self as u32 }
}

// Interface Power Management
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum HbaPortPowerState {
    Unknown = 0,
    Active = 1,
    PartialPowerManagement = 2,
    Slumber = 6,
    DevSleep = 8,

    ReservedValue = 0xFF
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
// Device Detection
pub enum HbaPortDeviceDetectState {
    Unknown = 0,
    PresentNoComm = 1,
    PresentWithComm = 3,
    PhyOffline = 4,

    ReservedValue = 0xFF
}

// Interface Speed
pub const HBA_PORT_SPD_UNKNOWN: u8 = 0;
pub const HBA_PORT_SPD_GEN1:    u8 = 1;
pub const HBA_PORT_SPD_GEN2:    u8 = 2;
pub const HBA_PORT_SPD_GEN3:    u8 = 3;

pub enum AhciGlobalHostControlBit {
    AhciEnable      = (1 << 31),
    InterruptEnable = (1 << 1),
    HbaReset        = (1 << 0),
}
