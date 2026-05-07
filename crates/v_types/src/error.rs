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
    ValueTooHigh {
        field: &'static str,
        max: usize,
        got: usize,
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
            ValueTooHigh { field, max, got } => write!(
                f,
                "Value for `{field}` was larger than expected: max {max}, got {got}"
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
