///////////////////////////////////////////////////////////////////////////////L
///////////////////////////////////////////////////////////////////////////////L

use volatile::Volatile;
use x86_64::{PhysAddr, VirtAddr};
use alloc::vec::Vec;
use super::port::HbaPortMemory;
use crate::util::{MemoryRead, MemoryWrite};
use crate::driver::ahci::fis::Fis;
use crate::PHYS_MEM_OFFSET;

pub const HBA_PORT_IS_ERR: u32 = 1 << 30 | 1 << 29 | 1 << 28 | 1 << 27;
pub const HBA_SSTS_PRESENT: u32 = 0x3;

#[repr(C)]
#[derive(Debug)]
/// Representation of global HBA memory.
pub struct HbaMemory {
    /// 0x00, Host capability
    pub host_capability: Volatile<u32>,
    /// 0x04, Global host control
    pub global_host_control: Volatile<u32>,
    /// 0x08, Interrupt status
    pub interrupt_status: Volatile<u32>,
    /// 0x0C, Port implemented
    pub port_implemented: Volatile<u32>,
    /// 0x10, Version
    pub version: Volatile<u32>,
    /// 0x14, Command completion coalescing control
    pub ccc_control: Volatile<u32>,
    /// 0x18, Command completion coalescing ports
    pub ccc_ports: Volatile<u32>,
    /// 0x1C, Enclosure management location
    pub em_location: Volatile<u32>,
    /// 0x20, Enclosure management control
    pub em_control: Volatile<u32>,
    /// 0x24, Host capabilities extended
    pub host_capabilities_ext: Volatile<u32>,
    /// 0x28, BIOS/OS handoff control and status
    pub bios_handoff_control: Volatile<u32>,

    /// 0x2C - 0x9F, Reserved
    pub reserved: [Volatile<u8>; 0x74],
    /// 0xA0 - 0xFF, Vendor specific registers
    pub vendor_registers: [Volatile<u8>; 0x60],

    /// 0x100 - 0x10FF, Port control registers
    pub port_registers:	[HbaPortMemory; 32]
}

impl HbaMemory {
    pub fn init(&mut self) {
        self.global_host_control.write(1 << 31 | 1 << 1);

        crate::serial_println!("AHCI HBA Controller:");
        crate::serial_println!("  Capabilities\t{:b}", self.host_capability.read());
        crate::serial_println!("              \t{:b}", self.host_capabilities_ext.read());
        crate::serial_println!("  Global host control\t{:X}", self.global_host_control.read());
        crate::serial_println!("  Interrupt status\t{:b}", self.interrupt_status.read());
        crate::serial_println!("  Ports implemented\t{:b}", self.port_implemented.read());
        crate::serial_println!("  Version\t\t{:X}", self.version.read());
        crate::serial_println!("  BIOS/OS handoff ctrl\t{:X}", self.bios_handoff_control.read());
    }
}

/// An entry in the Physical Region Descriptor Table
#[derive(Clone, Copy, Debug)]
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
#[derive(Debug)]
pub struct CommandTable {
    pub command_fis: Fis,
    pub atapi_cmd: [u8; 16],
    pub prdt: Vec<PrdEntry>,
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
    /// ## Safety
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
        unsafe { (*target).write(data); }
        // 0x40: ATAPI Command (optional)
        let target = (addr.as_u64() + 0x40) as *mut Volatile<[u8; 16]>;
        unsafe { (*target).write(self.atapi_cmd); }
        // 0x50: Reserved region
        let target = (addr.as_u64() + 0x50) as *mut Volatile<[u8; 48]>;
        unsafe { (*target).write([0u8; 48]); } // zeroes for reserved region
        // 0x80: Physical Region Descriptor Table
        for (i, p) in self.prdt.iter().enumerate() {
            let mut buf = [0u32; 4];
            // data base addr, low half
            buf[0] = (p.data_base_addr.as_u64() & 0xFFFFFFFF) as u32;
            // data base addr, upper half
            buf[1] = ((p.data_base_addr.as_u64() >> 32) & 0xFFFFFFFF) as u32;
            // reserved region
            buf[2] = 0;
            // data byte count and interrupt bit
            buf[3] = (p.data_byte_count & 0x3fffffff) | (match p.interrupt_on_completion { true => 0x80000000, false => 0 });
            // volatile write
            let target = (addr.as_u64() + 0x80 + (16 * i as u64)) as *mut Volatile<[u32; 4]>;
            unsafe { (*target).write(buf); }
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// HBA Command Header
///
/// A Command Header is an entry in a Command List, containing a reference to a Command Table
/// along with some metadata.
// TODO: replace this with a memory-transparent wrapper and utility functions to avoid using extra memory
pub struct CommandHeader {
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
    pub command_table_addr: PhysAddr,
}
impl MemoryWrite for CommandHeader {
    unsafe fn write_to_addr(&self, address: VirtAddr) {
        // 0x00
        // [7]: prefetchable
        // [6]: Write, 1: H2D, 0: D2H
        // [5]: ATAPI
        // [4-0]: Command FIS length in DWORDS, 2 ~ 16
        let target = address.as_u64() as *mut Volatile<u8>;
        unsafe {
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
        }

        // 0x01
        // [7-4]: Port multiplier port
        // [3]: Reserved
        // [2]: Clear busy upon R_OK
        // [1]: Built-In Self Test
        // [0]: Reset
        let target = (address.as_u64() + 0x01) as *mut Volatile<u8>;
        unsafe {
            (*target).write(
                (self.port_mult << 4)
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
        }

        // 0x02-0x03: PRDT Length (in entries)
        let target = (address.as_u64() + 0x02) as *mut Volatile<u16>;
        unsafe { (*target).write(self.prdt_len); }
        // 0x04-0x07: PDRT bytes transferred (software should set to zero)
        let target = (address.as_u64() + 0x04) as *mut Volatile<u32>;
        unsafe { (*target).write(self.prdt_bytes_transferred); }
        // 0x08-0x0B: Command table base address, lower half
        let target = (address.as_u64() + 0x08) as *mut Volatile<u32>;
        // must be 128-byte aligned
        unsafe { (*target).write((self.command_table_addr.as_u64() & 0xFFFFFF80) as u32); }
        // 0x0C-0x0F: Command table base address, upper half
        let target = (address.as_u64() + 0x0C) as *mut Volatile<u32>;
        unsafe { (*target).write(((self.command_table_addr.as_u64() >> 32) & 0xFFFFFFFF) as u32); }
        // 0x10-0x1F: Reserved
        let target = (address.as_u64() + 0x10) as *mut Volatile<[u32; 4]>;
        unsafe { (*target).write([0u32; 4]); }
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
        let byte0 = unsafe { (*target).read() };
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
        let byte1 = unsafe { (*target).read() };
        let port_mult = byte1 & 0b11110000;
        let should_clear_busy_on_ok = (byte1 & 0x4) != 0;
        let bist_bit = (byte1 & 0x2) != 0;
        let reset_bit = (byte1 & 0x1) != 0;
        // 0x02-0x03: PRDT Length (in entries)
        let target = (address.as_u64() + 0x02) as *mut Volatile<u16>;
        let prdt_len = unsafe { (*target).read() };
        // 0x04-0x07: PDRT bytes transferred (software should set to zero)
        let target = (address.as_u64() + 0x04) as *mut Volatile<u32>;
        let prdt_bytes_transferred = unsafe { (*target).read() };
        // 0x08-0x0B: Command table base address, lower half
        let target = (address.as_u64() + 0x08) as *mut Volatile<u32>;
        let addr_low = unsafe { (*target).read() };
        // 0x0C-0x0F: Command table base address, upper half
        let target = (address.as_u64() + 0x0C) as *mut Volatile<u32>;
        let addr_high = unsafe { (*target).read() };
        let command_table_addr = PhysAddr::new(addr_low as u64 + ((addr_high as u64) << 32));
        // 0x10-0x1F: Reserved

        Self {
            fis_length, is_atapi, host_to_device, prefetchable, reset_bit,
            bist_bit, should_clear_busy_on_ok, port_mult, prdt_len, prdt_bytes_transferred,
            command_table_addr
        }
    }
}


/// HBA Command List.
///
/// There's one command list per port, and each one can hold up to 32 Command Headers.
#[derive(Debug)]
pub struct HbaCommandList {
    base_address_phys: PhysAddr,
    base_address_virt: VirtAddr,
    headers: [Option<CommandHeader>; 32],
}
impl HbaCommandList {
    pub fn new(base_addr: VirtAddr) -> Self {
        Self {
            base_address_phys: PhysAddr::new(base_addr.as_u64() - PHYS_MEM_OFFSET),
            base_address_virt: base_addr,
            headers: [None; 32],
        }
    }
}