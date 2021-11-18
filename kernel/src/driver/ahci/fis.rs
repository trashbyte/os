///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use volatile::Volatile;

#[repr(u8)]
#[derive(Debug)]
pub enum FisType {
    /// Register FIS - host to device
    RegH2D = 0x27,
    /// Register FIS - device to host
    RegD2H = 0x34,
    /// DMA activate FIS - device to host
    DmaAct = 0x39,
    /// DMA setup FIS - bidirectional
    DmaSetup = 0x41,
    /// Data FIS - bidirectional
    Data = 0x46,
    /// BIST activate FIS - bidirectional
    Bist = 0x58,
    /// PIO setup FIS - device to host
    PioSetup = 0x5F,
    /// Set device bits FIS - device to host
    DevBits = 0xA1
}

#[repr(packed)]
#[derive(Debug)]
pub struct FisRegH2D {
    // DWORD 0
    pub fis_type: Volatile<u8>, // FIS_TYPE_REG_H2D

    pub pm: Volatile<u8>, // Port multiplier, 1: Command, 0: Control

    pub command: Volatile<u8>, // Command register
    pub featurel: Volatile<u8>, // Feature register, 7:0

    // DWORD 1
    pub lba0: Volatile<u8>, // LBA low register, 7:0
    pub lba1: Volatile<u8>, // LBA mid register, 15:8
    pub lba2: Volatile<u8>, // LBA high register, 23:16
    pub device: Volatile<u8>, // Device register

    // DWORD 2
    pub lba3: Volatile<u8>, // LBA register, 31:24
    pub lba4: Volatile<u8>, // LBA register, 39:32
    pub lba5: Volatile<u8>, // LBA register, 47:40
    pub featureh: Volatile<u8>, // Feature register, 15:8

    // DWORD 3
    pub countl: Volatile<u8>, // Count register, 7:0
    pub counth: Volatile<u8>, // Count register, 15:8
    pub icc: Volatile<u8>, // Isochronous command completion
    pub control: Volatile<u8>, // Control register

    // DWORD 4
    pub rsv1: [Volatile<u8>; 4], // Reserved
}

#[repr(packed)]
#[derive(Debug)]
pub struct FisRegD2H {
    // DWORD 0
    pub fis_type: Volatile<u8>, // FIS_TYPE_REG_D2H

    pub pm: Volatile<u8>, // Port multiplier, Interrupt bit: 2

    pub status: Volatile<u8>, // Status register
    pub error: Volatile<u8>, // Error register

    // DWORD 1
    pub lba0: Volatile<u8>, // LBA low register, 7:0
    pub lba1: Volatile<u8>, // LBA mid register, 15:8
    pub lba2: Volatile<u8>, // LBA high register, 23:16
    pub device: Volatile<u8>, // Device register

    // DWORD 2
    pub lba3: Volatile<u8>, // LBA register, 31:24
    pub lba4: Volatile<u8>, // LBA register, 39:32
    pub lba5: Volatile<u8>, // LBA register, 47:40
    pub rsv2: Volatile<u8>, // Reserved

    // DWORD 3
    pub countl: Volatile<u8>, // Count register, 7:0
    pub counth: Volatile<u8>, // Count register, 15:8
    pub rsv3: [Volatile<u8>; 2], // Reserved

    // DWORD 4
    pub rsv4: [Volatile<u8>; 4], // Reserved
}

#[repr(packed)]
#[derive(Debug)]
pub struct FisData {
    // DWORD 0
    pub fis_type: Volatile<u8>, // FIS_TYPE_DATA

    pub pm: Volatile<u8>, // Port multiplier

    pub rsv1: [Volatile<u8>; 2], // Reserved

    // DWORD 1 ~ N
    pub data: [Volatile<u8>; 252], // Payload
}

#[repr(C)]
#[derive(Debug)]
pub struct FisPioSetup {
    // DWORD 0
    pub fis_type: Volatile<u8>, // FIS_TYPE_PIO_SETUP

    pub pm: Volatile<u8>, // Port multiplier, direction: 4 - device to host, interrupt: 2

    pub status: Volatile<u8>, // Status register
    pub error: Volatile<u8>, // Error register

    // DWORD 1
    pub lba0: Volatile<u8>, // LBA low register, 7:0
    pub lba1: Volatile<u8>, // LBA mid register, 15:8
    pub lba2: Volatile<u8>, // LBA high register, 23:16
    pub device: Volatile<u8>, // Device register

    // DWORD 2
    pub lba3: Volatile<u8>, // LBA register, 31:24
    pub lba4: Volatile<u8>, // LBA register, 39:32
    pub lba5: Volatile<u8>, // LBA register, 47:40
    pub rsv2: Volatile<u8>, // Reserved

    // DWORD 3
    pub countl: Volatile<u8>, // Count register, 7:0
    pub counth: Volatile<u8>, // Count register, 15:8
    pub rsv3: Volatile<u8>, // Reserved
    pub e_status: Volatile<u8>, // New value of status register

    // DWORD 4
    pub tc: Volatile<u16>, // Transfer count
    pub rsv4: [Volatile<u8>; 2], // Reserved
}

#[repr(C)]
#[derive(Debug)]
pub struct FisDmaSetup {
    // DWORD 0
    pub fis_type: Volatile<u8>, // FIS_TYPE_DMA_SETUP

    pub pm: Volatile<u8>, // Port multiplier, direction: 4 - device to host, interrupt: 2, auto-activate: 1

    pub rsv1: [Volatile<u8>; 2], // Reserved

    // DWORD 1&2
    pub dma_buffer_id: Volatile<u64>, /* DMA Buffer Identifier. Used to Identify DMA buffer in host memory. SATA Spec says host specific and not in Spec. Trying AHCI spec might work. */

    // DWORD 3
    pub rsv3: Volatile<u32>, // More reserved

    // DWORD 4
    pub dma_buffer_offset: Volatile<u32>, // Byte offset into buffer. First 2 bits must be 0

    // DWORD 5
    pub transfer_count: Volatile<u32>, // Number of bytes to transfer. Bit 0 must be 0

    // DWORD 6
    pub rsv6: Volatile<u32>, // Reserved
}
