use magenc::cid::Cid;
use magenc::cli::{Cli, Commands, Parser};
use magenc::magnet::MagnetLink;
use magenc::server::{ServerState, serve};
use reqwest;
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
    for url in mag.xs {
        match reqwest::blocking::get(url) {
            Ok(response) => {
                let bytes = response.bytes().expect("Unable to read response body");
                let cid = Cid::of(&bytes);

                if mag.cid != cid {
                    eprintln!("Response bytes do not match CID. Trying next URL.");
                    continue;
                }

                io::stdout()
                    .write_all(&bytes)
                    .expect("Unable to write to stdout");

                return;
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        };
    }
    eprintln!("Resource not found");
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
