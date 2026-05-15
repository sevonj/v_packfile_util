// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::error::Error;

use crate::Packfile;
use crate::StaticMeshHeader;

#[derive(Debug)]
pub enum VolitionError {
    BufferTooSmall {
        for_what: &'static str,
        need: usize,
        avail: usize,
    },
    IoErr {
        src: std::io::Error,
    },
    ExpectedExactValue {
        field: &'static str,
        expected: i32,
        got: i32,
    },
    UnexpectedValue {
        desc: &'static str,
        got: i32,
    },
    ValueTooHigh {
        field: &'static str,
        max: usize,
        got: usize,
    },
    NonsensicalFloat {
        field: &'static str,
        got: f32,
    },
    InvalidString {
        offset: usize,
    },
    CStringRanOutOfBytes(usize),

    InvalidPackfileSignature(i32),
    UnknownPackfileVersion(i32),
    PackfileCompression,

    InvalidStaticMeshSignature(i32),
    UnknownStaticMeshVersion(i16),
}

impl std::fmt::Display for VolitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use VolitionError::*;

        match self {
            BufferTooSmall {
                for_what,
                need,
                avail,
            } => write!(
                f,
                "Not enough bytes for {for_what:?}: need {need:?}, available {avail:?}"
            ),
            IoErr { src } => src.fmt(f),
            ExpectedExactValue {
                field,
                expected,
                got,
            } => write!(
                f,
                "Unexpected value for `{field}`: expected {expected}, got {got}"
            ),
            &UnexpectedValue { desc, got } => write!(f, "Unexpected value: {desc}, got {got}"),
            ValueTooHigh { field, max, got } => write!(
                f,
                "Value for `{field}` was larger than expected: max {max}, got {got}"
            ),
            NonsensicalFloat { field, got } => write!(
                f,
                "Nonsensical float in `{field}`: got {got} ({:08X?})",
                u32::from_le_bytes(got.to_le_bytes())
            ),
            InvalidString { offset } => write!(f, "Invalid string at offset: {offset:X?}"),
            CStringRanOutOfBytes(len) => {
                write!(f, "Buffer ran out before cstr nullterm. len: {len}")
            }
            InvalidPackfileSignature(got) => write!(
                f,
                "Invalid packfile signature: expected {:08X?}, got {got:08X?}",
                Packfile::SIGNATURE
            ),
            UnknownPackfileVersion(got) => write!(
                f,
                "Unknown packfile version: expected {:08X?}, got {got:08X?}",
                Packfile::VERSION
            ),
            PackfileCompression => write!(f, "Packfile compression not yet supported"),
            InvalidStaticMeshSignature(got) => write!(
                f,
                "Invalid static mesh signature: expected {:08X?}, got {got:08X?}",
                StaticMeshHeader::SIGNATURE
            ),
            UnknownStaticMeshVersion(got) => write!(
                f,
                "Unknown static mesh version: expected {:08X?}, got {got:08X?}",
                StaticMeshHeader::VERSION
            ),
        }
    }
}

impl Error for VolitionError {}

impl From<std::io::Error> for VolitionError {
    fn from(src: std::io::Error) -> Self {
        Self::IoErr { src }
    }
}
