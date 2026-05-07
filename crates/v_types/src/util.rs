use std::any::type_name;

use crate::error::VolitionError;

pub fn check_fits_buf<T>(buf: &[u8]) -> Result<(), VolitionError> {
    let expected = size_of::<T>();
    if buf.len() < expected {
        Err(VolitionError::BufferTooSmall {
            for_what: type_name::<T>(),
            need: expected,
            avail: buf.len(),
        })
    } else {
        Ok(())
    }
}

pub fn read_i32_le(buf: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

#[allow(dead_code)]
pub fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

pub fn read_i16_le(buf: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap())
}

pub fn read_u16_le(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap())
}

pub fn read_f32_le(buf: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

pub fn read_bytes<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
    buf.get(offset..offset + N)
        .and_then(|b| b.try_into().ok())
        .unwrap()
}

pub fn read_cstr(buf: &[u8], offset: usize) -> Result<&str, VolitionError> {
    let buf = buf
        .get(offset..)
        .ok_or(VolitionError::CStringRanOutOfBytes(buf.len()))?;

    let len = buf
        .iter()
        .position(|&b| b == 0)
        .ok_or(VolitionError::InvalidString { offset })?;

    std::str::from_utf8(&buf[..len]).map_err(|_| VolitionError::InvalidString { offset })
}

pub fn align_16(offset: &mut usize) {
    *offset = offset.div_ceil(16) * 16;
}
