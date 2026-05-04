use crate::Packfile;

#[derive(Debug)]
pub enum VolitionError {
    BufferTooSmall {
        need: usize,
        got: usize,
    },
    UnexpectedValue {
        field: &'static str,
        expected: i32,
        got: i32,
    },
    InvalidString {
        offset: usize,
    },
    InvalidPackfileMagic(i32),
    UnknownPackfileVersion(i32),
    PackfileCompression,
}

impl std::fmt::Display for VolitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use VolitionError::*;

        match self {
            BufferTooSmall { need, got } => write!(f, "Not enough bytes: need {need}, got {got}"),
            UnexpectedValue {
                field,
                expected,
                got,
            } => write!(
                f,
                "Unexpected value for `{field}`: expected {expected}, got {got}"
            ),
            InvalidString { offset } => write!(f, "Invalid string at offset: {offset:X?}"),
            InvalidPackfileMagic(got) => write!(
                f,
                "Invalid magic for packfile: expected {:08X?}, got {got:08X?}",
                Packfile::MAGIC
            ),
            UnknownPackfileVersion(got) => write!(
                f,
                "Unknown pack version: expected {:08X?}, got {got:08X?}",
                Packfile::VERSION
            ),
            PackfileCompression => write!(f, "Packfile compression not yet supported"),
        }
    }
}
