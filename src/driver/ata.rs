// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

#![allow(dead_code)]

use x86_64::instructions::port::Port;
use core::fmt::Debug;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;
use num_traits::Float;
use crate::driver::{StorageDriver, WriteError, MountError, ReadError};
use crate::path::Path;

#[derive(Debug)]
pub struct AtaDrives {
    pub master0: Option<AtaDrive>,
    pub slave0: Option<AtaDrive>,
    pub master1: Option<AtaDrive>,
    pub slave1: Option<AtaDrive>,
}
impl AtaDrives {
    pub fn new() -> Self {
        Self {
            master0: None, slave0: None, master1: None, slave1: None
        }
    }
}

#[derive(Debug)]
pub struct AtaDrive {
    pub bus: u8,
    pub drive_num: u8,
    pub serial_number: String,
    pub model_number: String,
    pub num_cylinders: u16,
    pub num_heads: u16,
    pub sectors_per_track: u16,
    pub total_sectors: u32,
    pub maximum_block_transfer: u8,
    pub current_sector_capacity: u32,
    pub user_addressable_sectors: u32,
    pub io: u16,
}
impl AtaDrive {
    pub fn from_identify(data: AtaIdentifyData, bus: u8, drive_num: u8) -> Self {
        if bus > 1 { panic!("Invalid IDE bus id: {}", bus) }
        if drive_num > 1 { panic!("Invalid IDE drive number: {}", drive_num) }
        let mut serial_number = String::new();
        for c in data.serial_number.iter() {
            serial_number.push(*c as char);
        }
        let mut model_number = String::new();
        for c in data.model_number_1.iter() {
            model_number.push(*c as char);
        }
        for c in data.model_number_2.iter() {
            model_number.push(*c as char);
        }
        let io = match bus {
            ATA_PRIMARY => ATA_PRIMARY_IO,
            ATA_SECONDARY => ATA_SECONDARY_IO,
            _ => unreachable!()
        };
        Self {
            bus, drive_num, serial_number, model_number,
            num_cylinders: data.num_cylinders,
            num_heads: data.num_heads,
            sectors_per_track: data.num_sectors_per_track,
            total_sectors: data.num_cylinders as u32 * data.num_heads as u32 * data.num_sectors_per_track as u32,
            maximum_block_transfer: data.maximum_block_transfer,
            current_sector_capacity: data.current_sector_capacity,
            user_addressable_sectors: data.user_addressable_sectors,
            io
        }
    }

    // TODO: errors and such
    pub unsafe fn read_sector_to_slice(&self, buf: &mut [u8], lba: u32) {
        assert_eq!(buf.len(), 512);
        self.select();
        /* We only support 28bit LBA so far */
        let cmd = match self.drive_num {
            ATA_MASTER => 0xE0,
            ATA_SLAVE => 0xF0,
            _ => unreachable!()
        };

        Port::new(self.io + ATA_REG_HDDEVSEL).write(cmd | ((lba >> 24) & 0x0F) as u8);
        Port::new(self.io + 1).write(0u8);

        // Single sector read
        Port::new(self.io + ATA_REG_SECCOUNT0).write(1u8);

        // Select LBA
        Port::new(self.io + ATA_REG_LBA0).write((lba & 0xFF) as u8);
        Port::new(self.io + ATA_REG_LBA1).write(((lba >> 8) & 0xFF) as u8);
        Port::new(self.io + ATA_REG_LBA2).write(((lba >> 16) & 0xFF) as u8);

        // Select read command
        Port::new(self.io + ATA_REG_COMMAND).write(ATA_CMD_READ_PIO);

        // Wait until ready
        self.poll();

        let mut port = Port::<u16>::new(self.io + ATA_REG_DATA);
        for i in 0..256 {
            let double = port.read();
            buf[i*2] = (double & 0xFF) as u8;
            buf[i*2+1] = ((double >> 8) & 0xFF) as u8;
        }
        ide_400ns_delay();
    }

    // TODO: errors and such
    pub unsafe fn read_sector_to_vec(&self, sector: u32) -> Vec<u8> {
        let mut output = vec![0u8; 512];
        self.read_sector_to_slice(&mut output, sector);
        output
    }

    // TODO: errors and such
    unsafe fn ata_write_sector(&self, data: [u8; 512], lba: u32) {
        // We only support 28bit LBA so far
        let cmd = match self.drive_num {
            ATA_MASTER => 0xE0,
            ATA_SLAVE => 0xF0,
            _ => unreachable!()
        };

        Port::new(self.io + ATA_REG_HDDEVSEL).write(cmd | (lba >> 24 & 0x0F) as u8);
        Port::new(self.io + 1).write(0u8);

        // Single sector write
        Port::new(self.io + ATA_REG_SECCOUNT0).write(1u8);

        // Select LBA
        Port::new(self.io + ATA_REG_LBA0).write((lba & 0xFF) as u8);
        Port::new(self.io + ATA_REG_LBA1).write(((lba >> 8) & 0xFF) as u8);
        Port::new(self.io + ATA_REG_LBA2).write(((lba >> 16) & 0xFF) as u8);

        // Select write command
        Port::new(self.io + ATA_REG_COMMAND).write(ATA_CMD_WRITE_PIO);

        // Wait until ready
        self.poll();

        let mut port = Port::<u16>::new(self.io + ATA_REG_DATA);
        for i in 0..256 {
            let double = (data[i*2] as u16) | ((data[i*2+1] as u16) << 8);
            port.write(double);
        }
        ide_400ns_delay();
    }

    unsafe fn poll(&self) {
        ide_400ns_delay();

        let mut altstatus_port = Port::<u8>::new(self.io + ATA_REG_ALTSTATUS);
        let mut status_port = Port::<u8>::new(self.io + ATA_REG_STATUS);
        for _ in 0..4 {
            altstatus_port.read();
        }
        // wait for BSY to clear
        while status_port.read() & ATA_SR_BSY != 0 {}

        let mut status = status_port.read();
        while status & ATA_SR_DRQ == 0 {
            status = status_port.read();

            if status & ATA_SR_ERR != 0 {
                panic!("ERR set in ide_poll(): {:08b}", self.check_error());
            }
        }
        // DRQ set, ready for PIO
    }

    pub fn select(&self) {
        unsafe { ide_select_drive(self.bus, self.drive_num); }
    }

    pub unsafe fn check_error(&self) -> u8 {
        Port::new(self.io + ATA_REG_ERROR).read()
    }

    // Associated functions ////////////////////////////////////////////////////

    pub fn sector_containing_addr(addr: u64) -> u32 {
        (addr as f64 / 512.0).floor() as u32
    }
}

impl StorageDriver for AtaDrive {
    fn mount(&mut self, path: Path) -> Result<(), MountError> {
        unimplemented!()
    }

    fn unmount(&mut self) -> Result<(), MountError> {
        unimplemented!()
    }

    fn read_sector(&self, sector_num: u32) -> Result<[u8; 512], ReadError> {
        let mut data = [0u8; 512];
        unsafe { self.read_sector_to_slice(&mut data, sector_num); }
        Ok(data)
    }

    fn write_sector(&mut self, sector_num: u32, sector_data: &[u8; 512]) -> Result<(), WriteError> {
        unsafe { self.ata_write_sector(sector_data.clone(), sector_num); }
        Ok(())
    }
}

//pub fn scan() {
//    for bus in 0..2 {
//        for device in 0..2 {
//            unsafe {
//                if let Some(info) = ide_identify(bus, device) {
//                    let dev = PhysicalDevice {
//                        id: PhysicalDeviceId(0),
//                        dev_type: PhysicalDeviceType::FloppyDrive
//                    };
//                }
//            }
//        }
//    }
//}

#[derive(Debug, Clone)]
#[allow(dead_code, non_snake_case)]
#[repr(C)]
pub struct AtaIdentifyData {
    //    Reserved1 : 1;
    //    Retired3 : 1;
    //    ResponseIncomplete : 1;
    //    Retired2 : 3;
    //    FixedDevice : 1;
    //    RemovableMedia : 1;
    //    Retired1 : 7;
    //    DeviceType : 1;
    general_conf: u16,
    num_cylinders: u16,
    specific_conf: u16,
    num_heads: u16,
    retired_1: [u16; 2],
    num_sectors_per_track: u16,
    vendor_unique_1: [u16; 3],
    serial_number: [u8; 20],
    retired_2: [u16; 2],
    obselete_1: u16,
    firmware_revision: [u8; 8],
    model_number_1: [u8; 20],
    model_number_2: [u8; 20],
    maximum_block_transfer: u8,
    vendor_unique_2: u8,
    trusted_computing_supported: u16,
    //    u8  CurrentLongPhysicalSectorAlignment : 2;
    //    u8  ReservedByte49 : 6;
    //    u8  DmaSupported : 1;
    //    u8  LbaSupported : 1;
    //    u8  IordyDisable : 1;
    //    u8  IordySupported : 1;
    //    u8  Reserved1 : 1;
    //    u8  StandybyTimerSupport : 1;
    //    u8  Reserved2 : 2;
    //    u16 ReservedWord50;
    capabilities: [u8; 4],
    obsolete_words_51: [u16; 2],
    translation_fields_valid: u8,
    free_fall_controll_sensitivity: u8,
    number_of_current_cylinders: u16,
    number_of_current_heads: u16,
    current_sectors_per_track: u16,
    current_sector_capacity: u32,
    current_multi_sector_setting: u8,
    //u8  MultiSectorSettingValid : 1;
    //u8  ReservedByte59 : 3;
    //u8  SanitizeFeatureSupported : 1;
    //u8  CryptoScrambleExtCommandSupported : 1;
    //u8  OverwriteExtCommandSupported : 1;
    //u8  BlockEraseExtCommandSupported : 1;
    features_supported: u8,
    user_addressable_sectors: u32,
    obsolete_word_62: u16,
    multi_word_dma_support: u8,
    multi_word_dma_active: u8,
    advanced_pio_modes: u8,
    reserved_byte_64: u8,
    MinimumMWXferCycleTime: u16,
    RecommendedMWXferCycleTime: u16,
    MinimumPIOCycleTime: u16,
    MinimumPIOCycleTimeIORDY: u16,
    //    u16 ZonedCapabilities : 2;
    //    u16 NonVolatileWriteCache : 1;
    //    u16 ExtendedUserAddressableSectorsSupported : 1;
    //    u16 DeviceEncryptsAllUserData : 1;
    //    u16 ReadZeroAfterTrimSupported : 1;
    //    u16 Optional28BitCommandsSupported : 1;
    //    u16 IEEE1667 : 1;
    //    u16 DownloadMicrocodeDmaSupported : 1;
    //    u16 SetMaxSetPasswordUnlockDmaSupported : 1;
    //    u16 WriteBufferDmaSupported : 1;
    //    u16 ReadBufferDmaSupported : 1;
    //    u16 DeviceConfigIdentifySetDmaSupported : 1;
    //    u16 LPSAERCSupported : 1;
    //    u16 DeterministicReadAfterTrimSupported : 1;
    //    u16 CFastSpecSupported : 1;
    AdditionalSupported: u16,
    ReservedWords70: [u16; 5],
    QueueDepth: u16,
    //    u16 Reserved0 : 1;
    //    u16 SataGen1 : 1;
    //    u16 SataGen2 : 1;
    //    u16 SataGen3 : 1;
    //    u16 Reserved1 : 4;
    //    u16 NCQ : 1;
    //    u16 HIPM : 1;
    //    u16 PhyEvents : 1;
    //    u16 NcqUnload : 1;
    //    u16 NcqPriority : 1;
    //    u16 HostAutoPS : 1;
    //    u16 DeviceAutoPS : 1;
    //    u16 ReadLogDMA : 1;
    //    u16 Reserved2 : 1;
    //    u16 CurrentSpeed : 3;
    //    u16 NcqStreaming : 1;
    //    u16 NcqQueueMgmt : 1;
    //    u16 NcqReceiveSend : 1;
    //    u16 DEVSLPtoReducedPwrState : 1;
    //    u16 Reserved3 : 8;
    SerialAtaCapabilities: u32,
    //    u16 Reserved0 : 1;
    //    u16 NonZeroOffsets : 1;
    //    u16 DmaSetupAutoActivate : 1;
    //    u16 DIPM : 1;
    //    u16 InOrderData : 1;
    //    u16 HardwareFeatureControl : 1;
    //    u16 SoftwareSettingsPreservation : 1;
    //    u16 NCQAutosense : 1;
    //    u16 DEVSLP : 1;
    //    u16 HybridInformation : 1;
    //    u16 Reserved1 : 6;
    SerialAtaFeaturesSupported: u16,
    MajorRevision: u16,
    MinorRevision: u16,
    //    u16 SmartCommands : 1;
    //    u16 SecurityMode : 1;
    //    u16 RemovableMediaFeature : 1;
    //    u16 PowerManagement : 1;
    //    u16 Reserved1 : 1;
    //    u16 WriteCache : 1;
    //    u16 LookAhead : 1;
    //    u16 ReleaseInterrupt : 1;
    //    u16 ServiceInterrupt : 1;
    //    u16 DeviceReset : 1;
    //    u16 HostProtectedArea : 1;
    //    u16 Obsolete1 : 1;
    //    u16 WriteBuffer : 1;
    //    u16 ReadBuffer : 1;
    //    u16 Nop : 1;
    //    u16 Obsolete2 : 1;
    //    u16 DownloadMicrocode : 1;
    //    u16 DmaQueued : 1;
    //    u16 Cfa : 1;
    //    u16 AdvancedPm : 1;
    //    u16 Msn : 1;
    //    u16 PowerUpInStandby : 1;
    //    u16 ManualPowerUp : 1;
    //    u16 Reserved2 : 1;
    //    u16 SetMax : 1;
    //    u16 Acoustics : 1;
    //    u16 BigLba : 1;
    //    u16 DeviceConfigOverlay : 1;
    //    u16 FlushCache : 1;
    //    u16 FlushCacheExt : 1;
    //    u16 WordValid83 : 2;
    //    u16 SmartErrorLog : 1;
    //    u16 SmartSelfTest : 1;
    //    u16 MediaSerialNumber : 1;
    //    u16 MediaCardPassThrough : 1;
    //    u16 StreamingFeature : 1;
    //    u16 GpLogging : 1;
    //    u16 WriteFua : 1;
    //    u16 WriteQueuedFua : 1;
    //    u16 WWN64Bit : 1;
    //    u16 URGReadStream : 1;
    //    u16 URGWriteStream : 1;
    //    u16 ReservedForTechReport : 2;
    //    u16 IdleWithUnloadFeature : 1;
    //    u16 WordValid : 2;
    CommandSetSupport: [u16; 3],
    //    u16 SmartCommands : 1;
    //    u16 SecurityMode : 1;
    //    u16 RemovableMediaFeature : 1;
    //    u16 PowerManagement : 1;
    //    u16 Reserved1 : 1;
    //    u16 WriteCache : 1;
    //    u16 LookAhead : 1;
    //    u16 ReleaseInterrupt : 1;
    //    u16 ServiceInterrupt : 1;
    //    u16 DeviceReset : 1;
    //    u16 HostProtectedArea : 1;
    //    u16 Obsolete1 : 1;
    //    u16 WriteBuffer : 1;
    //    u16 ReadBuffer : 1;
    //    u16 Nop : 1;
    //    u16 Obsolete2 : 1;
    //    u16 DownloadMicrocode : 1;
    //    u16 DmaQueued : 1;
    //    u16 Cfa : 1;
    //    u16 AdvancedPm : 1;
    //    u16 Msn : 1;
    //    u16 PowerUpInStandby : 1;
    //    u16 ManualPowerUp : 1;
    //    u16 Reserved2 : 1;
    //    u16 SetMax : 1;
    //    u16 Acoustics : 1;
    //    u16 BigLba : 1;
    //    u16 DeviceConfigOverlay : 1;
    //    u16 FlushCache : 1;
    //    u16 FlushCacheExt : 1;
    //    u16 Resrved3 : 1;
    //    u16 Words119_120Valid : 1;
    //    u16 SmartErrorLog : 1;
    //    u16 SmartSelfTest : 1;
    //    u16 MediaSerialNumber : 1;
    //    u16 MediaCardPassThrough : 1;
    //    u16 StreamingFeature : 1;
    //    u16 GpLogging : 1;
    //    u16 WriteFua : 1;
    //    u16 WriteQueuedFua : 1;
    //    u16 WWN64Bit : 1;
    //    u16 URGReadStream : 1;
    //    u16 URGWriteStream : 1;
    //    u16 ReservedForTechReport : 2;
    //    u16 IdleWithUnloadFeature : 1;
    //    u16 Reserved4 : 2;
    CommandSetActive: [u16; 3],
    UltraDMASupport: u8,
    UltraDMAActive: u8,
    //    u16 TimeRequired : 15;
    //    u16 ExtendedTimeReported : 1;
    NormalSecurityEraseUnit: u16,
    //    u16 TimeRequired : 15;
    //    u16 ExtendedTimeReported : 1;
    EnhancedSecurityEraseUnit: u16,
    CurrentAPMLevel: u8,
    ReservedWord91: u8,
    MasterPasswordID: u16,
    HardwareResetResult: u16,
    CurrentAcousticValue: u8,
    RecommendedAcousticValue: u8,
    StreamMinRequestSize: u16,
    StreamingTransferTimeDMA: u16,
    StreamingAccessLatencyDMAPIO: u16,
    StreamingPerfGranularity: u32,
    Max48BitLBA: [u32; 2],
    StreamingTransferTime: u16,
    DsmCap: u16,
    //    u16 LogicalSectorsPerPhysicalSector : 4;
    //    u16 Reserved0 : 8;
    //    u16 LogicalSectorLongerThan256Words : 1;
    //    u16 MultipleLogicalSectorsPerPhysicalSector : 1;
    //    u16 Reserved1 : 2;
    PhysicalLogicalSectorSize: u16,
    InterSeekDelay: u16,
    WorldWideName: [u16; 4],
    ReservedForWorldWideName128: [u16; 4],
    ReservedForTlcTechnicalReport: u16,
    WordsPerLogicalSector: [u16; 2],
    //    u16 ReservedForDrqTechnicalReport : 1;
    //    u16 WriteReadVerify : 1;
    //    u16 WriteUncorrectableExt : 1;
    //    u16 ReadWriteLogDmaExt : 1;
    //    u16 DownloadMicrocodeMode3 : 1;
    //    u16 FreefallControl : 1;
    //    u16 SenseDataReporting : 1;
    //    u16 ExtendedPowerConditions : 1;
    //    u16 Reserved0 : 6;
    //    u16 WordValid : 2;
    CommandSetSupportExt: u16,
    //    u16 ReservedForDrqTechnicalReport : 1;
    //    u16 WriteReadVerify : 1;
    //    u16 WriteUncorrectableExt : 1;
    //    u16 ReadWriteLogDmaExt : 1;
    //    u16 DownloadMicrocodeMode3 : 1;
    //    u16 FreefallControl : 1;
    //    u16 SenseDataReporting : 1;
    //    u16 ExtendedPowerConditions : 1;
    //    u16 Reserved0 : 6;
    //    u16 Reserved1 : 2;
    CommandSetActiveExt: u16,
    ReservedForExpandedSupportandActive: [u16; 6],
    MsnSupport: u16,
    //    u16 SecuritySupported : 1;
    //    u16 SecurityEnabled : 1;
    //    u16 SecurityLocked : 1;
    //    u16 SecurityFrozen : 1;
    //    u16 SecurityCountExpired : 1;
    //    u16 EnhancedSecurityEraseSupported : 1;
    //    u16 Reserved0 : 2;
    //    u16 SecurityLevel : 1;
    //    u16 Reserved1 : 7;
    SecurityStatus: u16,
    ReservedWord129: [u16; 31],
    //    u16 MaximumCurrentInMA : 12;
    //    u16 CfaPowerMode1Disabled : 1;
    //    u16 CfaPowerMode1Required : 1;
    //    u16 Reserved0 : 1;
    //    u16 Word160Supported : 1;
    CfaPowerMode1: u16,
    ReservedForCfaWord161: [u16; 7],
    NominalFormFactor: u16,
    //    u16 SupportsTrim : 1;
    //    u16 Reserved0 : 15;
    DataSetManagementFeature: u16,
    AdditionalProductID: [u16; 4],
    ReservedForCfaWord174: [u16; 2],
    CurrentMediaSerialNumber: [u16; 30],
    //    u16 Supported : 1;
    //    u16 Reserved0 : 1;
    //    u16 WriteSameSuported : 1;
    //    u16 ErrorRecoveryControlSupported : 1;
    //    u16 FeatureControlSuported : 1;
    //    u16 DataTablesSuported : 1;
    //    u16 Reserved1 : 6;
    //    u16 VendorSpecific : 4;
    SCTCommandTransport: u16,
    ReservedWord207: [u16; 2],
    //    u16 AlignmentOfLogicalWithinPhysical : 14;
    //    u16 Word209Supported : 1;
    //    u16 Reserved0 : 1;
    BlockAlignment: u16,
    WriteReadVerifySectorCountMode3Only: [u16; 2],
    WriteReadVerifySectorCountMode2Only: [u16; 2],
    //    u16 NVCachePowerModeEnabled : 1;
    //    u16 Reserved0 : 3;
    //    u16 NVCacheFeatureSetEnabled : 1;
    //    u16 Reserved1 : 3;
    //    u16 NVCachePowerModeVersion : 4;
    //    u16 NVCacheFeatureSetVersion : 4;
    NVCacheCapabilities: u16,
    NVCacheSizeLSW: u16,
    NVCacheSizeMSW: u16,
    NominalMediaRotationRate: u16,
    ReservedWord218: u16,
    //    u8 NVCacheEstimatedTimeToSpinUpInSeconds;
    //    u8 Reserved;
    NVCacheOptions: u16,
    WriteReadVerifySectorCountMode: u8,
    ReservedWord220: u8,
    ReservedWord221: u16,
    //    u16 MajorVersion : 12;
    //    u16 TransportType : 4;
    TransportMajorVersion: u16,
    TransportMinorVersion: u16,
    ReservedWord224: [u16; 6],
    ExtendedNumberOfUserAddressableSectors: [u32; 2],
    MinBlocksPerDownloadMicrocodeMode03: u16,
    MaxBlocksPerDownloadMicrocodeMode03: u16,
    ReservedWord236: [u16; 19],
    Signature: u8,
    CheckSum: u8,
}


pub const ATA_SR_BSY:  u8 = 0x80;
pub const ATA_SR_DRDY: u8 = 0x40;
pub const ATA_SR_DF:   u8 = 0x20;
pub const ATA_SR_DSC:  u8 = 0x10;
pub const ATA_SR_DRQ:  u8 = 0x08;
pub const ATA_SR_CORR: u8 = 0x04;
pub const ATA_SR_IDX:  u8 = 0x02;
pub const ATA_SR_ERR:  u8 = 0x01;

pub const ATA_ER_BBK:   u8 = 0x80;
pub const ATA_ER_UNC:   u8 = 0x40;
pub const ATA_ER_MC:    u8 = 0x20;
pub const ATA_ER_IDNF:  u8 = 0x10;
pub const ATA_ER_MCR:   u8 = 0x08;
pub const ATA_ER_ABRT:  u8 = 0x04;
pub const ATA_ER_TK0NF: u8 = 0x02;
pub const ATA_ER_AMNF:  u8 = 0x01;

pub const ATA_CMD_READ_PIO:        u8 = 0x20;
pub const ATA_CMD_READ_PIO_EXT:    u8 = 0x24;
pub const ATA_CMD_READ_DMA:        u8 = 0xC8;
pub const ATA_CMD_READ_DMA_EXT:    u8 = 0x25;
pub const ATA_CMD_WRITE_PIO:       u8 = 0x30;
pub const ATA_CMD_WRITE_PIO_EXT:   u8 = 0x34;
pub const ATA_CMD_WRITE_DMA:       u8 = 0xCA;
pub const ATA_CMD_WRITE_DMA_EXT:   u8 = 0x35;
pub const ATA_CMD_CACHE_FLUSH:     u8 = 0xE7;
pub const ATA_CMD_CACHE_FLUSH_EXT: u8 = 0xEA;
pub const ATA_CMD_PACKET:          u8 = 0xA0;
pub const ATA_CMD_IDENTIFY_PACKET: u8 = 0xA1;
pub const ATA_CMD_IDENTIFY:        u8 = 0xEC;

pub const ATAPI_CMD_READ:  u8 = 0xA8;
pub const ATAPI_CMD_EJECT: u8 = 0x1B;

pub const ATA_IDENT_DEVICETYPE:   u8 = 0;
pub const ATA_IDENT_CYLINDERS:    u8 = 2;
pub const ATA_IDENT_HEADS:        u8 = 6;
pub const ATA_IDENT_SECTORS:      u8 = 12;
pub const ATA_IDENT_SERIAL:       u8 = 20;
pub const ATA_IDENT_MODEL:        u8 = 54;
pub const ATA_IDENT_CAPABILITIES: u8 = 98;
pub const ATA_IDENT_FIELDVALID:   u8 = 106;
pub const ATA_IDENT_MAX_LBA:      u8 = 120;
pub const ATA_IDENT_COMMANDSETS:  u8 = 164;
pub const ATA_IDENT_MAX_LBA_EXT:  u8 = 200;

pub const IDE_ATA:   u8 = 0x00;
pub const IDE_ATAPI: u8 = 0x01;

pub const ATA_MASTER: u8 = 0x00;
pub const ATA_SLAVE:  u8 = 0x01;

pub const ATA_REG_DATA:       u16 = 0x00;
pub const ATA_REG_ERROR:      u16 = 0x01;
pub const ATA_REG_FEATURES:   u16 = 0x01;
pub const ATA_REG_SECCOUNT0:  u16 = 0x02;
pub const ATA_REG_LBA0:       u16 = 0x03;
pub const ATA_REG_LBA1:       u16 = 0x04;
pub const ATA_REG_LBA2:       u16 = 0x05;
pub const ATA_REG_HDDEVSEL:   u16 = 0x06;
pub const ATA_REG_COMMAND:    u16 = 0x07;
pub const ATA_REG_STATUS:     u16 = 0x07;
pub const ATA_REG_SECCOUNT1:  u16 = 0x08;
pub const ATA_REG_LBA3:       u16 = 0x09;
pub const ATA_REG_LBA4:       u16 = 0x0A;
pub const ATA_REG_LBA5:       u16 = 0x0B;
pub const ATA_REG_CONTROL:    u16 = 0x0C;
pub const ATA_REG_ALTSTATUS:  u16 = 0x0C;
pub const ATA_REG_DEVADDRESS: u16 = 0x0D;

// Channels:
pub const ATA_PRIMARY:   u8 = 0x00;
pub const ATA_SECONDARY: u8 = 0x01;

// Directions:
pub const ATA_READ:  u8 = 0x00;
pub const ATA_WRITE: u8 = 0x013;

pub const ATA_PRIMARY_IRQ:   u8 = 14;
pub const ATA_SECONDARY_IRQ: u8 = 15;

pub const ATA_PRIMARY_IO:   u16 = 0x1F0;
pub const ATA_SECONDARY_IO: u16 = 0x170;

pub const ATA_PRIMARY_DCR_AS:   u16 = 0x3F6;
pub const ATA_SECONDARY_DCR_AS: u16 = 0x376;

pub const SECTOR_SIZE: u32 = 512;

unsafe fn ide_select_drive(bus: u8, drive_num: u8) {
    let port = match bus {
        ATA_PRIMARY => ATA_PRIMARY_IO + ATA_REG_HDDEVSEL,
        ATA_SECONDARY => ATA_SECONDARY_IO + ATA_REG_HDDEVSEL,
        _ => panic!("Invalid IDE bus id: {}", bus)
    };
    let value: u32 = match drive_num {
        ATA_MASTER => 0xA0,
        ATA_SLAVE => 0xB0,
        _ => panic!("Invalid IDE drive number: {}", drive_num)
    };
    Port::new(port).write(value);
}

fn ide_primary_irq() {
    //pic_acknowledge(ATA_PRIMARY_IRQ);
}

fn ide_secondary_irq() {
    //pic_acknowledge(ATA_SECONDARY_IRQ);
}

pub unsafe fn ide_identify(bus: u8, drive: u8) -> Option<AtaIdentifyData> {
    ide_select_drive(bus, drive);

    let io = match bus {
        ATA_PRIMARY => ATA_PRIMARY_IO,
        ATA_SECONDARY => ATA_SECONDARY_IO,
        _ => panic!("Invalid IDE bus id: {}", bus)
    };

    // These registers must be zero for IDENTIFY
    Port::new(io + ATA_REG_SECCOUNT0).write(0u8);
    Port::new(io + ATA_REG_LBA0).write(0u8);
    Port::new(io + ATA_REG_LBA1).write(0u8);
    Port::new(io + ATA_REG_LBA2).write(0u8);

    // Send IDENTIFY command
    Port::new(io + ATA_REG_COMMAND).write(ATA_CMD_IDENTIFY);

    // Read status register
    let status: u8 = Port::new(io + ATA_REG_STATUS).read();
    if status != 0 {
        // Wait until BSY is clear
        while (Port::<u8>::new(io + ATA_REG_STATUS).read() & ATA_SR_BSY) != 0 {}

        let mut status: u8 = Port::new(io + ATA_REG_STATUS).read();
        while status & ATA_SR_DRQ == 0 {
            if status & ATA_SR_ERR != 0 {
                return None;
            }
            status = Port::new(io + ATA_REG_STATUS).read();
        }

        let mut buf = [0u16; 256];
        // Read the response (256 x u16)
        for i in 0..256 {
            buf[i] = Port::<u16>::new(io + ATA_REG_DATA).read();
        }
        let result = (*(&buf as *const u16 as *const AtaIdentifyData)).clone();
        return Some(result);
    }
    else {
        return None;
    }
}

fn ide_400ns_delay() {
    for _ in 0..4 {
        unsafe { Port::<u8>::new(0x80).read(); }
    }
}


