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

    let mut packfile = Packfile::default();
    if condense {
        packfile.flags |= Packfile::FLAG_CONDENSED;
    }
    packfile.num_files = paths.len() as i32;

    let (mut stem_block, stem_offsets, mut ext_block, ext_offsets) =
        generate_filename_blocks(&paths);

    packfile.len_entries = packfile.num_files * size_of::<PackfileEntry>() as i32;
    packfile.len_stems = stem_block.len() as i32;
    packfile.len_exts = ext_block.len() as i32;

    let mut data_block = vec![];

    let mut entries = Vec::with_capacity(packfile.num_files as usize);
    for ((path, off_stem), off_ext) in paths.into_iter().zip(stem_offsets).zip(ext_offsets) {
        let off_data = data_block.len();
        data_block.append(&mut std::fs::read(path).unwrap());
        let len_data = data_block.len() - off_data;

        if !condense {
            let align = packfile.align_sector(data_block.len()) - data_block.len();
            data_block.append(&mut vec![0; align]);
        }

        entries.push(PackfileEntry {
            off_stem: off_stem as i32,
            off_ext: off_ext as i32,
            unknown_08: 0,
            off_data: off_data as i32,
            len_data: len_data as i32,
            runtime_14: -1,
            runtime_18: 0,
        });
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

/// Generates raw data for filename blocks and offsets to them
/// Return: (stem_block, ext_block, [(off_stem, off_ext)])
fn generate_filename_blocks(paths: &[PathBuf]) -> (Vec<u8>, Vec<usize>, Vec<u8>, Vec<usize>) {
    let mut stems: Vec<String> = vec![];
    let mut extensions: Vec<String> = vec![];
    let mut filename_indices: Vec<(usize, usize)> = Vec::with_capacity(paths.len());

    for path in paths {
        let stem = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let ext = path.extension().unwrap().to_str().unwrap().to_owned();

        let stem_index = stems
            .iter()
            .position(|old| old == &stem)
            .unwrap_or_else(|| {
                stems.push(stem);
                stems.len() - 1
            });

        let ext_index = extensions
            .iter()
            .position(|old| old == &ext)
            .unwrap_or_else(|| {
                extensions.push(ext);
                extensions.len() - 1
            });

        filename_indices.push((stem_index, ext_index));
    }

    let mut stem_block = vec![];
    let mut stem_ind_offsets = vec![];
    for stem in stems {
        stem_ind_offsets.push(stem_block.len());
        stem_block.extend_from_slice(stem.as_bytes());
        stem_block.push(0);
    }

    let mut ext_block = vec![];
    let mut ext_ind_offsets = vec![];
    for ext in extensions {
        ext_ind_offsets.push(ext_block.len());
        ext_block.extend_from_slice(ext.as_bytes());
        ext_block.push(0);
    }

    let mut stem_offsets = vec![];
    let mut ext_offsets = vec![];
    for (stem_index, ext_index) in &filename_indices {
        stem_offsets.push(stem_ind_offsets[*stem_index]);
        ext_offsets.push(ext_ind_offsets[*ext_index]);
    }

    assert_eq!(stem_offsets.len(), ext_offsets.len());
    assert_eq!(stem_offsets.len(), filename_indices.len());

    (stem_block, stem_offsets, ext_block, ext_offsets)
}
