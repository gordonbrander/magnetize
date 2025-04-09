use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Magenc")]
#[command(about = "Decentralize HTTP!")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Get a file from a magnet link")]
    Get {
        #[arg(help = "URL to fetch")]
        #[arg(value_name = "URL")]
        url: String,
    },

    #[command(about = "Serve content addressed files over HTTP")]
    Serve {
        #[arg(help = "Directory to serve")]
        #[arg(value_name = "DIRECTORY")]
        #[arg(default_value = "public")]
        directory: String,

        #[arg(help = "Port to listen on")]
        #[arg(value_name = "PORT")]
        #[arg(default_value = "80")]
        port: u16,
    },
}

fn main() {
    println!("Hello, world!");
}
