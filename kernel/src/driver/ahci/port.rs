///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use x86_64::{PhysAddr, VirtAddr};
use crate::driver::ahci::constants::{SataSignature, HbaPortPowerState, HbaPortDeviceDetectState, COMMAND_TABLE_SIZE};
use core::ptr;
use crate::driver::ahci::{CommandHeader, CommandTable, command_table_addr, command_list_addr, received_fis_addr, command_header_addr};
use crate::util::MemoryWrite;
use crate::PHYS_MEM_OFFSET;


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

#[derive(Clone, Copy, Debug)]
pub struct HbaPort {
    pub port_num: u32,
    pub port_mem_base_addr: PhysAddr,
    pub working_mem_base_addr: PhysAddr,
    pub cmd_list_addr: PhysAddr,
    pub fis_base_addr: PhysAddr,
    associated_device: (), // TODO
}
impl HbaPort {
    /// Creates a new instance of HbaPort
    ///
    /// ## Unsafety
    ///
    /// Caller must ensure all addresses provided are valid and correct:
    /// * port_mem_base_addr points to the memory-mapped HBA I/O registers for this port.
    /// * cmd_list_addr is 1k-byte aligned, and points to a free region at least 1k bytes in size.
    /// * fis_base_addr is 256-byte aligned, and points to a free region at least 256 bytes in size.
    pub unsafe fn new(port_num: u32, port_mem_base_addr: PhysAddr, working_mem_base_addr: PhysAddr) -> Self {
        let cmd_list_addr = command_list_addr(working_mem_base_addr, port_num);
        let fis_base_addr = received_fis_addr(working_mem_base_addr, port_num);
        let self_base_virt = port_mem_base_addr.as_u64() + PHYS_MEM_OFFSET;

        let cmd_list_addr_lower = self_base_virt;
        let cmd_list_addr_upper = self_base_virt + 4;
        let fis_base_addr_lower = self_base_virt + 8;
        let fis_base_addr_upper = self_base_virt + 12;

        unsafe {
            ptr::write_volatile(cmd_list_addr_lower as *mut u32, (cmd_list_addr.as_u64() & 0xFFFFFFFF) as u32);
            ptr::write_volatile(cmd_list_addr_upper as *mut u32, ((cmd_list_addr.as_u64() >> 32) & 0xFFFFFFFF) as u32);
            ptr::write_volatile(fis_base_addr_lower as *mut u32, (fis_base_addr.as_u64() & 0xFFFFFFFF) as u32);
            ptr::write_volatile(fis_base_addr_upper as *mut u32, ((fis_base_addr.as_u64() >> 32) & 0xFFFFFFFF) as u32);
        }

        Self {
            port_num, port_mem_base_addr, working_mem_base_addr, cmd_list_addr, fis_base_addr,
            associated_device: (),
        }
    }

    /// Find a free command list slot
    pub unsafe fn find_command_slot(&self) -> Option<u32> {
        // If not set in SACT and CI, the slot is free
        let mut slots = unsafe { self.read_sata_control() | self.read_command_issue() };
        for i in 0..32 {
            if (slots & 1) == 0 {
                return Some(i);
            }
            slots >>= 1;
        }
        None
    }

    pub unsafe fn submit_command(&mut self, slot: u32, mut header: CommandHeader, command: CommandTable) {
        // Spin lock timeout counter
        // TODO: replace with proper timeout
        let mut spin = 0;

        let cmd_table_addr = match header.command_table_addr {
            Some(addr) => addr,
            None => {
                let addr = command_table_addr(self.working_mem_base_addr, self.port_num, slot);
                header.command_table_addr = Some(addr);
                addr
            }
        };
        // Zero out command table memory
        let command_table_mem = unsafe { &mut *((cmd_table_addr.as_u64() + PHYS_MEM_OFFSET) as *mut [u8; COMMAND_TABLE_SIZE as usize]) };
        for i in 0..COMMAND_TABLE_SIZE {
            command_table_mem[i as usize] = 0;
        }

        // The below loop waits until the port is no longer busy before issuing a new command
        let mut tfd = unsafe { self.task_file_data() };
        while (tfd.busy || tfd.drq) && spin < (1<<31) {
            spin += 1;
            tfd = unsafe { self.task_file_data() };
        }
        if spin == (1<<31) {
            panic!("Port is hung");
        }

        unsafe {
            // write command table and header
            command.write_to_addr(VirtAddr::new(cmd_table_addr.as_u64() + PHYS_MEM_OFFSET));
            header.write_to_addr(VirtAddr::new(
                command_header_addr(self.working_mem_base_addr, self.port_num, slot).as_u64()
                    + PHYS_MEM_OFFSET
            ));

            self.set_command_issue_bit(slot as u8, true);
        }

        // Wait for completion
        loop {
            // In some longer duration reads, it may be helpful to spin on the DPS bit
            // in the PxIS port field as well (1 << 5)
            unsafe {
                if !self.get_command_issue_bit(slot as u8) { break; }

                if self.interrupt_status(HbaPortInterruptType::TaskFileError) {
                    panic!("Read disk error\n");
                }
            }
        }

        // Check again
        if unsafe { self.interrupt_status(HbaPortInterruptType::TaskFileError) } {
            panic!("Read disk error");
        }
    }

    /// Read the value of the Interrupt Status register for this port
    pub unsafe fn interrupt_status_register(&self) -> u32 {
        let addr = self.port_mem_base_addr.as_u64() + 0x10 + PHYS_MEM_OFFSET;
        unsafe { ptr::read_volatile(addr as *const u32) }
    }
    /// Retrieve the status of the given interrupt type from this port.
    /// `true` means the interrupt is enabled.
    pub unsafe fn interrupt_status(&self, int: HbaPortInterruptType) -> bool {
        let reg = unsafe { self.interrupt_status_register() };
        ((reg >> int as u32) & 1) != 0
    }
    /// Read the value of the Interrupt Enable register for this port
    pub unsafe fn interrupt_enable_register(&self) -> u32 {
        let addr = self.port_mem_base_addr.as_u64() + 0x14 + PHYS_MEM_OFFSET;
        unsafe { ptr::read_volatile(addr as *const u32) }
    }
    /// Enable the given interrupt type on this port.
    pub unsafe fn enable_interrupt(&self, int: HbaPortInterruptType) {
        let reg = unsafe { self.interrupt_enable_register() };
        let addr = self.port_mem_base_addr.as_u64() + 0x14 + PHYS_MEM_OFFSET;
        unsafe { ptr::write_volatile(addr as *mut u32, reg | (1 << int as u32)) };
    }
    /// Disable the given interrupt type on this port.
    pub unsafe fn disable_interrupt(&self, int: HbaPortInterruptType) {
        let reg = unsafe { self.interrupt_enable_register() };
        let addr = self.port_mem_base_addr.as_u64() + 0x14 + PHYS_MEM_OFFSET;
        unsafe { ptr::write_volatile( addr as *mut u32, reg & (!(1 << int as u32)) ) };
    }
    /// Read the value of the Command/Status register for this port
    pub unsafe fn command_and_status(&self) -> u32 {
        let addr = self.port_mem_base_addr.as_u64() + 0x18 + PHYS_MEM_OFFSET;
        unsafe { ptr::read_volatile(addr as *const u32) }
    }
    /// Write a value to the Command/Status register for this port
    pub unsafe fn set_command_and_status(&self, new_val: u32) {
        let addr = self.port_mem_base_addr.as_u64() + 0x18 + PHYS_MEM_OFFSET;
        unsafe { ptr::write_volatile(addr as *mut u32, new_val); }
    }
    /// Read the value of the Task File Data register for this port
    pub unsafe fn task_file_data(&self) -> HbaPortTaskFileData {
        let addr = self.port_mem_base_addr.as_u64() + 0x20 + PHYS_MEM_OFFSET;
        let reg = unsafe { ptr::read_volatile(addr as *const u32) };
        HbaPortTaskFileData::from_u32(reg)
    }
    /// Get the signature of the device attached to this port from the Signature register.
    /// `None` indicates an invalid signature.
    pub unsafe fn signature(&self) -> Option<SataSignature> {
        let addr = self.port_mem_base_addr.as_u64() + 0x24 + PHYS_MEM_OFFSET;
        let reg = unsafe { ptr::read_volatile(addr as *const u32) };
        match reg {
            0x00000101 => Some(SataSignature::ATA),
            0xEB140101 => Some(SataSignature::ATAPI),
            0xC33C0101 => Some(SataSignature::SEMB),
            0x96690101 => Some(SataSignature::PortMult),
            _ => None
        }
    }
    /// Read the value of the SATA Status register for this port
    pub unsafe fn sata_status(&self) -> u32 {
        let addr = self.port_mem_base_addr.as_u64() + 0x28 + PHYS_MEM_OFFSET;
        unsafe { ptr::read_volatile(addr as *const u32) }
    }
    /// Read the value of the SATA Control register for this port
    pub unsafe fn read_sata_control(&self) -> u32 {
        let addr = self.port_mem_base_addr.as_u64() + 0x2C + PHYS_MEM_OFFSET;
        unsafe { ptr::read_volatile(addr as *const u32) }
    }
    /// Write a value to the SATA Control register for this port
    pub unsafe fn write_sata_control(&self, value: u32) {
        let addr = self.port_mem_base_addr.as_u64() + 0x2C + PHYS_MEM_OFFSET;
        unsafe { ptr::write_volatile(addr as *mut u32, value); }
    }
    /// Read the value of the SATA Error register for this port
    pub unsafe fn sata_error(&self) -> u32 {
        let addr = self.port_mem_base_addr.as_u64() + 0x30 + PHYS_MEM_OFFSET;
        unsafe { ptr::read_volatile(addr as *const u32) }
    }
    /// Read the value of the SATA Active register for this port
    pub unsafe fn read_sata_active(&self) -> u32 {
        let addr = self.port_mem_base_addr.as_u64() + 0x34 + PHYS_MEM_OFFSET;
        unsafe { ptr::read_volatile(addr as *const u32) }
    }
    /// Get the status of the given bit in the SATA Active register
    pub unsafe fn get_sata_active_bit(&self, bit: u8) -> bool {
        let reg = unsafe { self.read_sata_active() };
        ((reg >> bit as u32) & 1) != 0
    }
    /// Set the status of the given bit in the SATA Active register
    pub unsafe fn set_sata_active(&self, bit: u8, value: bool) {
        // retrieve all bits except the one we wish to set
        let reg = unsafe { self.read_sata_active() & (!(1 << bit as u32)) };
        let new_val = reg | (match value { true => (1 << bit as u32), false => 0 });
        let addr = self.port_mem_base_addr.as_u64() + 0x34 + PHYS_MEM_OFFSET;
        unsafe { ptr::write_volatile(addr as *mut u32, new_val) };
    }
    pub unsafe fn read_command_issue(&self) -> u32 {
        let addr = self.port_mem_base_addr.as_u64() + 0x38 + PHYS_MEM_OFFSET;
        unsafe { ptr::read_volatile(addr as *const u32) }
    }
    /// Get the status of the given bit in the Command Issue register
    pub unsafe fn get_command_issue_bit(&self, bit: u8) -> bool {
        let reg = unsafe { self.read_command_issue() };
        ((reg >> bit as u32) & 1) != 0
    }
    /// Set the status of the given bit in the Command Issue register
    pub unsafe fn set_command_issue_bit(&self, bit: u8, value: bool) {
        // retrieve all bits except the one we wish to set
        let reg = unsafe { self.read_command_issue() & (!(1 << bit as u32)) };
        let new_val = reg | (match value { true => (1 << bit as u32), false => 0 });
        let addr = self.port_mem_base_addr.as_u64() + 0x38 + PHYS_MEM_OFFSET;
        unsafe { ptr::write_volatile(addr as *mut u32, new_val); }
    }
    pub unsafe fn power_state(&self) -> HbaPortPowerState {
        match ((unsafe { self.sata_status() } >> 8) & 0x0F) as u8 {
            0 => HbaPortPowerState::Unknown,
            1 => HbaPortPowerState::Active,
            2 => HbaPortPowerState::PartialPowerManagement,
            6 => HbaPortPowerState::Slumber,
            8 => HbaPortPowerState::DevSleep,
            _ => HbaPortPowerState::ReservedValue
        }
    }
    pub unsafe fn device_detect(&self) -> HbaPortDeviceDetectState {
        match (unsafe { self.sata_status() } & 0x0F) as u8 {
            0 => HbaPortDeviceDetectState::Unknown,
            1 => HbaPortDeviceDetectState::PresentNoComm,
            3 => HbaPortDeviceDetectState::PresentWithComm,
            4 => HbaPortDeviceDetectState::PhyOffline,
            _ => HbaPortDeviceDetectState::ReservedValue,
        }
    }
}

