///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

#![allow(dead_code)]
//! FIS Types
//!
//! FIS Types do not have `Volatile` fields because they should only be read from or written to
//! through a `Volatile<SomeFisType>`.

use crate::driver::ahci::constants::AtaCommand;
use crate::util::BufferWrite;
use alloc::vec::Vec;


#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum FisType {
    RegisterHostToDevice = 0x27, // Register FIS - host to device
    RegisterDeviceToHost = 0x34, // Register FIS - device to host
    DMAActivate          = 0x39, // DMA activate FIS - device to host
    DMASetup             = 0x41, // DMA setup FIS - bidirectional
    Data                 = 0x46, // Data FIS - bidirectional
    BISTActivate         = 0x58, // BIST activate FIS - bidirectional
    PIOSetup             = 0x5F, // PIO setup FIS - device to host
    SetDeviceBits        = 0xA1, // Set device bits FIS - device to host
}

#[derive(Debug)]
pub enum Fis {
    FisRegisterHostToDevice(FisRegisterHostToDevice),
    //FisRegisterDeviceToHost(FisRegisterDeviceToHost),
    //FisPioSetup(FisPioSetup),
    //FisData(FisData),
    //FisDmaSetup(FisDmaSetup),
}
impl Fis {
    pub fn fis_type(&self) -> FisType {
        match self {
            Fis::FisRegisterHostToDevice(_) => FisType::RegisterHostToDevice,
            //Fis::FisRegisterDeviceToHost(_) => FisType::RegisterDeviceToHost,
            //Fis::FisPioSetup(_) => FisType::PIOSetup,
            //Fis::FisData(_) => FisType::Data,
            //Fis::FisDmaSetup(_) => FisType::DMASetup,
        }
    }
    pub fn write_to_buffer(&self, bytes: &mut Vec<u8>) {
        match self {
            Fis::FisRegisterHostToDevice(f) => f.write_to_buffer(bytes),
            //_ => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub struct FisRegisterHostToDevice {
    pub port_mult_port: u8,
    pub is_from_command: bool,
    pub command: AtaCommand,
    pub features: u16,
    pub lba_address: u64,
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
impl BufferWrite for FisRegisterHostToDevice {
    fn write_to_buffer(&self, bytes: &mut Vec<u8>) {
        bytes.resize(20, 0);
        // DWORD 0
        // 0x00: FIS type
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

#[derive(Debug, Copy, Clone)]
#[repr(C)]
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

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct FisData {
    // DWORD 0
    pub fis_type: u8, // Always FisType::Data

    pub port_mult: u8,     // [7-4]: Port multiplier, [3-0]: Reserved
    pub reserved: [u8; 2], // Reserved

    // DWORD 1 ~ N
    pub data: [u32; 1], // Payload
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
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

#[derive(Debug, Copy, Clone)]
#[repr(C)]
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
