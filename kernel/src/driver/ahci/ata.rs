///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use core::ptr;
use super::hba::HbaPort;
use crate::device::physical::{Disk, PhysicalDeviceType};
use alloc::boxed::Box;

enum BufferKind<'a> {
    Read(&'a mut [u8]),
    Write(&'a [u8]),
}

#[derive(Debug)]
struct Request {
    address: usize,
    total_sectors: usize,
    sector: usize,
    running_opt: Option<(u32, usize)>,
}

#[derive(Debug)]
pub struct AtaDisk {
    id: usize,
    port: &'static mut HbaPort,
    size: Option<u64>,
    request_opt: Option<Request>,
    buf: Box<[u8; 256 * 512]>
}

impl AtaDisk {
    pub fn new(id: usize, port: &'static mut HbaPort) -> Result<Self, anyhow::Error> {
        let buf = Box::new([0u8; 256 * 512]);
        port.init(id as u8);
        let size = unsafe { port.identify() };
        Ok(AtaDisk { id, port, size, request_opt: None, buf })
    }

    fn request(&mut self, block: u64, mut buffer_kind: BufferKind<'_>) -> Result<Option<usize>, anyhow::Error> {
        let (write, address, total_sectors) = match buffer_kind {
            BufferKind::Read(ref buffer) => (false, buffer.as_ptr() as usize, buffer.len()/512),
            BufferKind::Write(buffer) => (true, buffer.as_ptr() as usize, buffer.len()/512),
        };

        //TODO: Go back to interrupt magic
        let use_interrupts = false;
        loop {
            let mut request = match self.request_opt.take() {
                Some(request) => if address == request.address && total_sectors == request.total_sectors {
                    // Keep servicing current request
                    request
                } else {
                    // Have to wait for another request to finish
                    self.request_opt = Some(request);
                    return Ok(None);
                },
                None => {
                    // Create new request
                    Request {
                        address,
                        total_sectors,
                        sector: 0,
                        running_opt: None,
                    }
                }
            };

            // Finish a previously running request
            if let Some(running) = request.running_opt.take() {
                if self.port.ata_running(running.0) {
                    // Continue waiting for request
                    request.running_opt = Some(running);
                    self.request_opt = Some(request);
                    if use_interrupts {
                        return Ok(None);
                    } else {
                        //::std::thread::yield_now();
                        continue;
                    }
                }

                self.port.ata_stop(running.0)?;

                if let BufferKind::Read(ref mut buffer) = buffer_kind {
                    unsafe { ptr::copy(self.buf.as_ptr(), buffer.as_mut_ptr().add(request.sector * 512), running.1 * 512); }
                }

                request.sector += running.1;
            }

            if request.sector < request.total_sectors {
                // Start a new request
                let sectors = if request.total_sectors - request.sector >= 255 {
                    255
                } else {
                    request.total_sectors - request.sector
                };

                if let BufferKind::Write(buffer) = buffer_kind {
                    unsafe { ptr::copy(buffer.as_ptr().add(request.sector * 512), self.buf.as_mut_ptr(), sectors * 512); }
                }

                if let Some(slot) = self.port.ata_dma(block + request.sector as u64, sectors, write, &mut self.buf) {
                    request.running_opt = Some((slot, sectors));
                }

                self.request_opt = Some(request);

                if use_interrupts {
                    return Ok(None);
                } else {
                    //::std::thread::yield_now();
                    continue;
                }
            } else {
                // Done
                return Ok(Some(request.sector * 512));
            }
        }
    }
}

impl Disk for AtaDisk {
    fn id(&self) -> usize { self.id }
    fn kind(&self) -> PhysicalDeviceType { PhysicalDeviceType::SataDrive }
    fn size(&self) -> Option<u64> { self.size }

    fn read(&mut self, block: u64, buffer: &mut [u8]) -> Result<Option<usize>, anyhow::Error> {
        self.request(block, BufferKind::Read(buffer))
    }

    fn write(&mut self, block: u64, buffer: &[u8]) -> Result<Option<usize>, anyhow::Error> {
        self.request(block, BufferKind::Write(buffer))
    }

    fn block_length(&mut self) -> Result<u32, anyhow::Error> { Ok(512) }
}
