use magnetize::cid::Cid;
use magnetize::cli::{Cli, Commands, Parser};
use magnetize::magnet::{MagnetLink, get_blocking};
use magnetize::server::{ServerState, serve};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Get { url } => cmd_get(&url),
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

fn cmd_get(url: &str) {
    let mag = MagnetLink::parse(url).expect("Unable to parse magnet link");
    let body = get_blocking(&mag).expect("Resource not found");

    io::stdout()
        .write_all(&body)
        .expect("Unable to write to stdout");
}

fn cmd_add(file: Option<PathBuf>) {
    match file {
        Some(file) => cmd_add_file(file),
        None => cmd_add_stdin(),
    }
}

fn cmd_add_file(file: PathBuf) {
    let bytes = fs::read(&file).expect("Unable to read file");
    let cid = Cid::of(&bytes);
    let cid_pathbuf = PathBuf::from(cid.to_string());
    fs::write(&cid_pathbuf, bytes).expect("Unable to write file");
    println!("{}", cid);
}

fn cmd_add_stdin() {
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .expect("Unable to read stdin");
    let cid = Cid::of(&bytes);
    let cid_pathbuf = PathBuf::from(cid.to_string());
    fs::write(&cid_pathbuf, bytes).expect("Unable to write file");
}
