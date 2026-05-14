// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2025 sevonj

/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::DivAssign;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Sub;
use std::ops::SubAssign;

use crate::VolitionError;
use crate::util::*;

/// 1:1 from disk
/// 3D float vector
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector {
    pub fn from_le_unsized(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;
        Self::from_le_bytes(buf[..size_of::<Self>()].try_into().unwrap())
    }

    pub fn from_le_bytes(buf: &[u8; size_of::<Self>()]) -> Result<Self, VolitionError> {
        Ok(Self {
            x: read_f32_le(buf, 0x0),
            y: read_f32_le(buf, 0x4),
            z: read_f32_le(buf, 0x8),
        })
    }

    pub fn to_le_bytes(&self) -> [u8; size_of::<Self>()] {
        let mut bytes = [0; size_of::<Self>()];
        bytes[0x0..0x4].copy_from_slice(&self.x.to_le_bytes());
        bytes[0x4..0x8].copy_from_slice(&self.y.to_le_bytes());
        bytes[0x8..0xc].copy_from_slice(&self.z.to_le_bytes());
        bytes
    }

    /// Returns `true` if any component is NaN.
    pub const fn is_nan(&self) -> bool {
        self.x.is_nan() || self.y.is_nan() || self.z.is_nan()
    }

    pub fn length(&self) -> f32 {
        let sq = self.x * self.x + self.y * self.y + self.z * self.z;
        sq.sqrt()
    }
}

impl Add for Vector {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl AddAssign for Vector {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vector {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl SubAssign for Vector {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for Vector {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl Mul<f32> for Vector {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl MulAssign for Vector {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl MulAssign<f32> for Vector {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div for Vector {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }
}

impl Div<f32> for Vector {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl DivAssign for Vector {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl DivAssign<f32> for Vector {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_vector_size() {
        assert_eq!(size_of::<Vector>(), 0x0c);
    }

    #[test]
    fn test_aabb_cycle_bytes() {
        let mut buf = vec![];
        buf.extend_from_slice(&3.0_f32.to_le_bytes());
        buf.extend_from_slice(&4.0_f32.to_le_bytes());
        buf.extend_from_slice(&5.0_f32.to_le_bytes());
        let vec = Vector::from_le_unsized(&buf).unwrap();
        assert_eq!(buf, vec.to_le_bytes());
    }
}
