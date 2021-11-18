///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![allow(dead_code)]
//! Frame Information Structure Types
//!
//! FIS Types do not have `Volatile` fields because they should only be read from or written to
//! through a `Volatile<SomeFisType>`.

use crate::driver::ahci::constants::AtaCommand;
use crate::util::BufferWrite;
use alloc::vec::Vec;

/// All of the possible FIS types (excluding reserved or vendor-specific types)
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum FisType {
    /// Register FIS - host to device
    RegisterHostToDevice = 0x27,
    /// Register FIS - device to host
    RegisterDeviceToHost = 0x34,
    /// DMA activate FIS - device to host
    DMAActivate          = 0x39,
    /// DMA setup FIS - bidirectional
    DMASetup             = 0x41,
    /// Data FIS - bidirectional
    Data                 = 0x46,
    /// BIST activate FIS - bidirectional
    BISTActivate         = 0x58,
    /// PIO setup FIS - device to host
    PIOSetup             = 0x5F,
    /// Set device bits FIS - device to host
    SetDeviceBits        = 0xA1,
}

/// A Frame Information Structure for transmitting through SATA
#[derive(Debug)]
pub enum Fis {
    /// Register FIS - host to device
    FisRegisterHostToDevice(FisRegisterHostToDevice),
    //FisRegisterDeviceToHost(FisRegisterDeviceToHost),
    //FisPioSetup(FisPioSetup),
    //FisData(FisData),
    //FisDmaSetup(FisDmaSetup),
}
impl Fis {
    /// Returns the FisType corresponding to which `Fis` variant this is.
    pub fn fis_type(&self) -> FisType {
        match self {
            Fis::FisRegisterHostToDevice(_) => FisType::RegisterHostToDevice,
            //Fis::FisRegisterDeviceToHost(_) => FisType::RegisterDeviceToHost,
            //Fis::FisPioSetup(_) => FisType::PIOSetup,
            //Fis::FisData(_) => FisType::Data,
            //Fis::FisDmaSetup(_) => FisType::DMASetup,
        }
    }
    /// Writes the bytes that make up this FIS to the provided buffer.
    pub fn write_to_buffer(&self, bytes: &mut Vec<u8>) {
        match self {
            Fis::FisRegisterHostToDevice(f) => f.write_to_buffer(bytes),
            //_ => unimplemented!(),
        }
    }
}

/// FIS for transfering the shadow register block from the SATA host to a device.
/// This is how commands are sent to a device.
#[derive(Debug)]
pub struct FisRegisterHostToDevice {
    /// The target port on the port multiplier, if one is being used
    pub port_mult_port: u8,
    /// True: command, false: control
    pub is_from_command: bool,
    /// Contents of the command register (the ATA command sent to the device)
    pub command: AtaCommand,
    /// Feature register
    pub features: u16,
    /// LBA address to target with the issued ATA command
    pub lba_address: u64,
    /// Device register
    pub device: u8,
    pub count: u16,
}
impl FisRegisterHostToDevice {
    pub fn new(lba_address: u64) -> Self {
        Self {
            port_mult_port: 0,
            is_from_command: true,
            command: AtaCommand::ReadDmaExt,
            features: 0,
            lba_address,
            device: 0,
            count: 0,
        }
    }
}
impl Into<Fis> for FisRegisterHostToDevice {
    fn into(self) -> Fis { Fis::FisRegisterHostToDevice(self) }
}
impl BufferWrite for FisRegisterHostToDevice {
    fn write_to_buffer(&self, bytes: &mut Vec<u8>) {
        bytes.resize(20, 0);
        // DWORD 0
        // 0x00: FIS type - FIS_TYPE_REG_H2D 0x27
        bytes[0x00] = FisType::RegisterHostToDevice as u8;
        // 0x01: [7-4]: Port mult, [3-1]: Reserved, [0]: 1=Command 0=Control
        bytes[0x01] = (self.port_mult_port & 0b11110000) | (match self.is_from_command { true => 1, false => 0 });
        // 0x02: Command register
        bytes[0x02] = self.command as u8;
        // 0x03: Features register, low byte
        bytes[0x03] = (self.features & 0xFF) as u8;
        // DWORD 1
        // 0x04: LBA low register (7:0)
        bytes[0x04] = (self.lba_address & 0xFF) as u8;
        // 0x05: LBA mid register (15:8)
        bytes[0x05] = ((self.lba_address >> 8) & 0xFF) as u8;
        // 0x06: LBA high register (23:16)
        bytes[0x06] = ((self.lba_address >> 16) & 0xFF) as u8;
        // 0x07: Device register
        bytes[0x07] = self.device;
        // DWORD 2
        // 0x08: LBA exp low register (31:24)
        bytes[0x08] = ((self.lba_address >> 24) & 0xFF) as u8;
        // 0x09: LBA exp mid register (39:32)
        bytes[0x09] = ((self.lba_address >> 32) & 0xFF) as u8;
        // 0x0A: LBA exp high register (47:40)
        bytes[0x0A] = ((self.lba_address >> 40) & 0xFF) as u8;
        // 0x0B: Feature register (expanded) (15:8)
        bytes[0x0B] = ((self.features >> 8) & 0xFF) as u8;
        // DWORD 3
        // 0x0C-0x0D: Count register
        bytes[0x0C] = (self.count & 0xFF) as u8;
        bytes[0x0D] = ((self.count >> 8) & 0xFF) as u8;
        // 0x0E: Reserved
        bytes[0x0E] = 0;
        // 0x0F: Control register (only used for legacy purposes)
        bytes[0x0F] = 0;
        // DWORD 4
        // 0x10-0x13: Reserved
        bytes[0x10] = 0; bytes[0x11] = 0; bytes[0x12] = 0; bytes[0x13] = 0;
    }
}

////////////////////////////////////////////////////////////////////////////////
//  END OF STUFF THAT IS DONE
////////////////////////////////////////////////////////////////////////////////

/// FIS for transfering the shadow register block from a SATA device to the host.
/// This is how the result or status of a command is sent from a device to the host.
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct FisRegisterDeviceToHost {
    // DWORD 0
    pub fis_type: u8, // Always FisType::RegisterDeviceToHost

    pub pmult_and_int: u8, // [7-4]: Port mult, [3-2]: Reserved, [1]: Interrupt bit, [0]: Reserved
    pub status:        u8, // Status register
    pub error:         u8, // Error register

    // DWORD 1
    pub lba0:   u8, // LBA low register, 7:0
    pub lba1:   u8, // LBA mid register, 15:8
    pub lba2:   u8, // LBA high register, 23:16
    pub device: u8, // Device register

    // DWORD 2
    pub lba3:     u8, // LBA register, 31:24
    pub lba4:     u8, // LBA register, 39:32
    pub lba5:     u8, // LBA register, 47:40
    pub reserved1: u8, // Reserved

    // DWORD 3
    pub count_low:  u8,      // Count register, 7:0
    pub count_high: u8,     // Count register, 15:8
    pub reserved2: [u8; 2], // Reserved

    // DWORD 4
    pub reserved3: [u8; 4] // Reserved
}

/// FIS for transferring payload data (media reads and writes, etc)
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct FisData {
    // DWORD 0
    pub fis_type: u8, // Always FisType::Data

    pub port_mult: u8,     // [7-4]: Port multiplier, [3-0]: Reserved
    pub reserved: [u8; 2], // Reserved

    // DWORD 1 ~ N
    pub data: [u32; 1], // Payload
}

/// FIS that a devices uses to provide the host with the necessary data
/// before initializing a Port I/O transfer.
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct FisPioSetup {
    // DWORD 0
    pub fis_type: u8, // Always FisType::PioSetup

    // [7-4]: Port multiplier,
    // [3]: Reserved,
    // [2]: Transfer dir, 1=device to host,
    // [1]: Interrupt bit,
    // [0]: Reserved
    pub pmult_dir_int: u8,

    pub status: u8, // Status register
    pub error:  u8, // Error register

    // DWORD 1
    pub lba0:   u8, // LBA low register, 7:0
    pub lba1:   u8, // LBA mid register, 15:8
    pub lba2:   u8, // LBA high register, 23:16
    pub device: u8, // Device register

    // DWORD 2
    pub lba3:     u8, // LBA register, 31:24
    pub lba4:     u8, // LBA register, 39:32
    pub lba5:     u8, // LBA register, 47:40
    pub reserved1: u8, // Reserved

    // DWORD 3
    pub count_low:  u8, // Count register, 7:0
    pub count_high: u8, // Count register, 15:8
    pub reserved2:  u8, // Reserved
    pub e_status:   u8, // New value of status register

    // DWORD 4
    pub trans_count: u16,  // Transfer count
    pub reserved3: [u8; 2] // Reserved
}

/// Bidirectional FIS used for configuring either the host or a device before a DMA transfer.
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
pub struct FisDmaSetup {
    // DWORD 0
    pub fis_type: u8, // Always FisType::DmaSetup

    // [7-4]: Port multiplier,
    // [3]: Reserved,
    // [2]: Transfer dir, 1=device to host,
    // [1]: Interrupt bit,
    // [0]: Auto-activate. Specifies if DMA Activate FIS is needed
    pub pmult_dir_int: u8,

    pub reserved1: [u8; 2], // Reserved

    //DWORD 1&2
    // DMA Buffer Identifier. Used to Identify DMA buffer in host memory.
    // SATA Spec says host specific and not in Spec. Trying AHCI spec might work.
    pub dma_buffer_id: u64,

    //DWORD 3
    pub reserved2: u32, // Reserved

    //DWORD 4
    pub dma_buf_offset: u32, // Byte offset into buffer. First 2 bits must be 0

    //DWORD 5
    pub transfer_count: u32, // Number of bytes to transfer. Bit 0 must be 0

    //DWORD 6
    pub reserved3: u32, // Reserved
}
