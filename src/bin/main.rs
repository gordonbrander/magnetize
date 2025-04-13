use magnetize::cid::Cid;
use magnetize::cli::{Cli, Commands, Parser};
use magnetize::magnet::{MagnetLink, get_blocking};
use magnetize::server::{ServerState, serve};
use reqwest;
use std::collections::HashSet;
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
        Commands::Link { url } => {
            cmd_link(url);
        }
        Commands::Serve {
            dir,
            addr,
            post,
            feds,
        } => {
            serve(ServerState {
                address: addr,
                file_storage_dir: dir,
                allow_post: post,
                feds,
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

fn cmd_link(ws: Vec<String>) {
    let mut cids: HashSet<Cid> = HashSet::new();

    for url in &ws {
        match reqwest::blocking::get(url) {
            Ok(response) => {
                let body = response.bytes().expect("Unable to read response");
                let body_cid = Cid::of(&body);
                cids.insert(body_cid);
            }
            Err(e) => {
                eprintln!("Error fetching URL: {}", e);
            }
        }
    }

    if cids.is_empty() {
        eprintln!("Unable to reach any of the provided URLs");
        return;
    }

    if cids.len() != 1 {
        eprintln!("URLs do not point to the same resource");
        return;
    }

    let cid = cids.into_iter().next().unwrap();

    let mag = MagnetLink {
        cid,
        ws,
        cdn: Vec::new(),
        xt: None,
        dn: None,
    };

    println!("{}", mag.to_string());
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
