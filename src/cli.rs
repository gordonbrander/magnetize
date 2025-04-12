pub use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "Magenc")]
#[command(about = "Decentralize HTTP somewhat")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Get a file from a magnet link")]
    Get {
        #[arg(help = "URL to fetch")]
        #[arg(value_name = "URL")]
        url: String,
    },

    Link {
        #[arg(
            help = "Create a magnet link from one or more HTTP URLs. Fetches the content, generates a CID, and returns the magnet link."
        )]
        #[arg(value_name = "URL")]
        url: Vec<String>,
    },

    #[command(about = "Add data to current directory. Creates a file using the CID as filename.")]
    Add {
        #[arg(help = "File to add. If file is not provided, reads from stdin.")]
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,
    },

    #[command(about = "Serve content addressed files over HTTP")]
    Serve {
        #[arg(help = "Directory to serve. Creates directory if it doesn't already exist.")]
        #[arg(value_name = "DIRECTORY")]
        #[arg(default_value = "public")]
        dir: PathBuf,

        #[arg(help = "Address to listen on")]
        #[arg(value_name = "ADDRESS")]
        #[arg(default_value = "0.0.0.0:3000")]
        addr: String,

        #[arg(help = "Allow file uploads via POST?")]
        #[arg(short = 'p', long = "post")]
        post: bool,
    },
}
