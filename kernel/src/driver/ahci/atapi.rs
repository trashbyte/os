// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

//use super::HbaPort;
//use crate::driver::ahci::{PrdEntry, ATA_DEV_BUSY, ATA_DEV_DRQ};

pub const ATAPI_PACKET_SIZE: usize = 16;
pub const ATA_CMD_PACKET: u8 = 0xA0;
pub const ATA_FEAT_PACKET_DMA: u8 = 0x01;
pub const ATA_FEAT_PACKET_DMADIR: u8 = 0x03;

pub struct CmdFis {
    cmd: u8,              // Command
    features: u8,         // Features
    lba: u32,             // LBA (24-bit)
    device: u8,           // Device
    lba_exp: u32,         // LBA (exp) (24-bit)
    features_exp: u8,     // Features (exp)
    sector_count: u8,	  // Sector Count
    sector_count_exp: u8, // Sector Count (exp)
    control: u8,          // Control
}
impl CmdFis {
    pub fn zeroed() -> Self {
        Self {
            cmd: 0, features: 0, lba: 0, device: 0, lba_exp: 0,
            features_exp: 0, sector_count: 0, sector_count_exp: 0, control: 0
        }
    }
}

//pub unsafe fn atapi_check_capacity(&self) {
//    self.select();
//    ide_400ns_delay();
//
//    Port::new(self.io + ATA_REG_FEATURES).write(0u8);
//    Port::new(self.io + ATA_REG_LBA1).write(8u8);
//    Port::new(self.io + ATA_REG_LBA2).write(0u8);
//
//    Port::new(self.io + ATA_REG_COMMAND).write(ATA_CMD_PACKET);
//
//    self.poll();
//
//    let mut data_port = Port::<u16>::new(self.io + ATA_REG_DATA);
//    data_port.write(0x25);
//    data_port.write(0);
//    data_port.write(0);
//    data_port.write(0);
//    data_port.write(0);
//    data_port.write(0);
//
//    if self.check_error() != 0 {
//        panic!("atapi packet error: {:08b}", self.check_error());
//    }
//
//    let mut lba: u32 = 0;
//    lba |= (data_port.read()) as u32;
//    lba |= (data_port.read() << 16) as u32;
//    let mut blocks: u32 = 0;
//    blocks |= (data_port.read()) as u32;
//    blocks |= (data_port.read() << 16) as u32;
//    serial_println!("lba: {}, blocks: {}", lba, blocks);
//}

//pub fn port_exec(port: *mut HbaPort, cmd: u32, timeout: u32) {
//    // Execute a command on a port, wait for the command to complete or for
//    // a timeout, and return whether the command succeeded or not.
//
//    let mut spin = 0;
//
//    //port_issue(ps, cmd, timeout);
//    port.command_issue.write(1 << cmd);
//
//    /* Put the thread to sleep until a timeout or a command completion
//     * happens. Earlier, we used to call port_wait which set the suspended
//     * flag. We now abandon it since the flag has to work on a per-thread,
//     * and hence per-tag basis and not on a per-port basis. Instead, we
//     * retain that call only to defer open calls during device/driver
//     * initialization. Instead, we call sleep here directly. Before
//     * sleeping, we register the thread.
//     */
//    //ps->cmd_info[cmd].tid = blockdriver_mt_get_tid();
//
//    //blockdriver_mt_sleep();
//
//    /* Cancelling a timer that just triggered, does no harm. */
//    //cancel_timer(&ps->cmd_info[cmd].timer);
//
//    //assert(!(ps->flags & FLAG_BUSY));
//    while (port.task_file_data.read() & (ATA_DEV_BUSY | ATA_DEV_DRQ) != 0) && spin < 1000000 {
//        spin += 1;
//    }
//    if spin == 1000000 {
//        panic!("Timeout");
//    }
//}
//
//
//pub fn atapi_exec(port: &mut HbaPort, cmd: u32, packet: &[u8; ATAPI_PACKET_SIZE], size: u32, write: u32) {
//    // Execute an ATAPI command. Return OK or error.
//    let mut fis = CmdFis::zeroed();
//    let mut prdt = HbaPrdtEntry::zeroed();
//    let mut nr_prds = 0;
//
//    // Fill in the command table with a FIS, a packet, and if a data transfer is requested, also a PRD.
//    fis.cmd = ATA_CMD_PACKET;
//
//    if size > 0 {
//        fis.features = ATA_FEAT_PACKET_DMA;
//        if !write && (port.flags & FLAG_USE_DMADIR) {
//            fis.features |= ATA_FEAT_PACKET_DMADIR;
//        }
//
//        prdt.data_base_addr_lower = 0; //ps->tmp_phys;
//        prdt.data_byte_count = size;
//
//        nr_prds = 1;
//    }
//
//    /* Start the command, and wait for it to complete or fail. */
//    port_set_cmd(ps, cmd, &fis, packet, prd, nr_prds, write);
//
//    port_exec(port, cmd, 0);
//}
//
//pub fn port_set_cmd(port: &mut HbaPort, cmd: u8, fis: &CmdFis, packet: Option<[u8; ATAPI_PACKET_SIZE]>, prdt: &HbaPrdtEntry, nr_prds: u32, write: u32) {
//    // Prepare the given command for execution, by constructing a command table and setting up a command list entry pointing to the table.
//    u8_t *ct;
//    u32_t *cl;
//    vir_bytes size;
//
//    /* Construct a command table, consisting of a command FIS, optionally
//    * a packet, and optionally a number of PRDs (making up the actual PRD
//    * table).
//    */
//    // ct = ptr to cmd table
//
//    size = ct_set_fis(ct, fis, cmd);
//
//    if let Some(p) = packet {
//        ct_set_packet(ct, p);
//    }
//
//    ct_set_prdt(ct, prdt, nr_prds);
//
//    // Construct a command list entry, pointing to the command's table. Current assumptions:
//    // callers always provide a Register - Host to Device type FIS, and all non-NCQ commands are prefetchable.
//    // cl = ptr to cmd header
//
//    cl[0] = (nr_prds << AHCI_CL_PRDTL_SHIFT) |
//        ((!ATA_IS_FPDMA_CMD(fis->cf_cmd) && (nr_prds > 0 || packet != NULL)) ? AHCI_CL_PREFETCHABLE : 0) |
//        (write ? AHCI_CL_WRITE : 0) | ((packet != NULL) ? AHCI_CL_ATAPI : 0) |
//        ((size / sizeof(u32_t)) << AHCI_CL_CFL_SHIFT);
//
//    cl[2] = ps->ct_phys[cmd];
//}
