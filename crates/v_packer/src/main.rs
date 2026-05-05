use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use v_commands::pack;
use v_commands::unpack;

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
            if let Err(e) = pack(input_dir, output_file, compress, condense) {
                println!("{e}");
            }
        }
    }

    println!("Done.");
}
