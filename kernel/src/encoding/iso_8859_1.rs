///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use crate::encoding::{EncodingError, InvalidCharPolicy, CharacterType, handle_invalid, EncodingResult};
use alloc::string::String;

const CHARACTER_CODES: [char; 256] = [
    '\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0',
    '\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0',
    ' ', '!', '"', '#', '$', '%', '&', '\'','(', ')', '*', '+', ',', '-', '.', '/',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
    '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
    'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\',']', '^', '_',
    '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
    'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '\0',
    '\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0',
    '\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0','\0',
    '\u{A0}','\u{A1}','\u{A2}','\u{A3}','\u{A4}','\u{A5}','\u{A6}','\u{A7}','\u{A8}','\u{A9}','\u{AA}','\u{AB}','\u{AC}','\u{AD}','\u{AE}','\u{AF}',
    '\u{B0}','\u{B1}','\u{B2}','\u{B3}','\u{B4}','\u{B5}','\u{B6}','\u{B7}','\u{B8}','\u{B9}','\u{BA}','\u{BB}','\u{BC}','\u{BD}','\u{BE}','\u{BF}',
    '\u{C0}','\u{C1}','\u{C2}','\u{C3}','\u{C4}','\u{C5}','\u{C6}','\u{C7}','\u{C8}','\u{C9}','\u{CA}','\u{CB}','\u{CC}','\u{CD}','\u{CE}','\u{CF}',
    '\u{D0}','\u{D1}','\u{D2}','\u{D3}','\u{D4}','\u{D5}','\u{D6}','\u{D7}','\u{D8}','\u{D9}','\u{DA}','\u{DB}','\u{DC}','\u{DD}','\u{DE}','\u{DF}',
    '\u{E0}','\u{E1}','\u{E2}','\u{E3}','\u{E4}','\u{E5}','\u{E6}','\u{E7}','\u{E8}','\u{E9}','\u{EA}','\u{EB}','\u{EC}','\u{ED}','\u{EE}','\u{EF}',
    '\u{F0}','\u{F1}','\u{F2}','\u{F3}','\u{F4}','\u{F5}','\u{F6}','\u{F7}','\u{F8}','\u{F9}','\u{FA}','\u{FB}','\u{FC}','\u{FD}','\u{FE}','\u{FF}',
];

pub fn decode_slice(chars: &[u8], policy: Option<InvalidCharPolicy>) -> Result<String, EncodingError> {
    let mut result = String::new();
    for i in 0..chars.len() {
        match decode_char(CharacterType::SingleByte(chars[i]), &policy) {
            EncodingResult::Ok(c) => result.push(c),
            EncodingResult::Err(e) => return Err(e),
            EncodingResult::Ignore => {} // do nothing
            EncodingResult::StopEarly => return Ok(result)
        }
    }
    Ok(result)
}

pub unsafe fn decode_ptr(chars: *const u8, len: usize, policy: Option<InvalidCharPolicy>) -> Result<String, EncodingError> {
    let mut result = String::new();
    for i in 0..len {
        let addr = chars as u64 + i as u64;
        match decode_char(CharacterType::SingleByte(unsafe { *(addr as *const u8) }), &policy) {
            EncodingResult::Ok(c) => result.push(c),
            EncodingResult::Err(e) => return Err(e),
            EncodingResult::Ignore => {} // do nothing
            EncodingResult::StopEarly => return Ok(result)
        }
    }
    Ok(result)
}

pub fn decode_char(character: CharacterType, policy: &Option<InvalidCharPolicy>) -> EncodingResult {
    match character {
        CharacterType::SingleByte(b) => {
            let c = CHARACTER_CODES[b as usize];
            if b == 0 {
                return EncodingResult::StopEarly;
            }
            if c == '\0' {
                handle_invalid(character, policy)
            }
            else {
                EncodingResult::Ok(c)
            }
        },
        CharacterType::DoubleByte(_,_) => unreachable!()
    }
}
