// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

#[derive(Debug, Clone, Copy)]
pub enum EncodingResult {
    Ok(char),
    Ignore,
    Err(EncodingError),
    StopEarly,
}

#[derive(Debug, Clone, Copy)]
pub enum CharacterType {
    SingleByte(u8),
    DoubleByte(u8, u8)
}

#[derive(Debug, Clone, Copy)]
pub enum InvalidCharPolicy {
    Ignore,
    ReplaceWithUnknownSymbol,
    ReplaceWithChar(char),
    Func(fn(CharacterType) -> EncodingResult),
    StopEarly,
    Abort
}

#[derive(Debug, Clone, Copy)]
pub enum EncodingError {
    InvalidCharacter
}

#[allow(non_snake_case)]
pub mod ISO_8859_1;


pub(crate) fn handle_invalid(c: CharacterType, policy: &Option<InvalidCharPolicy>) -> EncodingResult {
    match policy {
        None => EncodingResult::Err(EncodingError::InvalidCharacter),
        Some(p) => {
            match p {
                InvalidCharPolicy::Abort => EncodingResult::Err(EncodingError::InvalidCharacter),
                InvalidCharPolicy::Ignore => EncodingResult::Ignore,
                InvalidCharPolicy::ReplaceWithUnknownSymbol => EncodingResult::Ok('\u{FFFD}'),
                InvalidCharPolicy::ReplaceWithChar(c) => EncodingResult::Ok(*c),
                InvalidCharPolicy::Func(f) => f(c),
                InvalidCharPolicy::StopEarly => EncodingResult::StopEarly
            }
        }
    }
}
