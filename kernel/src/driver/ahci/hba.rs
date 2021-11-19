///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use core::mem::size_of;

use super::fis::{FisType, FisRegH2D};
use super::constants::*;
use crate::PHYS_MEM_OFFSET;
use crate::memory::AHCI_MEM_REGION;
use volatile::Volatile;
use alloc::string::String;
use x86_64::VirtAddr;
use alloc::boxed::Box;

#[repr(C)]
#[derive(Debug)]
/// The memory layout for a set of per-port registers, memory mapped to the HBA
pub struct HbaPort {
    /// 0x00, command list base address, 1K-byte aligned
    pub cmd_list_base_addr: [Volatile<u32>; 2],
    /// 0x08, FIS base address, 256-byte aligned
    pub fis_base_addr: [Volatile<u32>; 2],
    /// 0x10, interrupt status
    pub interrupt_status: Volatile<u32>,
    /// 0x14, interrupt enable
    pub interrupt_enable: Volatile<u32>,
    /// 0x18, command and status
    pub command_and_status: Volatile<u32>,
    /// 0x1C, Reserved
    pub _reserved1: Volatile<u32>,
    /// 0x20, task file data
    pub task_file_data: Volatile<u32>,
    /// 0x24, signature
    pub signature: Volatile<u32>,
    /// 0x28, SATA status (SCR0:SStatus)
    pub sata_status: Volatile<u32>,
    /// 0x2C, SATA control (SCR2:SControl)
    pub sata_control: Volatile<u32>,
    /// 0x30, SATA error (SCR1:SError)
    pub sata_error: Volatile<u32>,
    /// 0x34, SATA active (SCR3:SActive)
    pub sata_active: Volatile<u32>,
    /// 0x38, command issue
    pub command_issue: Volatile<u32>,
    /// 0x3C, SATA notification (SCR4:SNotification)
    pub sata_notif: Volatile<u32>,
    /// 0x40, FIS-based switch control
    pub fis_switch_ctrl: Volatile<u32>,
    /// 0x44 ~ 0x6F, Reserved
    pub _reserved2: [Volatile<u32>; 11],
    /// 0x70 ~ 0x7F, vendor specific
    pub vendor: [Volatile<u32>; 4],
}

impl HbaPort {
    /// Attempt to detect the type of device present on this port, if any
    pub fn probe(&self) -> HbaPortType {
        if self.sata_status.read() & HBA_SSTS_PRESENT != 0 {
            HbaPortType::from_signature(self.signature.read())
        } else {
            HbaPortType::None
        }
    }

    /// Start the command engine on this port
    pub fn start(&mut self) {
        while self.command_and_status.read() & HbaPortCmdBit::CmdListRunning.as_u32() != 0 {
            // TODO: async wait
        }

        let old = self.command_and_status.read();
        self.command_and_status.write(old | (HbaPortCmdBit::FisReceiveEnable | HbaPortCmdBit::Start).as_u32());
    }

    /// Stop the command engine on this port
    pub fn stop(&mut self) {
        let old = self.command_and_status.read();
        self.command_and_status.write(old & !(HbaPortCmdBit::FisReceiveEnable | HbaPortCmdBit::Start).as_u32());

        while self.command_and_status.read()
            & (HbaPortCmdBit::FisReceiveRunning | HbaPortCmdBit::CmdListRunning).as_u32() != 0
        {
            // TODO: async wait
        }

    }

    /// Finds an unused command slot or returns `None` if all ports are busy
    pub fn slot(&self) -> Option<u32> {
        let slots = self.sata_active.read() | self.command_issue.read();
        for i in 0..32 {
            if slots & 1 << i == 0 {
                return Some(i);
            }
        }
        None
    }

    /// Initialize this port.
    ///
    /// This involves setting the command list and FIS addresses for the port,
    /// as well as assigning command table pointers for each command header.
    pub fn init(&mut self, num: u8) {
        self.stop();

        let all_ports_working_mem_base = AHCI_MEM_REGION.lock().unwrap().range.start_addr();
        let working_mem_base = all_ports_working_mem_base + PORT_MEMORY_SIZE * num as u64;

        let cmd_table_addr = AHCI_MEM_REGION.lock().unwrap().range.start_addr() + COMMAND_LIST_TOTAL_SIZE + RECEIVED_FIS_SIZE + (COMMAND_TABLE_SIZE * num as u64);

        for i in 0..32 {
            let cmd_hdr_addr = VirtAddr::new(working_mem_base + COMMAND_HEADER_SIZE * i + PHYS_MEM_OFFSET);
            let cmdheader = unsafe { &mut *(cmd_hdr_addr.as_u64() as *mut HbaCommandHeader) };
            cmdheader.cmd_table_base_addr.write(cmd_table_addr);
            cmdheader.prdt_length.write(0);
        }

        self.cmd_list_base_addr[0].write(working_mem_base as u32);
        self.cmd_list_base_addr[1].write((working_mem_base >> 32) as u32);
        let fis_base = working_mem_base + COMMAND_LIST_TOTAL_SIZE;
        self.fis_base_addr[0].write(fis_base as u32);
        self.fis_base_addr[1].write((fis_base >> 32) as u32);
        let is = self.interrupt_status.read();
        self.interrupt_status.write(is);
        self.interrupt_enable.write(0 /*TODO: Enable interrupts: 0b10111*/);
        let serr = self.sata_error.read();
        self.sata_error.write(serr);

        // Disable power management
        let sctl = self.sata_control.read() ;
        self.sata_control.write(sctl
            | (HbaPortPwrTransitionDisable::PartialDisable
            |  HbaPortPwrTransitionDisable::SlumberDisable
            |  HbaPortPwrTransitionDisable::DevSleepDisable).bits());

        // Power on and spin up device
        self.command_and_status.write(self.command_and_status.read() | 1 << 2 | 1 << 1);

        crate::serial_println!("   - AHCI init port {} - CMD: {:b}", num, self.command_and_status.read());
    }

    /// Send an ATA identify command to the disk
    pub unsafe fn identify(&mut self) -> Option<u64> {
        unsafe { self.identify_inner(AtaCommand::Identify.as_u8()) }
    }

    /// Send an ATAPI packet identify command to the disk
    pub unsafe fn identify_packet(&mut self) -> Option<u64> {
        unsafe { self.identify_inner(AtaCommand::AtapiIdentifyPacket.as_u8()) }
    }

    // Shared between identify() and identify_packet()
    unsafe fn identify_inner(&mut self, cmd: u8) -> Option<u64> {
        let dest = Box::new([0u16; 256]);

        let slot = self.ata_start(|cmdheader, cmdfis, prdt_entry, _acmd| {
            cmdheader.prdt_length.write(1);

            prdt_entry.data_base_addr.write(dest.as_ref() as *const _ as u64 - PHYS_MEM_OFFSET);
            prdt_entry.data_byte_count.write(512 | 1);

            cmdfis.pm.write(1 << 7);
            cmdfis.command.write(cmd);
            cmdfis.device.write(0);
            cmdfis.countl.write(1);
            cmdfis.counth.write(0);
        })?;

        if self.ata_stop(slot).is_ok() {
            let mut serial = String::new();
            for word in 10..20 {
                let d = dest[word];
                let a = ((d >> 8) as u8) as char;
                if a != '\0' {
                    serial.push(a);
                }
                let b = (d as u8) as char;
                if b != '\0' {
                    serial.push(b);
                }
            }

            let mut firmware = String::new();
            for word in 23..27 {
                let d = dest[word];
                let a = ((d >> 8) as u8) as char;
                if a != '\0' {
                    firmware.push(a);
                }
                let b = (d as u8) as char;
                if b != '\0' {
                    firmware.push(b);
                }
            }

            let mut model = String::new();
            for word in 27..47 {
                let d = dest[word];
                let a = ((d >> 8) as u8) as char;
                if a != '\0' {
                    model.push(a);
                }
                let b = (d as u8) as char;
                if b != '\0' {
                    model.push(b);
                }
            }

            let mut sectors = (dest[100] as u64) |
                              ((dest[101] as u64) << 16) |
                              ((dest[102] as u64) << 32) |
                              ((dest[103] as u64) << 48);

            let lba_bits = if sectors == 0 {
                sectors = (dest[60] as u64) | ((dest[61] as u64) << 16);
                28
            } else {
                48
            };

            crate::serial_println!("   + Serial: '{}' Firmware: '{}' Model: '{}' LBA: {}-bit Capacity: {} MB",
                  serial.trim(), firmware.trim(), model.trim(), lba_bits, sectors / 2048);

            Some(sectors * 512)
        } else {
            None
        }
    }

    /// Begin an ATA DMA transaction
    ///
    /// # Arguments
    ///
    /// * `block` - the starting LBA for the transaction
    /// * `sectors` -  the number of sectors to transfer
    /// * `write` - true -> writing to the device, false -> reading from the device
    /// * `buf` - A reference to the host-side data buffer for sending/receiving data
    pub fn ata_dma(&mut self, block: u64, sectors: usize, write: bool, buf: &mut Box<[u8; 256 * 512]>) -> Option<u32> {
        crate::serial_println!("AHCI DMA - BLOCK: {:X} SECTORS: {} WRITE: {}", block, sectors, write);

        assert!(sectors > 0 && sectors < 256);

        self.ata_start(|cmdheader, cmdfis, prdt_entry, _acmd| {
            if write {
                let cfl = cmdheader.command_fis_length.read();
                cmdheader.command_fis_length.write(cfl | 1 << 7 | 1 << 6)
            }

            cmdheader.prdt_length.write(1);

            prdt_entry.data_base_addr.write(buf as *const _ as u64 - PHYS_MEM_OFFSET);
            prdt_entry.data_byte_count.write(((sectors * 512) as u32) | 1);

            cmdfis.pm.write(1 << 7);
            if write {
                cmdfis.command.write(AtaCommand::WriteDmaExt.as_u8());
            } else {
                cmdfis.command.write(AtaCommand::ReadDmaExt.as_u8());
            }

            cmdfis.lba0.write(block as u8);
            cmdfis.lba1.write((block >> 8) as u8);
            cmdfis.lba2.write((block >> 16) as u8);

            cmdfis.device.write(1 << 6);

            cmdfis.lba3.write((block >> 24) as u8);
            cmdfis.lba4.write((block >> 32) as u8);
            cmdfis.lba5.write((block >> 40) as u8);

            cmdfis.countl.write(sectors as u8);
            cmdfis.counth.write((sectors >> 8) as u8);
        })
    }

    /// Send ATAPI packet
    pub fn atapi_dma(&mut self, cmd: &[u8; 16], size: u32, buf: &mut Box<[u8; 256 * 512]>) -> Result<(), anyhow::Error> {
        let slot = self.ata_start(|cmdheader, cmdfis, prdt_entry, acmd| {
            let cfl = cmdheader.command_fis_length.read();
            cmdheader.command_fis_length.write(cfl | 1 << 5);

            cmdheader.prdt_length.write(1);

            prdt_entry.data_base_addr.write(buf as *mut _ as u64 - PHYS_MEM_OFFSET);
            prdt_entry.data_byte_count.write(size - 1);

            cmdfis.pm.write(1 << 7);
            cmdfis.command.write(AtaCommand::AtapiCmdPacket.as_u8());
            cmdfis.device.write(0);
            cmdfis.lba1.write(0);
            cmdfis.lba2.write(0);
            cmdfis.featurel.write(1);
            cmdfis.featureh.write(0);

            unsafe { core::ptr::write_volatile(acmd.as_mut_ptr() as *mut [u8; 16], *cmd) };
        }).ok_or_else(|| anyhow::anyhow!("ATAPI DMA start failed"))?;
        self.ata_stop(slot)
    }

    pub fn ata_start<F>(&mut self, callback: F) -> Option<u32>
        where F: FnOnce(&mut HbaCommandHeader, &mut FisRegH2D, &mut HbaPrdtEntry, &mut [Volatile<u8>; 16])
    {

        //TODO: Should probably remove
        self.interrupt_status.write(u32::MAX);

        if let Some(slot) = self.slot() {
            {
                let header_addr = AHCI_MEM_REGION.lock().unwrap().range.start_addr() + (COMMAND_HEADER_SIZE * slot as u64) + PHYS_MEM_OFFSET;
                let cmdheader = unsafe { &mut *(header_addr as *mut HbaCommandHeader) };
                cmdheader.command_fis_length.write((size_of::<FisRegH2D>() / size_of::<u32>()) as u8);

                let cmd_tbl_addr = AHCI_MEM_REGION.lock().unwrap().range.start_addr() + COMMAND_LIST_TOTAL_SIZE + RECEIVED_FIS_SIZE + (COMMAND_TABLE_SIZE * slot as u64) + PHYS_MEM_OFFSET;
                let cmdtbl = unsafe { &mut *(cmd_tbl_addr as *mut HbaCommandTable) };
                unsafe { core::ptr::write_bytes(cmdtbl as *mut HbaCommandTable as *mut u8, 0, COMMAND_TABLE_SIZE as usize); }

                let cmdfis = unsafe { &mut *(cmdtbl.command_fis.as_mut_ptr() as *mut FisRegH2D) };
                cmdfis.fis_type.write(FisType::RegH2D as u8);

                let prdt_addr = cmd_tbl_addr + PRDT_OFFSET_IN_TABLE;
                let prdt = unsafe { &mut *(prdt_addr as *mut HbaPrdtEntry) };

                let acmd = unsafe { &mut *(cmdtbl.atapi_command.as_mut_ptr() as *mut [Volatile<u8>; 16]) };

                callback(cmdheader, cmdfis, prdt, acmd)
            }

            while self.task_file_data.read() & (ATA_DEV_BUSY | ATA_DEV_DRQ) as u32 != 0 {
                //unsafe { asm!("pause"); }
            }

            self.command_issue.write(self.command_issue.read() | 1 << slot);

            //TODO: Should probably remove
            self.start();

            Some(slot)
        } else {
            None
        }
    }

    pub fn ata_running(&self, slot: u32) -> bool {
        (self.command_issue.read() & (1 << slot) != 0 || self.task_file_data.read() & 0x80 != 0) && self.interrupt_status.read() & HBA_PORT_IS_ERR == 0
    }

    pub fn ata_stop(&mut self, slot: u32) -> Result<(), anyhow::Error> {
        while self.ata_running(slot) {
            // TODO: async yield
        }

        self.stop();

        if self.interrupt_status.read() & HBA_PORT_IS_ERR != 0 {
            panic!("ERROR IS {:X} IE {:X} CMD {:X} TFD {:X}\nSSTS {:X} SCTL {:X} SERR {:X} SACT {:X}\nCI {:X} SNTF {:X} FBS {:X}",
                   self.interrupt_status.read(), self.interrupt_enable.read(), self.command_and_status.read(), self.task_file_data.read(),
                   self.sata_status.read(), self.sata_control.read(), self.sata_error.read(), self.sata_active.read(),
                   self.command_issue.read(), self.sata_notif.read(), self.fis_switch_ctrl.read());
        } else {
            Ok(())
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct HbaMemory {
    /// 0x00, Host capability
    pub capabilities: Volatile<u32>,
    /// 0x04, Global host control
    pub global_host_control: Volatile<u32>,
    /// 0x08, Interrupt status
    pub interrupt_status: Volatile<u32>,
    /// 0x0C, Port implemented
    pub ports_impl: Volatile<u32>,
    /// 0x10, Version
    pub version: Volatile<u32>,
    /// 0x14, Command completion coalescing control
    pub ccc_control: Volatile<u32>,
    /// 0x18, Command completion coalescing ports
    pub ccc_ports: Volatile<u32>,
    /// 0x1C, Enclosure management location
    pub enclosure_mgmt_loc: Volatile<u32>,
    /// 0x20, Enclosure management control
    pub enclosure_mgmt_ctrl: Volatile<u32>,
    /// 0x24, Host capabilities extended
    pub capabilities_ext: Volatile<u32>,
    /// 0x24, Host capabilities extended
    pub bios_handoff_ctrl: Volatile<u32>,
    /// 0x2C - 0x9F, Reserved
    pub _reserved: [Volatile<u8>; 116],
    /// 0xA0 - 0xFF, Vendor specific registers
    pub vendor: [Volatile<u8>; 96],
    /// 0x100 - 0x10FF, Port control registers
    pub ports: [HbaPort; 32],
}

impl HbaMemory {
    pub fn init(&mut self) {
        self.global_host_control.write((AhciGlobalHostControlBit::AhciEnable | AhciGlobalHostControlBit::InterruptEnable).bits());

        crate::serial_println!("   - AHCI CAP {:X} GHC {:X} IS {:X} PI {:X} VS {:X} CAP2 {:X} BOHC {:X}",
            self.capabilities.read(), self.global_host_control.read(), self.interrupt_status.read(), self.ports_impl.read(),
            self.version.read(), self.capabilities_ext.read(), self.bios_handoff_ctrl.read());
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct HbaPrdtEntry {
    /// Data base address
    data_base_addr: Volatile<u64>,
    /// Reserved
    _reserved: Volatile<u32>,
    /// Byte count, 4M max, interrupt = 1
    data_byte_count: Volatile<u32>,
}

#[repr(C)]
#[derive(Debug)]
pub struct HbaCommandTable {
    /// 0x00 - Command FIS
    command_fis: [Volatile<u8>; 64],

    /// 0x40 - ATAPI command, 12 or 16 bytes
    atapi_command: [Volatile<u8>; 16],

    /// 0x50 - Reserved
    _reserved: [Volatile<u8>; 48],

    /// 0x80 - Physical region descriptor table entries, limited to 0 ~ 32 in this implementation
    prdt_entries: [HbaPrdtEntry; 32],
}

#[repr(C)]
#[derive(Debug)]
pub struct HbaCommandHeader {
    // DW0
    /// Command FIS length in DWORDS, 2 ~ 16, atapi: 4, write - host to device: 2, prefetchable: 1
    command_fis_length: Volatile<u8>,
    /// Reset - 0x80, bist: 0x40, clear busy on ok: 0x20, port multiplier
    port_multipler_port: Volatile<u8>,
    /// Physical region descriptor table length in entries
    prdt_length: Volatile<u16>,

    // DW1
    /// Physical region descriptor byte count transferred
    prd_bytes_transferred: Volatile<u32>,

    // DW2, 3
    /// Command table descriptor base address
    cmd_table_base_addr: Volatile<u64>,

    // DW4 - 7
    /// Reserved
    _reserved: [Volatile<u32>; 4],
}
