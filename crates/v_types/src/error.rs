use std::error::Error;

use crate::Packfile;

#[derive(Debug)]
pub enum VolitionError {
    BufferTooSmall {
        need: usize,
        got: usize,
    },
    IoErr {
        src: std::io::Error,
    },
    UnexpectedValue {
        field: &'static str,
        expected: i32,
        got: i32,
    },
    InvalidString {
        offset: usize,
    },

    InvalidPackfileSignature(i32),
    UnknownPackfileVersion(i32),
    PackfileCompression,
}

impl std::fmt::Display for VolitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use VolitionError::*;

        match self {
            BufferTooSmall { need, got } => write!(f, "Not enough bytes: need {need}, got {got}"),
            IoErr { src } => src.fmt(f),
            UnexpectedValue {
                field,
                expected,
                got,
            } => write!(
                f,
                "Unexpected value for `{field}`: expected {expected}, got {got}"
            ),
            InvalidString { offset } => write!(f, "Invalid string at offset: {offset:X?}"),
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
        }
    }
}

impl Error for VolitionError {}

impl From<std::io::Error> for VolitionError {
    fn from(src: std::io::Error) -> Self {
        Self::IoErr { src }
    }
}
