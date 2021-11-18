///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use x86_64::{PhysAddr, VirtAddr};
use crate::driver::ahci::constants::{SataSignature, HbaPortPowerState, HbaPortDeviceDetectState, COMMAND_TABLE_SIZE, COMMAND_HEADER_SIZE, COMMAND_TABLE_LIST_OFFSET};
use crate::driver::ahci::{CommandHeader, CommandTable};
use crate::util::{MemoryWrite, MemoryRead};
use crate::PHYS_MEM_OFFSET;
use volatile::Volatile;
use super::hba::HBA_SSTS_PRESENT;


bitflags::bitflags! {
    pub struct HbaPortCommandBit: u32 {
        const COMMAND_RUNNING = 1 << 15;
        const FIS_RECEIVE_RUNNING = 1 << 14;
        const FIS_RECEIVE_ENABLE = 1 << 4;
        const POWER_ON_DEVICE = 1 << 2;
        const SPIN_UP_DEVICE = 1 << 1;
        const START = 1;
        //const ABC = Self::A.bits | Self::B.bits | Self::C.bits;
    }
}


#[derive(Debug, Copy, Clone)]
pub enum HbaPortType {
    None,
    Unknown(u32),
    SATA,
    SATAPI,
    PM,
    SEMB,
}


#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum HbaPortInterruptType {
    DeviceToHostRegisterFIS = 0,
    PIOSetupFIS = 1,
    DMASetupFIS = 2,
    SetDeviceBits = 3,
    UnknownFIS = 4,
    DescriptorProcessed = 5,
    PortConnectChange = 6,
    DeviceMechanicalPresense = 7,

    PhyRdyChange = 22,
    IncorrectPortMultiplier = 23,
    Overflow = 24,

    InterfaceNonFatalError = 26,
    InterfaceFatalError = 27,
    HostBusDataError = 28,
    HostBusFatalError = 29,
    TaskFileError = 30,
    ColdPortDetect = 31,
}

#[derive(Clone, Copy, Debug)]
pub struct HbaPortTaskFileData {
    pub error_register: u8,
    pub busy: bool,
    pub drq: bool,
    pub err: bool,
}
impl HbaPortTaskFileData {
    pub fn from_u32(val: u32) -> Self {
        Self {
            error_register: ((val >> 8) & 0xFF) as u8,
            busy: (val & 0b10000000) != 0,
            drq: (val & 0b00001000) != 0,
            err: (val & 0b00000001) != 0
        }
    }
    pub fn to_u32(&self) -> u32 {
        ((self.error_register as u32) << 8)
            | match self.busy { true => 0b10000000, false => 0 }
            | match self.drq  { true => 0b00001000, false => 0 }
            | match self.err  { true => 0b00000001, false => 0 }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct HbaPortMemory {
    /// 0x00, command list base address, 1K-byte aligned
    pub command_list_base_addr: [Volatile<u32>; 2],
    /// 0x08, FIS base address, 256-byte aligned
    pub fis_base_addr: [Volatile<u32>; 2],
    /// 0x10, interrupt status
    pub interrupt_status: Volatile<u32>,
    /// 0x14, interrupt enable
    pub interrupt_enable: Volatile<u32>,
    /// 0x18, command and status
    pub command_and_status: Volatile<u32>,
    /// 0x1C, Reserved
    pub _reserved0: Volatile<u32>,
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
    pub sata_notification: Volatile<u32>,
    /// 0x40, FIS-based switch control
    pub fis_switch_control: Volatile<u32>,
    /// 0x44 ~ 0x6F, Reserved
    pub _reserved1: [Volatile<u32>; 11],
    /// 0x70 ~ 0x7F, vendor specific
    pub vendor_specific: [Volatile<u32>; 4],
}


#[derive(Debug)]
pub struct HbaPort {
    /// Mapped control registers in the HBA for this port
    pub hba: &'static mut HbaPortMemory,
    pub port_num: u32,
    /// Base physical address for the port's working memory
    /// (the allocated "AHCI memory region" is the working memory for all 32 ports).
    /// It contains, in order:
    /// 32 command lists (1024 bytes each)
    /// Received FIS region (256 bytes)
    /// 32 command tables (4224 bytes each)
    pub port_mem_base: PhysAddr
}
impl HbaPort {
    /// Initializes a port on the HBA
    pub unsafe fn init(&mut self) {
        self.stop();

        // set up command header pointers
        for i in 0..32 {
            let addr = self.command_header_address(i);
            let virt_addr = VirtAddr::new(addr.as_u64() + PHYS_MEM_OFFSET);
            let mut header = unsafe { CommandHeader::read_from_addr(virt_addr) };
            header.command_table_addr = PhysAddr::new(
                self.port_mem_base.as_u64() + COMMAND_TABLE_LIST_OFFSET + COMMAND_TABLE_SIZE * i as u64);
            header.prdt_len = 0;
            unsafe { header.write_to_addr(virt_addr); }
        }

        // set pointers to command list and received FIS base
        self.hba.command_list_base_addr[0].write(self.port_mem_base.as_u64() as u32);
        self.hba.command_list_base_addr[1].write((self.port_mem_base.as_u64() >> 32) as u32);
        let fis_addr = self.port_mem_base.as_u64() + COMMAND_HEADER_SIZE * 32;
        self.hba.fis_base_addr[0].write(fis_addr as u32);
        self.hba.fis_base_addr[1].write((fis_addr >> 32) as u32);

        // i don't know why we do this but apparently it's important
        let int_status = self.hba.interrupt_status.read();
        self.hba.interrupt_status.write(int_status);
        self.hba.interrupt_enable.write(0 /*TODO: Enable interrupts: 0b10111*/);
        let serr = self.hba.sata_error.read();
        self.hba.sata_error.write(serr);

        // Disable power management
        let sctl = self.hba.sata_control.read();
        self.hba.sata_control.write(sctl | 7 << 8);

        // Power on and spin up device
        self.set_command_bits(HbaPortCommandBit::POWER_ON_DEVICE | HbaPortCommandBit::SPIN_UP_DEVICE);

        crate::serial_println!("   - AHCI port {} init {:b}", self.port_num, self.hba.command_and_status.read());
    }

    pub fn command_header_address(&mut self, port_num: usize) -> PhysAddr {
        PhysAddr::new(self.port_mem_base.as_u64() + (port_num as u64 * COMMAND_HEADER_SIZE))
    }

    pub fn probe(&self) -> HbaPortType {
        const HBA_SIG_ATA: u32 = 0x00000101;
        const HBA_SIG_ATAPI: u32 = 0xEB140101;
        const HBA_SIG_PM: u32 = 0x96690101;
        const HBA_SIG_SEMB: u32 = 0xC33C0101;

        if self.hba.sata_status.read() & HBA_SSTS_PRESENT == HBA_SSTS_PRESENT {
            let sig = self.hba.signature.read();
            match sig {
                HBA_SIG_ATA => HbaPortType::SATA,
                HBA_SIG_ATAPI => HbaPortType::SATAPI,
                HBA_SIG_PM => HbaPortType::PM,
                HBA_SIG_SEMB => HbaPortType::SEMB,
                _ => HbaPortType::Unknown(sig),
            }
        } else {
            HbaPortType::None
        }
    }

    /// Start command engine on this port
    pub fn start(&mut self) {
        while self.test_command_bit(HbaPortCommandBit::COMMAND_RUNNING) {
            // TODO: async yield
        }

        self.set_command_bits(HbaPortCommandBit::START | HbaPortCommandBit::FIS_RECEIVE_ENABLE);
    }

    /// Stop command engine on this port
    pub fn stop(&mut self) {
        self.clear_command_bits(HbaPortCommandBit::START| HbaPortCommandBit::FIS_RECEIVE_ENABLE);

        while self.test_command_bit(
            HbaPortCommandBit::COMMAND_RUNNING | HbaPortCommandBit::FIS_RECEIVE_RUNNING
        ) {
            // TODO: async yield
        }

    }

    pub fn test_command_bit(&self, bit: HbaPortCommandBit) -> bool {
        self.hba.command_and_status.read() & bit.bits != 0
    }

    pub fn set_command_bits(&mut self, bits: HbaPortCommandBit) {
        let old = self.hba.command_and_status.read();
        self.hba.command_and_status.write(old | bits.bits);
    }

    pub fn clear_command_bits(&mut self, bits: HbaPortCommandBit) {
        let old = self.hba.command_and_status.read();
        self.hba.command_and_status.write(old & !bits.bits);
    }

    /// Find a free command list slot
    pub fn find_command_slot(&self) -> Option<u8> {
        // If not set in SACT and CI, the slot is free
        let mut slots = self.read_sata_control() | self.read_command_issue();
        for i in 0..32u8 {
            if (slots & 1) == 0 {
                return Some(i);
            }
            slots >>= 1;
        }
        None
    }

    pub unsafe fn submit_command(&mut self, slot: u8, header: CommandHeader, command: CommandTable) {
        // Spin lock timeout counter
        // TODO: replace with proper timeout
        let mut spin = 0;

        // The below loop waits until the port is no longer busy before issuing a new command
        let mut tfd = self.task_file_data();
        while (tfd.busy || tfd.drq) && spin < (1<<31) {
            spin += 1;
            tfd = self.task_file_data();
        }
        if spin == (1<<31) {
            panic!("Port is hung");
        }

        crate::serial_println!("{:?}\n\n{:?}", header, command);

        // write command table
        unsafe {
            header.write_to_addr(VirtAddr::new(self.port_mem_base.as_u64() + (COMMAND_HEADER_SIZE * slot as u64) + PHYS_MEM_OFFSET));
            command.write_to_addr(VirtAddr::new(header.command_table_addr.as_u64() + PHYS_MEM_OFFSET));
        }
        self.set_command_issue_bit(slot, true);
        crate::serial_println!("slot {}", slot);
        self.start();

        // Wait for completion
        loop {
            crate::serial_println!("loop");
            // In some longer duration reads, it may be helpful to spin on the DPS bit
            // in the PxIS port field as well (1 << 5)
            if !self.get_command_issue_bit(slot) { break; }

            if self.interrupt_status_register() & (1<<30 | 1<<29 | 1<<28 | 1<<27 | 1<<24 | 1<<23) != 0 {
                panic!("Read disk error\n");
            }
        }

        // Check again
        if self.interrupt_status(HbaPortInterruptType::TaskFileError) {
            crate::serial_println!("Read disk error");
        }
    }

    /// Read the value of the Interrupt Status register for this port
    pub fn interrupt_status_register(&self) -> u32 {
        self.hba.interrupt_status.read()
    }
    /// Retrieve the status of the given interrupt type from this port.
    /// `true` means the interrupt is enabled.
    pub fn interrupt_status(&self, int: HbaPortInterruptType) -> bool {
        let reg = self.hba.interrupt_status.read();
        ((reg >> int as u32) & 1) != 0
    }
    /// Enable the given interrupt type on this port.
    pub fn enable_interrupt(&mut self, int: HbaPortInterruptType) {
        let reg = self.hba.interrupt_enable.read();
        self.hba.interrupt_enable.write(reg | (1 << int as u32));
    }
    /// Disable the given interrupt type on this port.
    pub fn disable_interrupt(&mut self, int: HbaPortInterruptType) {
        let reg = self.hba.interrupt_enable.read();
        self.hba.interrupt_enable.write(reg & !(1 << int as u32));
    }
    /// Read the value of the Command/Status register for this port
    pub fn command_and_status(&self) -> u32 {
        self.hba.command_and_status.read()
    }
    /// Write a value to the Command/Status register for this port
    pub fn set_command_and_status(&mut self, new_val: u32) {
        self.hba.command_and_status.write(new_val)
    }
    /// Read the value of the Task File Data register for this port
    pub fn task_file_data(&self) -> HbaPortTaskFileData {
        HbaPortTaskFileData::from_u32(self.hba.task_file_data.read())
    }
    /// Get the signature of the device attached to this port from the Signature register.
    /// `None` indicates an invalid signature.
    pub fn signature(&self) -> Option<SataSignature> {
        let reg = self.hba.signature.read();
        match reg {
            0x00000101 => Some(SataSignature::ATA),
            0xEB140101 => Some(SataSignature::ATAPI),
            0xC33C0101 => Some(SataSignature::SEMB),
            0x96690101 => Some(SataSignature::PortMult),
            _ => None
        }
    }
    /// Read the value of the SATA Status register for this port
    pub fn sata_status(&self) -> u32 {
        self.hba.sata_status.read()
    }
    /// Read the value of the SATA Control register for this port
    pub fn read_sata_control(&self) -> u32 {
        self.hba.sata_control.read()
    }
    /// Write a value to the SATA Control register for this port
    pub fn write_sata_control(&mut self, value: u32) {
        self.hba.sata_control.write(value);
    }
    /// Read the value of the SATA Error register for this port
    pub fn sata_error(&self) -> u32 {
        self.hba.sata_error.read()
    }
    /// Read the value of the SATA Active register for this port
    pub fn read_sata_active(&self) -> u32 {
        self.hba.sata_active.read()
    }
    /// Get the status of the given bit in the SATA Active register
    pub fn get_sata_active_bit(&self, bit: u8) -> bool {
        let reg = self.read_sata_active();
        ((reg >> bit as u32) & 1) != 0
    }
    /// Set the status of the given bit in the SATA Active register
    pub fn set_sata_active(&mut self, bit: u8, value: bool) {
        // retrieve all bits except the one we wish to set
        let reg = self.read_sata_active() & (!(1 << bit as u32));
        let new_val = reg | (match value { true => (1 << bit as u32), false => 0 });
        self.hba.sata_active.write(new_val);
    }
    pub fn read_command_issue(&self) -> u32 {
        self.hba.command_issue.read()
    }
    /// Get the status of the given bit in the Command Issue register
    pub fn get_command_issue_bit(&self, bit: u8) -> bool {
        let reg = self.hba.command_issue.read();
        ((reg >> bit as u32) & 1) != 0
    }
    /// Set the status of the given bit in the Command Issue register
    pub fn set_command_issue_bit(&mut self, bit: u8, value: bool) {
        // retrieve all bits except the one we wish to set
        let reg = self.read_command_issue() & (!(1 << bit as u32));
        let new_val = reg | (match value { true => (1 << bit as u32), false => 0 });
        self.hba.command_issue.write(new_val);
    }
    pub unsafe fn power_state(&self) -> HbaPortPowerState {
        match ((self.sata_status() >> 8) & 0x0F) as u8 {
            0 => HbaPortPowerState::Unknown,
            1 => HbaPortPowerState::Active,
            2 => HbaPortPowerState::PartialPowerManagement,
            6 => HbaPortPowerState::Slumber,
            8 => HbaPortPowerState::DevSleep,
            _ => HbaPortPowerState::ReservedValue
        }
    }
    pub unsafe fn device_detect(&self) -> HbaPortDeviceDetectState {
        match (self.sata_status() & 0x0F) as u8 {
            0 => HbaPortDeviceDetectState::Unknown,
            1 => HbaPortDeviceDetectState::PresentNoComm,
            3 => HbaPortDeviceDetectState::PresentWithComm,
            4 => HbaPortDeviceDetectState::PhyOffline,
            _ => HbaPortDeviceDetectState::ReservedValue,
        }
    }
}

