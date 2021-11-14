// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref LOCAL_APIC: Mutex<APIC> = Mutex::new(APIC::new(0));
}

pub struct APIC {
    pub base_addr: u64,
}
impl APIC {
    pub fn new(base_addr: u64) -> Self {
        Self { base_addr }
    }
}
