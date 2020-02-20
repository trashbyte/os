// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

use spin::Mutex;

pub static TIME_START: Mutex<(u64, u64)> = Mutex::new((0, 0));
