///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

// Size constants //////////////////////////////////////////////////////////////////////////////////

pub const RECEIVED_FIS_SIZE: u64 = 256;
pub const COMMAND_HEADER_SIZE: u64 = 32;
pub const COMMAND_LIST_TOTAL_SIZE: u64 = COMMAND_HEADER_SIZE * 32;
pub const PRDT_OFFSET_IN_TABLE: u64 = 0x80;
pub const PRDT_SIZE: u64 = 16;
pub const NUM_PRDTS_PER_COMMAND: u64 = 32;
pub const PRDT_LIST_TOTAL_SIZE: u64 = PRDT_SIZE * NUM_PRDTS_PER_COMMAND;
// Command tables must be 128-byte aligned, but 4224 is already a multiple of 128
pub const COMMAND_TABLE_SIZE: u64 = PRDT_LIST_TOTAL_SIZE + PRDT_OFFSET_IN_TABLE; // 4224 bytes
pub const COMMAND_TABLE_LIST_OFFSET: u64 = COMMAND_LIST_TOTAL_SIZE + RECEIVED_FIS_SIZE + PRDT_LIST_TOTAL_SIZE;
pub const PORT_MEMORY_SIZE: u64 = COMMAND_TABLE_LIST_OFFSET + COMMAND_TABLE_SIZE * 32;
/// Total size of AHCI memory region in bytes.
pub const AHCI_MEMORY_SIZE: u64 = PORT_MEMORY_SIZE * 32;

// Miscellaneous ATA constants /////////////////////////////////////////////////////////////////////

pub const ATA_DEV_BUSY: u8 = 0x80;
pub const ATA_DEV_DRQ: u8 = 0x08;
pub const HBA_SSTS_PRESENT: u32 = 0x3;
pub const HBA_PXIS_TASK_FILE_ERR: u32 = 0x40000000;
pub const HBA_PORT_IS_ERR: u32 = 1 << 30 | 1 << 29 | 1 << 28 | 1 << 27;

// ATA commands ////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
/// ATA command codes
pub enum AtaCommand {
    ReadDma = 0xC8,
    ReadDmaExt = 0x25,
    WriteDma = 0xCA,
    WriteDmaExt = 0x35,
    AtapiCmdPacket = 0xA0,
    AtapiIdentifyPacket = 0xA1,
    Identify = 0xEC,
}
impl AtaCommand { pub fn as_u8(self) -> u8 { self as u8 } }

// HBA control /////////////////////////////////////////////////////////////////////////////////////

bitflags::bitflags! {
    #[allow(non_upper_case_globals)]
    pub struct AhciGlobalHostControlBit: u32 {
        const AhciEnable      = (1 << 31);
        const InterruptEnable = (1 << 1);
        const HbaReset        = (1 << 0);
    }
}

bitflags::bitflags! {
    /// Bitmasks for the per-port Command and Status register (PxCMD)
    #[allow(non_upper_case_globals)]
    pub struct HbaPortCmdBit: u32 {
        const Start             = 1;
        const SpinUpDevice      = 1 << 1;
        const PowerOnDevice     = 1 << 2;
        const FisReceiveEnable  = 1 << 4;
        const FisReceiveRunning = 1 << 14;
        const CmdListRunning    = 1 << 15;
    }
}
impl HbaPortCmdBit {
    pub fn as_u32(self) -> u32 { self.bits }
}

// HBA port types //////////////////////////////////////////////////////////////////////////////////

const HBA_SIGNATURE_SATA:   u32 = 0x00000101;
const HBA_SIGNATURE_SATAPI: u32 = 0xEB140101;
const HBA_SIGNATURE_SEMB:   u32 = 0xC33C0101;
const HBA_SIGNATURE_PM:     u32 = 0x96690101;

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u32)]
/// Valid signatures for various SATA devlices
pub enum HbaPortType {
    /// No device
    None,
    /// Unknown signature
    Unknown(u32),
    /// SATA drive
    SATA,
    /// SATAPI drive
    SATAPI,
    /// Enclosure management bridge
    SEMB,
    /// Port multiplier
    PortMult,
}
impl HbaPortType {
    pub fn from_signature(sig: u32) -> Self {
        match sig {
            HBA_SIGNATURE_SATA => Self::SATA,
            HBA_SIGNATURE_SATAPI => Self::SATAPI,
            HBA_SIGNATURE_SEMB => Self::SEMB,
            HBA_SIGNATURE_PM => Self::PortMult,
            _ => Self::Unknown(sig)
        }
    }

    pub fn to_signature(self) -> u32 {
        match self {
            Self::SATA => HBA_SIGNATURE_SATA,
            Self::SATAPI => HBA_SIGNATURE_SATAPI,
            Self::SEMB => HBA_SIGNATURE_SEMB,
            Self::PortMult => HBA_SIGNATURE_PM,
            Self::Unknown(u) => u,
            Self::None => 0
        }
    }
}

// Power management ////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum HbaPortPowerState {
    Unknown = 0,
    Active = 1,
    Partial = 2,
    Slumber = 6,
    DevSleep = 8,

    ReservedValue = 0xFF
}

bitflags::bitflags !{
    #[allow(non_upper_case_globals)]
    pub struct HbaPortPwrTransitionDisable: u32 {
        /// No transition restrictions
        const None = 0;
        /// Transition to the Partial state is disallowed
        const PartialDisable = 1 << 8;
        /// Transition to the Slumber state is disallowed
        const SlumberDisable = 1 << 9;
        /// Transition to the DevSlep state is disallowed
        const DevSleepDisable = 1 << 10;
    }
}

// Device detection ////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum HbaPortDeviceDetectState {
    Unknown = 0,
    PresentNoComm = 1,
    PresentWithComm = 3,
    PhyOffline = 4,

    ReservedValue = 0xFF
}

// Interface speed /////////////////////////////////////////////////////////////////////////////////

pub const HBA_PORT_SPD_UNKNOWN: u8 = 0;
pub const HBA_PORT_SPD_GEN1:    u8 = 1;
pub const HBA_PORT_SPD_GEN2:    u8 = 2;
pub const HBA_PORT_SPD_GEN3:    u8 = 3;
