use crate::error::VolitionError;
use crate::util::*;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Packfile {
    pub magic: i32,
    pub version: i32,
    /// Unused. filename/pathname/pad probably, 65/256/3 bytes
    packfile_name_bufs: [u8; 324],
    pub flags: i32,
    /// Label from SRTT "packfile starts at this sector"
    /// Always Zero in SR2PC.
    pub sector: i32,
    pub num_files: i32,
    /// Size of this packfile itself
    pub len_packfile: i32,
    pub len_entries: i32,
    pub len_stems: i32,
    pub len_exts: i32,
    /// Label from SRTT.
    /// Uncompressed size
    pub len_data: i32,
    /// Label from SRTT
    /// Always -1 in SR2PC, whic doesn't use compression?
    pub len_compressed: i32,
    /// Label from SRTT
    pub ptr_entries: i32,
    /// Label from SRTT
    pub ptr_names: i32,
    /// Label from SRTT
    pub ptr_data: i32,
    /// Label from SRTT
    /// "how many files have open handles into the packfile"
    pub open_count: i32,
}

impl Default for Packfile {
    fn default() -> Self {
        Self {
            magic: Self::SIGNATURE,
            version: Self::VERSION,
            packfile_name_bufs: [0; 324],
            flags: 0,
            sector: 0,
            num_files: 0,
            len_packfile: 0,
            len_entries: 0,
            len_stems: 0,
            len_exts: 0,
            len_data: 0,
            len_compressed: -1,
            ptr_entries: -1,
            ptr_names: -1,
            ptr_data: -1,
            open_count: 0,
        }
    }
}

impl Packfile {
    pub const SIGNATURE: i32 = 0x51890ACE;
    pub const VERSION: i32 = 4;
    /// "The header is actually different pieces aligned to 2048 bytes for historical cd drive reasons..."
    pub const SECTOR_SIZE: usize = 0x800;

    /// Entry data is compressed
    pub const FLAG_COMPRESSED: i32 = 1;
    /// If NOT compressed: Remove padding between files in data block (each file is sector-aligned by default).
    /// If IS compressed: Compress entire data block instead files individually. Padding is not removed.
    pub const FLAG_CONDENSED: i32 = 2;

    pub const fn is_compressed(&self) -> bool {
        self.flags & Self::FLAG_COMPRESSED != 0
    }

    /// I have no clue what "condensing" means
    pub const fn is_condensed(&self) -> bool {
        self.flags & Self::FLAG_CONDENSED != 0
    }

    /// Offset of file entries relative to header
    pub const fn off_entry_block(&self) -> usize {
        Self::SECTOR_SIZE
    }

    /// Offset of file stems relative to header
    pub const fn off_stem_block(&self) -> usize {
        self.off_entry_block() + self.align_sector(self.len_entries as usize)
    }

    /// Offset of file extensions relative to header
    pub const fn off_ext_block(&self) -> usize {
        self.off_stem_block() + self.align_sector(self.len_stems as usize)
    }

    /// Offset of data relative to header
    pub const fn off_data_block(&self) -> usize {
        self.off_ext_block() + self.align_sector(self.len_exts as usize)
    }

    pub const fn align_sector(&self, offset: usize) -> usize {
        offset.div_ceil(Self::SECTOR_SIZE) * Self::SECTOR_SIZE
    }

    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let magic = read_i32_le(buf, 0);
        if magic != Self::SIGNATURE {
            return Err(VolitionError::InvalidPackfileSignature(magic));
        }

        let version = read_i32_le(buf, 0x4);
        if version != Self::VERSION {
            return Err(VolitionError::UnknownPackfileVersion(version));
        }

        let len_compressed = read_i32_le(buf, 0x16c);
        if len_compressed != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "Packfile::len_compressed",
                expected: -1,
                got: len_compressed,
            });
        }

        let open_count_maybe = read_i32_le(buf, 0x17c);
        if open_count_maybe != 0 {
            return Err(VolitionError::ExpectedExactValue {
                field: "Packfile::open_count_maybe",
                expected: 0,
                got: open_count_maybe,
            });
        }

        Ok(Self {
            magic,
            version,
            packfile_name_bufs: read_bytes(buf, 0x08),
            flags: read_i32_le(buf, 0x14c),
            sector: read_i32_le(buf, 0x150),
            num_files: read_i32_le(buf, 0x154),
            len_packfile: read_i32_le(buf, 0x158),
            len_entries: read_i32_le(buf, 0x15c),
            len_stems: read_i32_le(buf, 0x160),
            len_exts: read_i32_le(buf, 0x164),
            len_data: read_i32_le(buf, 0x168),
            len_compressed,
            ptr_entries: read_i32_le(buf, 0x170),
            ptr_names: read_i32_le(buf, 0x174),
            ptr_data: read_i32_le(buf, 0x178),
            open_count: open_count_maybe,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(size_of::<Self>());
        bytes.extend_from_slice(&Self::SIGNATURE.to_le_bytes());
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.packfile_name_bufs);
        bytes.extend_from_slice(&self.flags.to_le_bytes());
        bytes.extend_from_slice(&self.sector.to_le_bytes());
        bytes.extend_from_slice(&self.num_files.to_le_bytes());
        bytes.extend_from_slice(&self.len_packfile.to_le_bytes());
        bytes.extend_from_slice(&self.len_entries.to_le_bytes());
        bytes.extend_from_slice(&self.len_stems.to_le_bytes());
        bytes.extend_from_slice(&self.len_exts.to_le_bytes());
        bytes.extend_from_slice(&self.len_data.to_le_bytes());
        bytes.extend_from_slice(&self.len_compressed.to_le_bytes());
        bytes.extend_from_slice(&self.ptr_entries.to_le_bytes());
        bytes.extend_from_slice(&self.ptr_names.to_le_bytes());
        bytes.extend_from_slice(&self.ptr_data.to_le_bytes());
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes
    }

    pub fn read_entries(&self, buf: &[u8]) -> Result<Vec<PackfileEntry>, VolitionError> {
        let mut entries = Vec::with_capacity(self.num_files as usize);
        for i in 0..self.num_files as usize {
            let off = Self::SECTOR_SIZE + i * size_of::<PackfileEntry>();
            entries.push(PackfileEntry::from_data(&buf[off..])?);
        }
        Ok(entries)
    }

    pub fn read_filenames(
        &self,
        buf: &[u8],
        entries: &[PackfileEntry],
    ) -> Result<Vec<String>, VolitionError> {
        assert_eq!(entries.len(), self.num_files as usize);
        let mut names = Vec::with_capacity(self.num_files as usize);
        for entry in entries.iter() {
            let stem = read_cstr(buf, self.off_stem_block() + entry.off_stem as usize)?;
            let ext = read_cstr(buf, self.off_ext_block() + entry.off_ext as usize)?;
            names.push(format!("{stem}.{ext}"));
        }
        Ok(names)
    }

    pub fn entry_data<'a>(
        &self,
        buf: &'a [u8],
        entry: &PackfileEntry,
    ) -> Result<&'a [u8], VolitionError> {
        if self.is_compressed() {
            return Err(VolitionError::PackfileCompression);
        }

        let start = self.off_data_block() + entry.off_data as usize;
        let end = start + entry.len_data as usize;

        if buf.len() < end {
            Err(VolitionError::BufferTooSmall {
                for_what: "packfile entry contents",
                need: end,
                avail: buf.len(),
            })
        } else {
            Ok(&buf[start..end])
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PackfileEntry {
    /// Offset of file stem (name without extension) in stem block
    pub off_stem: i32,
    /// Offset of file extension in extension block
    pub off_ext: i32,
    pub unknown_08: i32,
    /// Offset of data in data block
    pub off_data: i32,
    pub len_data: i32,
    /// Always -1
    pub runtime_14: i32,
    /// Always Zero
    pub runtime_18: i32,
}

impl Default for PackfileEntry {
    fn default() -> Self {
        Self {
            off_stem: 0,
            off_ext: 0,
            unknown_08: 0,
            off_data: 0,
            len_data: 0,
            runtime_14: -1,
            runtime_18: 0,
        }
    }
}

impl PackfileEntry {
    pub fn from_data(buf: &[u8]) -> Result<Self, VolitionError> {
        check_fits_buf::<Self>(buf)?;

        let runtime_14 = read_i32_le(buf, 0x14);
        let runtime_18 = read_i32_le(buf, 0x18);

        if runtime_14 != -1 {
            return Err(VolitionError::ExpectedExactValue {
                field: "PackfileEntry::runtime_14",
                expected: -1,
                got: runtime_14,
            });
        }

        if runtime_18 != 0 {
            return Err(VolitionError::ExpectedExactValue {
                field: "PackfileEntry::runtime_18",
                expected: 0,
                got: runtime_18,
            });
        }

        Ok(Self {
            off_stem: read_i32_le(buf, 0x0),
            off_ext: read_i32_le(buf, 0x4),
            unknown_08: read_i32_le(buf, 0x8),
            off_data: read_i32_le(buf, 0xc),
            len_data: read_i32_le(buf, 0x10),
            runtime_14,
            runtime_18,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(size_of::<Self>());
        bytes.extend_from_slice(&self.off_stem.to_le_bytes());
        bytes.extend_from_slice(&self.off_ext.to_le_bytes());
        bytes.extend_from_slice(&self.unknown_08.to_le_bytes());
        bytes.extend_from_slice(&self.off_data.to_le_bytes());
        bytes.extend_from_slice(&self.len_data.to_le_bytes());
        bytes.extend_from_slice(&(-1_i32).to_le_bytes());
        bytes.extend_from_slice(&0_i32.to_le_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packfile_size() {
        assert_eq!(size_of::<Packfile>(), 0x180)
    }

    #[test]
    fn test_packfile_serialize_size() {
        assert_eq!(Packfile::default().to_bytes().len(), size_of::<Packfile>())
    }

    #[test]
    fn test_packfile_entry_size() {
        assert_eq!(size_of::<PackfileEntry>(), 28)
    }

    #[test]
    fn test_packfile_entry_serialize_size() {
        assert_eq!(
            PackfileEntry::default().to_bytes().len(),
            size_of::<PackfileEntry>()
        )
    }
}
