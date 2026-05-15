// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::VolitionError;
use crate::util::*;

/// 1:1 from disk
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    pub fn from_le_unsized(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Self::from_le_bytes(buf[..size_of::<Self>()].try_into().unwrap())
    }

    pub fn from_le_bytes(buf: &[u8; size_of::<Self>()]) -> Result<Self, VolitionError> {
        Ok(Self {
            x: read_f32_le(buf, 0x0),
            y: read_f32_le(buf, 0x4),
            z: read_f32_le(buf, 0x8),
            w: read_f32_le(buf, 0xc),
        })
    }

    pub fn to_le_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        bytes[0x0..0x04].copy_from_slice(&self.x.to_le_bytes());
        bytes[0x4..0x08].copy_from_slice(&self.y.to_le_bytes());
        bytes[0x8..0x0c].copy_from_slice(&self.z.to_le_bytes());
        bytes[0xc..0x10].copy_from_slice(&self.w.to_le_bytes());
        bytes
    }

    /// Returns `true` if any component is NaN.
    pub const fn is_nan(&self) -> bool {
        self.x.is_nan() || self.y.is_nan() || self.z.is_nan() || self.w.is_nan()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_quat_size() {
        assert_eq!(size_of::<Quaternion>(), 0x10);
    }

    #[test]
    fn test_quat_cycle_bytes() {
        let mut buf = vec![];
        buf.extend_from_slice(&3.0_f32.to_le_bytes());
        buf.extend_from_slice(&4.0_f32.to_le_bytes());
        buf.extend_from_slice(&5.0_f32.to_le_bytes());
        buf.extend_from_slice(&7.0_f32.to_le_bytes());
        let quat = Quaternion::from_le_unsized(&buf).unwrap();
        assert_eq!(buf, quat.to_le_bytes());
    }
}
