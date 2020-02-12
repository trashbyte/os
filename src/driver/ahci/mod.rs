//! ## Limitations
//!
//! The AHCI specification allows for up to 65535 PRDTs in a command table. This
//! implementation only supports a maximum of 256 PRDTs per command table.

// todo:
// clean up remaining funcs here
// complete all fis types
// add atapi stuffs
// atapi identify
// atapi get capacity
// atapi read

#![allow(dead_code)]
#![allow(non_upper_case_globals)]

pub mod fis;
pub mod atapi;
pub mod constants;
pub mod port;

use crate::driver::ahci::fis::{FisRegisterHostToDevice, Fis};
use crate::{phys_mem_offset};

use volatile::Volatile;
use core::ops::Range;
use x86_64::{VirtAddr, PhysAddr};
use alloc::vec::Vec;
use crate::driver::ahci::constants::*;
use crate::util::{MemoryWrite, MemoryRead, MemoryReadWrite};
use crate::driver::ahci::port::{HbaPort, HbaPortInterruptType};
use core::ptr;


// Main AHCI Driver type ///////////////////////////////////////////////////////

pub struct AhciDriver {
    pub hba_memory: &'static mut HbaMemory,
    pub ahci_mem_range: Range<u64>,
    pub ports: [Option<HbaPort>; 32],
}
impl AhciDriver {
    pub unsafe fn new(hba_memory_addr: PhysAddr, ahci_mem_range: Range<u64>) -> Self {
        let hba_memory = &mut *((hba_memory_addr.as_u64() + phys_mem_offset()) as *mut HbaMemory);
        let mut ports = [None; 32];
        let ports_implemented = hba_memory.port_implemented.read();
        for i in 0..32 {
            if ((ports_implemented >> i) & 1) != 0 {
                let p = init_port(i as u32, hba_memory_addr, ahci_mem_range.clone());
                p.enable_interrupt(HbaPortInterruptType::DeviceToHostRegisterFIS);
                p.enable_interrupt(HbaPortInterruptType::PIOSetupFIS);
                p.enable_interrupt(HbaPortInterruptType::DMASetupFIS);
                p.enable_interrupt(HbaPortInterruptType::SetDeviceBits);
                p.enable_interrupt(HbaPortInterruptType::UnknownFIS);
                p.enable_interrupt(HbaPortInterruptType::DescriptorProcessed);
                p.enable_interrupt(HbaPortInterruptType::PortConnectChange);
                p.enable_interrupt(HbaPortInterruptType::DeviceMechanicalPresense);
                p.enable_interrupt(HbaPortInterruptType::PhyRdyChange);
                p.enable_interrupt(HbaPortInterruptType::InterfaceNonFatalError);
                p.enable_interrupt(HbaPortInterruptType::InterfaceFatalError);
                p.enable_interrupt(HbaPortInterruptType::HostBusDataError);
                p.enable_interrupt(HbaPortInterruptType::HostBusFatalError);
                p.enable_interrupt(HbaPortInterruptType::TaskFileError);
                p.enable_interrupt(HbaPortInterruptType::ColdPortDetect);
                p.set_command_and_status(0b1100000000010001);
                ports[i] = Some(p);
            }
        }
        Self { hba_memory, ahci_mem_range, ports }
    }
    pub fn set_ahci_enable(&mut self, value: bool) {
        let old = self.hba_memory.global_host_control.read() & !(AhciGlobalHostControlBit::AhciEnable as u32);
        self.hba_memory.global_host_control.write(old | match value {
            false => 0, true => AhciGlobalHostControlBit::AhciEnable as u32
        });
    }
    pub fn set_interrupt_enable(&mut self, value: bool) {
        let old = self.hba_memory.global_host_control.read() & !(AhciGlobalHostControlBit::InterruptEnable as u32);
        self.hba_memory.global_host_control.write(old | match value {
            false => 0, true => AhciGlobalHostControlBit::InterruptEnable as u32
        });
    }
    pub fn reset(&mut self) {
        let old = self.hba_memory.global_host_control.read();
        self.hba_memory.global_host_control.write(old | AhciGlobalHostControlBit::HbaReset as u32);
        // TODO: replace with proper timer
        let mut spin = 0;
        while (self.hba_memory.global_host_control.read() & AhciGlobalHostControlBit::HbaReset as u32) != 0 && spin < (1<<31) {
            spin += 1;
        }
        if spin == (1<<31) {
            panic!("failed to reset HBA");
        }
    }
}


// AHCI types //////////////////////////////////////////////////////////////////

/// An entry in the Physical Region Descriptor Table
pub struct PrdEntry {
    pub data_base_addr: PhysAddr,
    pub data_byte_count: u32,
    pub interrupt_on_completion: bool,
}
impl PrdEntry {
    pub fn new(data_base_addr: PhysAddr, data_byte_count: u32, interrupt_on_completion: bool) -> Self {
        Self {
            data_base_addr, data_byte_count, interrupt_on_completion
        }
    }
}

/// An AHCI Command Table
pub struct CommandTable {
    command_fis: Fis,
    atapi_cmd: [u8; 16],
    prdt: Vec<PrdEntry>,
}
impl CommandTable {
    pub fn new(command_fis: Fis) -> Self {
        Self {
            command_fis,
            atapi_cmd: [0u8; 16],
            prdt: Vec::new(),
        }
    }
}
impl MemoryWrite for CommandTable {
    /// Writes the contents of this table to the specified virtual address, formatted
    /// as a AHCI Command Table.
    ///
    /// ## Unsafety
    ///
    /// Caller must ensure that the provided address is valid and properly aligned (1kb align).
    unsafe fn write_to_addr(&self, addr: VirtAddr) {
        // 0x00: Command FIS
        let target = addr.as_u64() as *mut Volatile<[u8; 64]>;
        let mut vec = Vec::new();
        self.command_fis.write_to_buffer(&mut vec);
        vec.resize(64, 0);
        let mut data = [0u8; 64];
        data.copy_from_slice(vec.as_slice());
        (*target).write(data);
        // 0x40: ATAPI Command (optional)
        let target = (addr.as_u64() + 0x40) as *mut Volatile<[u8; 16]>;
        (*target).write(self.atapi_cmd);
        // 0x50: Reserved region
        let target = (addr.as_u64() + 0x50) as *mut Volatile<[u8; 48]>;
        (*target).write([0u8; 48]); // zeroes for reserved region
        // 0x80: Physical Region Descriptor Table
        for (i, p) in self.prdt.iter().enumerate() {
            let target = (addr.as_u64() + 0x80 + (16 * i as u64)) as *mut Volatile<[u32; 4]>;
            let mut buf = [0u32; 4];
            // data base addr, low half
            buf[0] = (p.data_base_addr.as_u64() & 0xFFFFFFFF) as u32;
            // data base addr, upper half
            buf[1] = ((p.data_base_addr.as_u64() >> 32) & 0xFFFFFFFF) as u32;
            // reserved region
            buf[2] = 0;
            // data byte count and interrupt bit
            buf[3] = (p.data_byte_count & 0x3fffff) | (match p.interrupt_on_completion { true => 0x80000000, false => 0 });
            // volatile write
            (*target).write(buf);
        }
    }
}


#[derive(Debug, Clone, Copy)]
/// HBA Command Header
///
/// A Command Header is an entry in a Command List, containing a reference to a Command Table
/// along with some metadata.
pub struct CommandHeader {
    pub slot: Option<u32>,
    pub fis_length: u8,
    pub is_atapi: bool,
    pub host_to_device: bool,
    pub prefetchable: bool,
    pub reset_bit: bool,
    pub bist_bit: bool,
    pub should_clear_busy_on_ok: bool,
    pub port_mult: u8,
    pub prdt_len: u16,
    pub prdt_bytes_transferred: u32,
    pub command_table_addr: Option<PhysAddr>,
}
impl CommandHeader {
    pub fn new(slot: u32) -> Self {
        Self {
            slot: Some(slot),
            fis_length: 0, // TODO: this should probably be inferred automatically
            is_atapi: false,
            host_to_device: true,
            prefetchable: false,
            reset_bit: false,
            bist_bit: false,
            should_clear_busy_on_ok: true,
            port_mult: 0,
            prdt_len: 0,
            prdt_bytes_transferred: 0,
            command_table_addr: None,
        }
    }
}
impl MemoryWrite for CommandHeader {
    unsafe fn write_to_addr(&self, address: VirtAddr) {
        let command_table_addr = self.command_table_addr
            .expect("Tried to write command header to memory with no command table address set");
        // 0x00
        // [7]: prefetchable
        // [6]: Write, 1: H2D, 0: D2H
        // [5]: ATAPI
        // [4-0]: Command FIS length in DWORDS, 2 ~ 16
        let target = address.as_u64() as *mut Volatile<u8>;
        (*target).write(
            (self.fis_length & 0b11111)
                | (match self.is_atapi {
                true => 0x20,
                false => 0
            })
                | (match self.host_to_device {
                true => 0x40,
                false => 0
            })
                | (match self.prefetchable {
                true => 0x80,
                false => 0
            })
        );
        // 0x01
        // [7-4]: Port multiplier port
        // [3]: Reserved
        // [2]: Clear busy upon R_OK
        // [1]: Built-In Self Test
        // [0]: Reset
        let target = (address.as_u64() + 0x01) as *mut Volatile<u8>;
        (*target).write(
            (self.port_mult & 0b11110000)
                | (match self.should_clear_busy_on_ok {
                true => 0x4,
                false => 0
            })
                | (match self.bist_bit {
                true => 0x02,
                false => 0
            })
                | (match self.reset_bit {
                true => 0x01,
                false => 0
            })
        );
        // 0x02-0x03: PRDT Length (in entries)
        let target = (address.as_u64() + 0x02) as *mut Volatile<u16>;
        (*target).write(self.prdt_len);
        // 0x04-0x07: PDRT bytes transferred (software should set to zero)
        let target = (address.as_u64() + 0x04) as *mut Volatile<u32>;
        (*target).write(self.prdt_bytes_transferred);
        // 0x08-0x0B: Command table base address, lower half
        let target = (address.as_u64() + 0x08) as *mut Volatile<u32>;
        (*target).write((command_table_addr.as_u64() & 0xFFFFFFFF) as u32);
        // 0x0C-0x0F: Command table base address, upper half
        let target = (address.as_u64() + 0x0C) as *mut Volatile<u32>;
        (*target).write(((command_table_addr.as_u64() >> 32) & 0xFFFFFFFF) as u32);
        // 0x10-0x1F: Reserved
        let target = (address.as_u64() + 0x10) as *mut Volatile<[u32; 4]>;
        (*target).write([0u32; 4]);
    }
}
impl MemoryRead for CommandHeader {
    unsafe fn read_from_addr(address: VirtAddr) -> Self {
        // 0x00
        // [7]: prefetchable
        // [6]: Write, 1: H2D, 0: D2H
        // [5]: ATAPI
        // [4-0]: Command FIS length in DWORDS, 2 ~ 16
        let target = address.as_u64() as *mut Volatile<u8>;
        let byte0 = (*target).read();
        let fis_length = byte0 & 0b11111;
        let is_atapi = (byte0 & 0x20) != 0;
        let host_to_device = (byte0 & 0x40) != 0;
        let prefetchable = (byte0 & 0x80) != 0;
        // 0x01
        // [7-4]: Port multiplier port
        // [3]: Reserved
        // [2]: Clear busy upon R_OK
        // [1]: Built-In Self Test
        // [0]: Reset
        let target = (address.as_u64() + 0x01) as *mut Volatile<u8>;
        let byte1 = (*target).read();
        let port_mult = byte1 & 0b11110000;
        let should_clear_busy_on_ok = (byte1 & 0x4) != 0;
        let bist_bit = (byte1 & 0x2) != 0;
        let reset_bit = (byte1 & 0x1) != 0;
        // 0x02-0x03: PRDT Length (in entries)
        let target = (address.as_u64() + 0x02) as *mut Volatile<u16>;
        let prdt_len = (*target).read();
        // 0x04-0x07: PDRT bytes transferred (software should set to zero)
        let target = (address.as_u64() + 0x04) as *mut Volatile<u32>;
        let prdt_bytes_transferred = (*target).read();
        // 0x08-0x0B: Command table base address, lower half
        let target = (address.as_u64() + 0x08) as *mut Volatile<u32>;
        let addr_low = (*target).read();
        // 0x0C-0x0F: Command table base address, upper half
        let target = (address.as_u64() + 0x0C) as *mut Volatile<u32>;
        let addr_high = (*target).read();
        let command_table_addr = PhysAddr::new(addr_low as u64 + ((addr_high as u64) << 32));
        // 0x10-0x1F: Reserved

        Self {
            slot: None,
            fis_length, is_atapi, host_to_device, prefetchable, reset_bit,
            bist_bit, should_clear_busy_on_ok, port_mult, prdt_len, prdt_bytes_transferred,
            command_table_addr: Some(command_table_addr)
        }
    }
}
impl MemoryReadWrite for CommandHeader {}


/// HBA Command List.
///
/// There's one command list per port, and each one can hold up to 32 Command Headers.
pub struct HbaCommandList {
    base_address_phys: PhysAddr,
    base_address_virt: VirtAddr,
    headers: [Option<CommandHeader>; 32],
}
impl HbaCommandList {
    pub fn new(base_addr: VirtAddr) -> Self {
        Self {
            base_address_phys: PhysAddr::new(base_addr.as_u64() - phys_mem_offset()),
            base_address_virt: base_addr,
            headers: [None; 32],
        }
    }
}

/// Placeholder for HBA Port MMIO
#[repr(transparent)]
pub struct HbaPortDummy([u8; 128]);

#[repr(C)]
/// Representation of global HBA memory.
pub struct HbaMemory {
    pub host_capability: Volatile<u32>,       // 0x00, Host capability
    pub global_host_control: Volatile<u32>,   // 0x04, Global host control
    pub interrupt_status: Volatile<u32>,      // 0x08, Interrupt status
    pub port_implemented: Volatile<u32>,      // 0x0C, Port implemented
    pub version: Volatile<u32>,               // 0x10, Version
    pub ccc_control: Volatile<u32>,           // 0x14, Command completion coalescing control
    pub ccc_ports: Volatile<u32>,             // 0x18, Command completion coalescing ports
    pub em_location: Volatile<u32>,           // 0x1C, Enclosure management location
    pub em_control: Volatile<u32>,            // 0x20, Enclosure management control
    pub host_capabilities_ext: Volatile<u32>, // 0x24, Host capabilities extended
    pub bios_handoff_control: Volatile<u32>,  // 0x28, BIOS/OS handoff control and status

    pub reserved: [Volatile<u8>; 0x74],         // 0x2C - 0x9F, Reserved
    pub vendor_registers: [Volatile<u8>; 0x60], // 0xA0 - 0xFF, Vendor specific registers

    pub port_registers:	[HbaPortDummy; 32] // 0x100 - 0x10FF, Port control registers
}


// Utility functions ///////////////////////////////////////////////////////////

/// Start command engine
unsafe fn start_cmd(port_base_addr: PhysAddr) {
    let command_and_status_addr = port_base_addr.as_u64() + 0x18 + phys_mem_offset();
    // Wait until CR (bit15) is cleared
    while (ptr::read_volatile(command_and_status_addr as *const u32) & HbaPxCMDBit::CmdListRunning.as_u32()) != 0 {}

    // Set FRE (bit4) and ST (bit0)
    let old = ptr::read_volatile(command_and_status_addr as *const u32);
    ptr::write_volatile(command_and_status_addr as *mut u32, old | HbaPxCMDBit::FisReceiveEnable.as_u32() | HbaPxCMDBit::Start.as_u32());
}

/// Stop command engine
unsafe fn stop_cmd(port_base_addr: PhysAddr) {
    let command_and_status_addr = port_base_addr.as_u64() + 0x18 + phys_mem_offset();
    // Clear ST (bit0) and FRE (bit4)
    let old = ptr::read_volatile(command_and_status_addr as *const u32);
    ptr::write_volatile(command_and_status_addr as *mut u32, old & !(HbaPxCMDBit::Start.as_u32()) & !(HbaPxCMDBit::FisReceiveEnable.as_u32()));

    // Wait until FR (bit14), CR (bit15) are cleared
    loop {
        let value = ptr::read_volatile(command_and_status_addr as *const u32);
        if (value & HbaPxCMDBit::FisReceiveRunning.as_u32()) != 0 { continue; }
        if (value & HbaPxCMDBit::CmdListRunning.as_u32()) != 0 { continue; }
        break;
    }
}

/// ## Unsafety
///
/// Caller must ensure:
/// * `port_num` is a valid, implemented port number
/// * `hba_base_addr` points to the beginning of HBA MMIO
/// * `ahci_mem_range` properly represents the bounds of the AHCI working memory area.
unsafe fn init_port(port_num: u32, hba_base_addr: PhysAddr, ahci_mem_range: Range<u64>) -> HbaPort {
    let port_mem_addr = port_registers_addr(hba_base_addr, port_num);

    // Stop command engine
    stop_cmd(port_mem_addr);

    let port = HbaPort::new(port_num, port_mem_addr, PhysAddr::new(ahci_mem_range.start));

    // Start command engine
    start_cmd(port_mem_addr);

    port
}

fn port_registers_addr(hba_base_addr: PhysAddr, port_num: u32) -> PhysAddr {
    // port registers start at 0x100 relative to the start of HBA memory, and are 128b long each
    PhysAddr::new(hba_base_addr.as_u64() + 0x100 + (128 * port_num as u64))
}

fn command_list_addr(base_addr: PhysAddr, port_num: u32) -> PhysAddr {
    // command lists start at the beginning of the area, and are 1kb each
    PhysAddr::new(base_addr.as_u64() + (1024 * port_num as u64))
}

pub fn command_header_addr(base_addr: PhysAddr, port_num: u32, cmd_number: u32) -> PhysAddr {
    // command headers are what make up the command lists, and there's 32 32-byte headers in each list
    PhysAddr::new(command_list_addr(base_addr, port_num).as_u64() + (32 * cmd_number as u64))
}

fn received_fis_addr(base_addr: PhysAddr, port_num: u32) -> PhysAddr {
    // received fises start at base + 32k and are 256b each
    PhysAddr::new(base_addr.as_u64() + (1024 * 32) + (256 * port_num as u64))
}

fn command_table_addr(base_addr: PhysAddr, port_num: u32, slot: u32) -> PhysAddr {
    // command tables start at base + 40k, and there's 32 tables per port, 4224 bytes each, for 135,168b per port
    PhysAddr::new(base_addr.as_u64() + (1024 * 40) + (32 * COMMAND_TABLE_SIZE * port_num as u64) + (COMMAND_TABLE_SIZE * slot as u64))
}

/// ## Unsafety
/// Caller must ensure all parameters are valid
pub unsafe fn test_read(port: &mut HbaPort, start_lba_addr: u64, count: u16, buf: *mut u16) -> Result<(), ()> {
    // find free slot
    let slot = port.find_command_slot();
    if slot.is_none() {
        panic!("Cannot find free command list entry");
    }
    let slot = slot.unwrap();

    // command FIS
    let mut cmd_fis = FisRegisterHostToDevice::new(start_lba_addr);
    cmd_fis.is_from_command = true;
    cmd_fis.command = AtaCommand::ReadDmaExt;
    cmd_fis.device = 1 << 6; // LBA mode
    cmd_fis.count = count;

    // command table (contains fis and prdt)
    let mut cmd_table = CommandTable::new(Fis::FisRegisterHostToDevice(cmd_fis));

    // command header (points to table)
    let mut header = CommandHeader::new(slot);
    header.fis_length = 5;
    header.prdt_len = (((count as u16) - 1) >> 4) + 1;

    // build PRDT entries (8K bytes (16 sectors) each)
    let mut count = count as i32; // subtraction later can go below zero
    let len = header.prdt_len as u32;
    for i in 0..len {
        let buf_addr = buf as u64 + (4096 * i as u64);
        let byte_count = if i == len-1 {
            (count << 9) - 1 // 512 bytes per sector
        } else {
            8 * 1024 - 1 // 8K bytes (this value should always be set to 1 less than the actual value)
        };
        let prdt = PrdEntry::new(PhysAddr::new(buf_addr), byte_count as u32, true);
        cmd_table.prdt.push(prdt);
        count -= 16; // 16 sectors
    }

    port.submit_command(slot, header, cmd_table);
    Ok(())
}






