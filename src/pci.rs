use x86_64::instructions::port::Port;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter, Error};
use alloc::string::String;
use alloc::format;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq)]
pub enum PciClass {
    Unclassified = 0x00,
    MassStorage = 0x01,
    Network = 0x02,
    Display = 0x03,
    Multimedia = 0x04,
    Memory = 0x05,
    Bridge = 0x06,
    Other = 0xFF,
}
impl PciClass {
    pub fn from_u8(u: u8) -> PciClass {
        let opt = num::FromPrimitive::from_u8(u);
        match opt {
            Some(e) => e,
            None => PciClass::Other
        }
    }
    pub fn as_u8(&self) -> u8 { *self as u8 }
}

#[allow(non_camel_case_types, dead_code)]
#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq)]
pub enum PciFullClass {
    Unclassified_NonVgaCompatible = 0x0000,
    Unclassified_VgaCompatible = 0x0001,

    MassStorage_ScsiBus = 0x0100,
    MassStorage_IDE = 0x0101,
    MassStorage_Floppy = 0x0102,
    MassStorage_IpiBus = 0x0103,
    MassStorage_RAID = 0x0104,
    MassStorage_ATA = 0x0105,
    MassStorage_SATA = 0x0106,
    MassStorage_SerialSCSI = 0x0107,
    MassStorage_NVM = 0x0108,
    MassStorage_Other = 0x0180,

    Network_Ethernet = 0x0200,
    Network_TokenRing = 0x0201,
    Network_FDDI = 0x0202,
    Network_ATM = 0x0203,
    Network_ISDN = 0x0204,
    Network_WorldFlip = 0x0205,
    Network_PICMG = 0x0206,
    Network_Infiniband = 0x0207,
    Network_Fabric = 0x0208,
    Network_Other = 0x0280,

    Display_VGA = 0x0300,
    Display_XGA = 0x0301,
    Display_3D = 0x0302,
    Display_Other = 0x0380,

    Multimedia_Video = 0x0400,
    Multimedia_AudioController = 0x0401,
    Multimedia_Telephony = 0x0402,
    Multimedia_AudioDevice = 0x0403,
    Multimedia_Other = 0x0480,

    Memory_RAM = 0x0500,
    Memory_Flash = 0x0501,
    Memory_Other = 0x0580,

    Bridge_Host = 0x0600,
    Bridge_ISA = 0x0601,
    Bridge_EISA = 0x0602,
    Bridge_MCA = 0x0603,
    Bridge_PciToPci = 0x0604,
    Bridge_PCMCIA = 0x0605,
    Bridge_NuBus = 0x0606,
    Bridge_CardBus = 0x0607,
    Bridge_RACEway = 0x0608,
    Bridge_PciToPciSemiTransparent = 0x0609,
    Bridge_InfinibandToPci = 0x060A,
    Bridge_Other = 0x0680,

    Unknown = 0xFFFF,
}
impl PciFullClass {
    pub fn from_u16(u: u16) -> PciFullClass {
        let opt = num::FromPrimitive::from_u16(u);
        match opt {
            Some(e) => e,
            None => PciFullClass::Unknown
        }
    }
    pub fn as_u16(&self) -> u16 { *self as u16 }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct PciDeviceInfo {
    pub device: u8,
    pub bus: u8,
    pub device_id: u16,
    pub vendor_id: u16,
    pub full_class: PciFullClass,
    pub header_type: u8,
    pub bars: [u32; 6],
    pub supported_fns: [bool; 8],
}
impl PciDeviceInfo {
    pub fn class(&self) -> PciClass {
        PciClass::from_u8(((self.full_class.as_u16() >> 8) & 0xFF) as u8)
    }
    pub fn subclass(&self) -> PciClass {
        PciClass::from_u8((self.full_class.as_u16() & 0xFF) as u8)
    }
}
impl Display for PciDeviceInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let vendor_name = name_for_vendor_id(self.vendor_id);
        writeln!(f, "Device {:X} | Bus {:X} | Vendor: {}", self.device, self.bus, vendor_name)?;
        writeln!(f, "    Class: {:?} ({:#06X})", self.full_class, self.full_class.as_u16())?;
        writeln!(f, "    Header type: {:X}", self.header_type)?;
        write!(f,   "    Supported functions: 0")?;
        for (i, b) in self.supported_fns.iter().enumerate().skip(1) {
            if *b {
                write!(f, ", {}", i)?;
            }
        }
        writeln!(f)?;
        write!(f, "    BARs: [ ")?;
        for i in self.bars.iter() {
            if *i == 0 {
                write!(f, "0x0 ")?;
            }
            else {
                write!(f, "{:#010X} ", i)?;
            }
        }
        writeln!(f, "]")?;
        Ok(())
    }
}

pub fn name_for_vendor_id(vendor_id: u16) -> String {
    match vendor_id {
        0x8086 => "Intel Corp. (0x8086)".into(),
        0x1234 => "QEMU (0x1234)".into(),
        _ => format!("Unknown({:#06X})", vendor_id)
    }
}

pub fn brute_force_scan(infos: &mut Vec<PciDeviceInfo>) {
    for bus in 0u8..=255 {
        for device in 0u8..32 {
            if let Some(info) = check_device(bus, device) {
                infos.push(info);
            }
        }
    }
}

fn check_device(bus: u8, device: u8) -> Option<PciDeviceInfo> {
    let function = 0u8;

    let (device_id, vendor_id) = get_ids(bus, device, function);
    if vendor_id == 0xFFFF {
        // Device doesn't exist
        return None;
    }

    let class = pci_config_read(bus, device, 0, 0x8);
    let class = (class >> 16) & 0x0000FFFF;
    let pci_class = PciFullClass::from_u16(class as u16);
    let header_type = get_header_type(bus, device, function);

    let mut supported_fns = [true, false, false, false, false, false, false, false];
    if (header_type & 0x80) != 0 {
        // It is a multi-function device, so check remaining functions
        for function in 0u8..8 {
            if get_ids(bus, device, function).1 != 0xFFFF {
                if check_function(bus, device, function) {
                    supported_fns[function as usize] = true;
                }
            }
        }
    }

    let mut bars = [0, 0, 0, 0, 0, 0];
    bars[0] = pci_config_read(bus, device, 0, 0x10);
    bars[1] = pci_config_read(bus, device, 0, 0x14);
    bars[2] = pci_config_read(bus, device, 0, 0x18);
    bars[3] = pci_config_read(bus, device, 0, 0x1C);
    bars[4] = pci_config_read(bus, device, 0, 0x20);
    bars[5] = pci_config_read(bus, device, 0, 0x24);

    Some(PciDeviceInfo {
        device, bus, device_id, vendor_id,
        full_class: pci_class,
        header_type,
        bars,
        supported_fns
    })
}

fn pci_config_read (bus: u8, device: u8, func: u8, offset: u8) -> u32 {
    let bus = bus as u32;
    let device = device as u32;
    let func = func as u32;
    let offset = offset as u32;
    // construct address param
    let address = ((bus << 16) | (device << 11) | (func << 8) | (offset & 0xfc) | 0x80000000) as u32;

    // write address
    let mut port = Port::new(0xCF8);
    unsafe { port.write(address); }

    // read data
    let mut port = Port::new(0xCFC);
    unsafe { port.read() }
}

fn get_header_type(bus: u8, device: u8, function: u8) -> u8 {
    let res = pci_config_read(bus, device, function, 0x0C);
    ((res >> 16) & 0xFF) as u8
}

fn check_function(bus: u8, device: u8, function: u8) -> bool {
    get_ids(bus, device, function).1 != 0xFFFF
}

fn get_ids(bus: u8, device: u8, function: u8) -> (u16, u16) {
    let res = pci_config_read(bus, device, function, 0);
    let dev_id = ((res >> 16) & 0xFFFF) as u16;
    let vnd_id = (res & 0xFFFF) as u16;
    (dev_id, vnd_id)
}
