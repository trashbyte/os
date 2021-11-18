///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

//! ## Limitations
//!
//! The AHCI specification allows for up to 65535 PRDTs in a command table. This
//! implementation only supports a maximum of 32 PRDTs per command table.
//! See [self::constants::NUM_PRDTS_PER_COMMAND](NUM_PRDTS_PER_COMMAND)

// todo:
// clean up remaining funcs here
// complete all fis types
// add atapi stuffs
// atapi identify
// atapi get capacity
// atapi read

#![allow(dead_code)]
#![allow(non_upper_case_globals)]

pub mod hba;
pub mod fis;
//pub mod atapi;
pub mod constants;
pub mod port;

use crate::driver::ahci::fis::FisRegisterHostToDevice;
use crate::PHYS_MEM_OFFSET;

use core::ops::Range;
use x86_64::{VirtAddr, PhysAddr};
use crate::driver::ahci::constants::*;
use crate::util::MemoryRead;
use crate::driver::ahci::port::{HbaPort, HbaPortMemory};
use crate::driver::ahci::hba::{HbaMemory, CommandTable, CommandHeader, PrdEntry};
use alloc::boxed::Box;


// Main AHCI Driver type ///////////////////////////////////////////////////////

#[derive(Debug)]
pub struct AhciDriver {
    pub hba_memory: &'static mut HbaMemory,
    pub ahci_mem_range: Range<u64>,
    pub ports: [Option<Box<HbaPort>>; 32],
}
impl AhciDriver {
    pub unsafe fn new(hba_memory_addr: PhysAddr, ahci_mem_range: Range<u64>) -> Self {
        let hba_memory = unsafe { &mut *((hba_memory_addr.as_u64() + PHYS_MEM_OFFSET) as *mut HbaMemory) };
        hba_memory.init();
        let mut ports = [None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None];
        let ports_implemented = hba_memory.port_implemented.read();
        for i in 0..32 {
            if ((ports_implemented >> i) & 1) != 0 {
                unsafe {
                    let mut port_base = PhysAddr::new(&hba_memory.port_registers[i] as *const _ as u64);
                    let port_mem_base = PhysAddr::new(ahci_mem_range.start + i as u64 * PORT_MEMORY_SIZE);
                    let mut p = Box::new(HbaPort {
                        hba: &mut *(&mut port_base as *mut _ as *mut HbaPortMemory),
                        port_num: i as u32,
                        port_mem_base,
                    });
                    p.init();
                    ports[i] = Some(p);
                };
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
        while (self.hba_memory.global_host_control.read() & AhciGlobalHostControlBit::HbaReset as u32) != 0 && spin < (1<<15) {
            spin += 1;
        }
        if spin == (1<<15) {
            panic!("Timed out while resetting the HBA");
        }
    }
}

/// ## Safety
/// Caller must ensure all parameters are valid
pub unsafe fn test_read(port: &mut HbaPort, start_lba_addr: u64, count: u16, buf: *mut u16) -> Result<(), ()> {
    // find free slot
    let slot = match port.find_command_slot() {
        None => panic!("Cannot find free command list slot"),
        Some(slot) => slot
    };

    // command FIS
    let mut cmd_fis = FisRegisterHostToDevice::new(start_lba_addr);
    cmd_fis.is_from_command = true;
    cmd_fis.port_mult_port = 0;
    cmd_fis.command = AtaCommand::Identify;
    cmd_fis.device = 0;//1 << 6; // LBA mode
    cmd_fis.count = 1;

    // command table (contains FIS and PRDT)
    let mut cmd_table = CommandTable::new(cmd_fis.into());

    // command header (points to table)
    let header_addr = port.port_mem_base.as_u64() + COMMAND_HEADER_SIZE * slot as u64;
    let mut header = unsafe {
        CommandHeader::read_from_addr(VirtAddr::new(header_addr + PHYS_MEM_OFFSET))
    };
    header.fis_length = 5;
    header.prdt_len = 1;
    header.host_to_device = true;
    header.port_mult = 0;
    cmd_table.prdt.push(PrdEntry::new(PhysAddr::new(buf as u64), 512, true));

    unsafe { port.submit_command(slot, header, cmd_table); }
    Ok(())
}

