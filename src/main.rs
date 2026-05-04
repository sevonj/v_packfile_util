use std::collections::HashMap;
use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use v_packfile_util::Packfile;
use v_packfile_util::PackfileEntry;
use v_packfile_util::VolitionError;

#[derive(Parser)]
#[command(author, version, about = "Volition packfile tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Unpack a vpp archive
    Unpack {
        input_file: PathBuf,
        /// Defaults to folder next to input file
        output_dir: Option<PathBuf>,
    },
    /// Create a vpp archive from a directory
    Pack {
        input_dir: PathBuf,
        /// Defaults to file next to input dir
        output_file: Option<PathBuf>,
        #[arg(short, long)]
        compress: bool,
        #[arg(short('d'), long)]
        condense: bool,
    },
}

fn main() {
    match Cli::parse().command {
        Commands::Unpack {
            input_file,
            output_dir,
        } => {
            if let Err(e) = unpack(input_file, output_dir) {
                println!("{e}");
            }
        }
        Commands::Pack {
            input_dir,
            output_file,
            compress,
            condense,
        } => {
            pack(input_dir, output_file, compress, condense);
        }
    }
}

fn unpack(input_file: PathBuf, output_dir: Option<PathBuf>) -> Result<(), VolitionError> {
    let buf = std::fs::read(&input_file).unwrap();

    let out_dir = output_dir.unwrap_or_else(|| {
        let stem = input_file.file_stem().unwrap().to_str().unwrap();
        input_file
            .parent()
            .unwrap()
            .join(format!("{stem}_extracted"))
    });

    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).unwrap();

    let packfile = Packfile::from_data(&buf).unwrap();

    let entries = packfile.read_entries(&buf).unwrap();
    let filenames = packfile.read_filenames(&buf, &entries).unwrap();

    println!("{packfile:?}");

    assert_eq!(filenames.len(), entries.len());

    for (entry, filename) in entries.iter().zip(filenames) {
        std::fs::write(out_dir.join(filename), packfile.entry_data(&buf, entry)?).unwrap();
    }

    Ok(())
}

fn pack(input_dir: PathBuf, output_file: Option<PathBuf>, compress: bool, condense: bool) {
    if compress {
        panic!("{}", VolitionError::PackfileCompression);
    }

    let out_file = output_file.unwrap_or_else(|| {
        let stem = input_dir.file_stem().unwrap().to_str().unwrap();
        input_dir
            .parent()
            .unwrap()
            .join(format!("{stem}_packed"))
            .with_extension("vpp_pc")
    });

    let mut paths = vec![];
    for result in std::fs::read_dir(input_dir).unwrap() {
        let entry = result.unwrap();
        let path = entry.path();
        if entry.metadata().unwrap().is_dir() {
            println!("Warn: ignoring subdirectory: {path:?}");
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
    let (mut stem_block, mut ext_block) = generate_filename_blocks(&paths, &mut entries);

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

    let mut packed_data = packfile.to_bytes();
    packed_data.append(&mut vec![
        0;
        packfile.align_sector(packed_data.len())
            - packed_data.len()
    ]);
    for entry in entries {
        packed_data.append(&mut entry.to_bytes());
    }
    packed_data.append(&mut vec![
        0;
        packfile.align_sector(packed_data.len())
            - packed_data.len()
    ]);
    packed_data.append(&mut stem_block);
    packed_data.append(&mut vec![
        0;
        packfile.align_sector(packed_data.len())
            - packed_data.len()
    ]);
    packed_data.append(&mut ext_block);
    packed_data.append(&mut vec![
        0;
        packfile.align_sector(packed_data.len())
            - packed_data.len()
    ]);
    packed_data.append(&mut data_block);
    packed_data.append(&mut vec![
        0;
        packfile.align_sector(packed_data.len())
            - packed_data.len()
    ]);

    std::fs::write(out_file, packed_data).unwrap();
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
