use magenc::cli::{Cli, Commands, Parser};
use magenc::server::serve;

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Get { url } => {
            println!("{:?}", url);
        }
        Commands::Post { file } => {
            println!("{:?}", file);
        }
        Commands::Serve { dir, addr } => {
            serve(&addr, dir);
        }
    }
}
