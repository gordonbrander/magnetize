use magenc::cid::Cid;
use magenc::cli::{Cli, Commands, Parser};
use magenc::server::{ServerState, serve};
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Get { url } => {
            println!("{:?}", url);
        }
        Commands::Add { file } => {
            cmd_add(file);
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

fn cmd_add(file: Option<PathBuf>) {
    match file {
        Some(file) => cmd_add_file(file),
        None => cmd_add_stdin(),
    }
}

fn cmd_add_file(file: PathBuf) {
    let bytes = fs::read(&file).expect("Unable to read file");
    let cid = Cid::new(&bytes);
    let cid_pathbuf = PathBuf::from(cid.to_string());
    fs::write(&cid_pathbuf, bytes).expect("Unable to write file");
    println!("{}", cid);
}

fn cmd_add_stdin() {
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .expect("Unable to read stdin");
    let cid = Cid::new(&bytes);
    let cid_pathbuf = PathBuf::from(cid.to_string());
    fs::write(&cid_pathbuf, bytes).expect("Unable to write file");
}
