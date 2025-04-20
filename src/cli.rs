pub use clap::Parser;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "Magnetize")]
#[command(about = "Content-addressed data over HTTP using magnet links")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum Commands {
    #[command(about = "Get a file from a magnet link")]
    Get {
        #[arg(help = "URL to fetch")]
        #[arg(value_name = "URL")]
        url: String,
    },

    #[command(about = "Create a magnet link from one or more HTTP URLs")]
    Link {
        #[arg(
            long_help = "Create a magnet link from one or more HTTP URLs. Fetches the content, generates a CID, and returns the magnet link."
        )]
        #[arg(value_name = "URL")]
        url: Vec<String>,
    },

    #[command(about = "Add data to current directory. Creates a file using the CID as filename.")]
    Add {
        #[arg(
            help = "File to add. If file is not provided, reads from stdin.",
            value_name = "FILE"
        )]
        file: Option<PathBuf>,
    },

    #[command(about = "Serve content addressed files over HTTP")]
    Serve {
        #[arg(
            help = "Directory to serve. Creates directory if it doesn't already exist.",
            value_name = "DIRECTORY",
            default_value = "public"
        )]
        dir: PathBuf,

        #[arg(
            long,
            help = "Address to listen on",
            value_name = "ADDRESS",
            default_value = "0.0.0.0:3000"
        )]
        addr: String,
    },
}
