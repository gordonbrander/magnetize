use magnetize::cid::Cid;
use magnetize::cli::{Cli, Commands, Parser};
use magnetize::magnet::MagnetLink;
use magnetize::request::get_and_check_cid;
use magnetize::server::{ServerConfig, serve};
use magnetize::url::Url;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use tokio::runtime;

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
        Commands::Serve { dir, addr } => {
            serve(ServerConfig { addr, dir });
        }
    }
}

fn cmd_get(url: &str) {
    let mag = MagnetLink::parse(url).expect("Unable to parse magnet link");
    let client = reqwest::Client::new();

    // Create a single-threaded tokio runtime
    let runtime = runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .expect("Unable to create tokio runtime");

    for url in mag.urls() {
        match runtime.block_on(get_and_check_cid(&client, &url, &mag.cid)) {
            Ok(body) => {
                io::stdout()
                    .write_all(&body)
                    .expect("Unable to write to stdout");
                return;
            }
            Err(e) => {
                eprintln!("Error getting URL {}\n\tError: {}", &url, e);
            }
        }
    }

    eprintln!("Resource not found");
}

fn cmd_add(file: Option<PathBuf>) {
    match file {
        Some(file) => cmd_add_file(file),
        None => cmd_add_stdin(),
    }
}

fn cmd_link(ws: Vec<String>) {
    let ws_urls: Vec<Url> = ws
        .iter()
        .map(|s| Url::parse(s).expect("Invalid url"))
        .collect();

    let mut cids: HashSet<Cid> = HashSet::new();

    for url in &ws_urls {
        match reqwest::blocking::get(url.as_str()) {
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
        ws: ws_urls,
        rs: Vec::new(),
        btmh: None,
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
