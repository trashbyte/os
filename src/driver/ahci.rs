#![allow(dead_code)]

const SATA_SIG_ATA: u32   = 0x00000101; // SATA drive
const SATA_SIG_ATAPI: u32 = 0xEB140101; // SATAPI drive
const SATA_SIG_SEMB: u32  = 0xC33C0101; // Enclosure management bridge
const SATA_SIG_PM: u32    = 0x96690101; // Port multiplier

#[derive(Debug, Copy, Clone)]
pub enum SataDeviceType {
    NotPresent,
    SATA,
    SEMB,
    PortMult,
    SATAPI,
}

// Interface Power Management
const HBA_PORT_IPM_UNKNOWN:          u8 = 0;
const HBA_PORT_IPM_ACTIVE:           u8 = 1;
const HBA_PORT_IPM_POWER_MANAGEMENT: u8 = 2;
const HBA_PORT_IPM_SLUMBER:          u8 = 6;
const HBA_PORT_IPM_DEVSLEEP:         u8 = 8;

// Interface Speed
const HBA_PORT_SPD_UNKNOWN: u8 = 0;
const HBA_PORT_SPD_GEN1:    u8 = 1;
const HBA_PORT_SPD_GEN2:    u8 = 2;
const HBA_PORT_SPD_GEN3:    u8 = 3;

// Device Detection
const HBA_PORT_DET_UNKNOWN:           u8 = 0;
const HBA_PORT_DET_PRESENT_NO_COMM:   u8 = 1;
const HBA_PORT_DET_PRESENT_WITH_COMM: u8 = 3;
const HBA_PORT_DET_PHY_OFFLINE:       u8 = 4;

#[repr(C)]
pub struct HbaMemory {
    pub host_capability: u32,       // 0x00, Host capability
    pub global_host_control: u32,   // 0x04, Global host control
    pub interrupt_status: u32,      // 0x08, Interrupt status
    pub port_implemented: u32,		// 0x0C, Port implemented
    pub version: u32,               // 0x10, Version
    pub ccc_control: u32,           // 0x14, Command completion coalescing control
    pub ccc_ports: u32,             // 0x18, Command completion coalescing ports
    pub em_location: u32,           // 0x1C, Enclosure management location
    pub em_control: u32,            // 0x20, Enclosure management control
    pub host_capabilities_ext: u32, // 0x24, Host capabilities extended
    pub bios_handoff_control: u32,  // 0x28, BIOS/OS handoff control and status

    pub reserved: [u8; 0x74],         // 0x2C - 0x9F, Reserved
    pub vendor_registers: [u8; 0x60], // 0xA0 - 0xFF, Vendor specific registers

    pub port_registers:	[HbaPort; 32] // 0x100 - 0x10FF, Port control registers
}

#[repr(C)]
pub struct HbaPort {
    pub cmd_list_addr_aligned: u32, // 0x00, command list base address, 1K-byte aligned
    pub cmd_list_addr_upper: u32,   // 0x04, command list base address upper 32 bits
    pub fis_base_addr_aligned: u32, // 0x08, FIS base address, 256-byte aligned
    pub fis_base_addr_upper: u32,   // 0x0C, FIS base address upper 32 bits
    pub interrupt_status: u32,      // 0x10, interrupt status
    pub interrupt_enable: u32,      // 0x14, interrupt enable
    pub command_and_status: u32,    // 0x18, command and status
    pub reserved_area_1: u32,       // 0x1C, Reserved
    pub task_file_data: u32,        // 0x20, task file data
    pub signature: u32,             // 0x24, signature
    pub sata_status: u32,           // 0x28, SATA status (SCR0:SStatus)
    pub sata_control: u32,          // 0x2C, SATA control (SCR2:SControl)
    pub sata_error: u32,            // 0x30, SATA error (SCR1:SError)
    pub sata_active: u32,           // 0x34, SATA active (SCR3:SActive)
    pub command_issue: u32,         // 0x38, command issue
    pub sata_notification: u32,     // 0x3C, SATA notification (SCR4:SNotification)
    pub fis_switch_control: u32,    // 0x40, FIS-based switch control
    pub reserved_area_2: [u32; 11], // 0x44 ~ 0x6F, Reserved
    pub vendor_specific: [u32; 4]   // 0x70 ~ 0x7F, vendor specific
}

impl HbaPort {
    pub fn power_state(&self) -> u8 { ((self.sata_status >> 8) & 0x0F) as u8 }
    pub fn device_detect(&self) -> u8 { (self.sata_status & 0x0F) as u8 }

    pub fn device_type(&self) -> SataDeviceType {
        if self.device_detect() != HBA_PORT_DET_PRESENT_WITH_COMM {
            return SataDeviceType::NotPresent;
        }
        if self.power_state() != HBA_PORT_IPM_ACTIVE {
            return SataDeviceType::NotPresent;
        }

        match self.signature {
            SATA_SIG_ATA => SataDeviceType::SATA,
            SATA_SIG_SEMB => SataDeviceType::SEMB,
            SATA_SIG_PM => SataDeviceType::PortMult,
            SATA_SIG_ATAPI => SataDeviceType::SATAPI,
            _ => panic!("invalid device signature: {:#X}", self.signature)
        }
    }
}
