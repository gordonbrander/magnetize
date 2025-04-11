use magenc::cli::{Cli, Commands, Parser};
use magenc::server::{ServerState, serve};

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Get { url } => {
            println!("{:?}", url);
        }
        Commands::Post { file } => {
            println!("{:?}", file);
        }
        Commands::Serve { dir, addr, post } => {
            serve(ServerState {
                address: addr,
                file_storage_dir: dir,
                allow_post: post,
            });
        }
    }
}
