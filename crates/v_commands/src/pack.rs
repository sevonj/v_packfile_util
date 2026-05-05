use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

use v_types::Packfile;
use v_types::PackfileEntry;
use v_types::VolitionError;

pub fn pack(
    input_dir: PathBuf,
    output_file: Option<PathBuf>,
    compress: bool,
    condense: bool,
) -> Result<(), VolitionError> {
    if compress {
        panic!("{}", VolitionError::PackfileCompression);
    }

    let out_path = output_file.unwrap_or_else(|| {
        let stem = input_dir.file_stem().unwrap().to_str().unwrap();
        input_dir
            .parent()
            .unwrap()
            .join(format!("{stem}_packed"))
            .with_extension("vpp_pc")
    });

    let out_file = File::create(out_path).unwrap();
    let mut writer = BufWriter::new(out_file);

    let mut paths = vec![];
    for result in std::fs::read_dir(input_dir).unwrap() {
        let entry = result.unwrap();
        let path = entry.path();
        if entry.metadata().unwrap().is_dir() {
            println!("Warn: subdirectories will be ignored. (found {path:?})");
            continue;
        }
        paths.push(path);
    }

    let num_files = paths.len();

    let mut packfile = Packfile::default();
    if condense {
        packfile.flags |= Packfile::FLAG_CONDENSED;
    }

    let mut entries = vec![PackfileEntry::default(); num_files];
    let (stem_block, ext_block) = generate_filename_blocks(&paths, &mut entries);

    packfile.num_files = num_files as i32;
    packfile.len_entries = (num_files * size_of::<PackfileEntry>()) as i32;
    packfile.len_stems = stem_block.len() as i32;
    packfile.len_exts = ext_block.len() as i32;

    let mut data_block = vec![];

    for (entry, path) in entries.iter_mut().zip(paths) {
        let off_data = data_block.len();
        data_block.append(&mut std::fs::read(path).unwrap());
        let len_data = data_block.len() - off_data;

        if !condense {
            let align = packfile.align_sector(data_block.len()) - data_block.len();
            data_block.append(&mut vec![0; align]);
        }

        entry.off_data = off_data as i32;
        entry.len_data = len_data as i32;
    }

    packfile.len_data = data_block.len() as i32;
    packfile.len_packfile = (packfile.off_data_block() + data_block.len()) as i32;

    let mut entry_block = Vec::with_capacity(packfile.len_entries as usize);
    for entry in entries {
        entry_block.append(&mut entry.to_bytes());
    }

    write_sector_aligned(&mut writer, &packfile.to_bytes(), &packfile)?;
    write_sector_aligned(&mut writer, &entry_block, &packfile)?;
    write_sector_aligned(&mut writer, &stem_block, &packfile)?;
    write_sector_aligned(&mut writer, &ext_block, &packfile)?;
    write_sector_aligned(&mut writer, &data_block, &packfile)?;

    writer.flush()?;

    Ok(())
}

/// Generate filename blocks and patch their offsets in entries
/// Return: (stem_block, ext_block)
fn generate_filename_blocks(
    paths: &[PathBuf],
    entries: &mut [PackfileEntry],
) -> (Vec<u8>, Vec<u8>) {
    assert_eq!(paths.len(), entries.len());

    let mut stem_offs = HashMap::new();
    let mut ext_offs = HashMap::new();
    let mut stem_block = vec![];
    let mut ext_block = vec![];

    for (path, entry) in paths.iter().zip(entries) {
        let stem = path.file_stem().unwrap().to_str().unwrap().to_owned();
        entry.off_stem = if let Some(off) = stem_offs.get(&stem) {
            *off
        } else {
            let off = stem_block.len() as i32;
            stem_block.extend_from_slice(stem.as_bytes());
            stem_block.push(0);
            stem_offs.insert(stem, off);
            off
        };

        let ext = path.extension().unwrap().to_str().unwrap().to_owned();
        entry.off_ext = if let Some(off) = ext_offs.get(&ext) {
            *off
        } else {
            let off = ext_block.len() as i32;
            ext_block.extend_from_slice(ext.as_bytes());
            ext_block.push(0);
            ext_offs.insert(ext, off);
            off
        };
    }

    (stem_block, ext_block)
}

fn write_sector_aligned(
    w: &mut impl Write,
    data: &[u8],
    packfile: &Packfile,
) -> std::io::Result<()> {
    w.write_all(data)?;
    let padding = packfile.align_sector(data.len()) - data.len();
    w.write_all(&vec![0u8; padding])
}
