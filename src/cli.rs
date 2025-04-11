pub use clap::Parser;
use clap::Subcommand;

#[derive(Parser, Debug)]
#[command(name = "Magenc")]
#[command(about = "Decentralize HTTP!")]
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

    Post {
        #[arg(help = "File to post")]
        #[arg(value_name = "FILE")]
        file: String,
    },

    #[command(about = "Serve content addressed files over HTTP")]
    Serve {
        #[arg(help = "Directory to serve")]
        #[arg(value_name = "DIRECTORY")]
        #[arg(default_value = "public")]
        dir: String,

        #[arg(help = "Address to listen on")]
        #[arg(value_name = "ADDRESS")]
        #[arg(default_value = "0.0.0.0:3000")]
        addr: String,
    },
}
