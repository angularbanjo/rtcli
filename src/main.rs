mod cli;
mod config;
mod error;
mod format;
mod rpc;
mod scgi;
mod torrent;

use std::process;

use clap::Parser;
use serde::Serialize;

use cli::{Cli, Command};
use rpc::Client;
use torrent::{Peer, Status, Torrent, TorrentFile, Tracker};

#[derive(Serialize)]
struct TorrentDetail {
    #[serde(flatten)]
    torrent: Torrent,
    files: Vec<TorrentFile>,
    trackers: Vec<Tracker>,
    peers: Vec<Peer>,
}

fn parse_bool_filter(s: &str) -> Option<bool> {
    match s {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

fn parse_status_filter(s: &str) -> Option<Status> {
    match s {
        "stopped" => Some(Status::Stopped),
        "seeding" => Some(Status::Seeding),
        "downloading" => Some(Status::Downloading),
        "hashing" => Some(Status::Hashing),
        "error" => Some(Status::Error),
        _ => None,
    }
}

fn status_eq(a: &Status, b: &Status) -> bool {
    matches!(
        (a, b),
        (Status::Stopped, Status::Stopped)
            | (Status::Seeding, Status::Seeding)
            | (Status::Downloading, Status::Downloading)
            | (Status::Hashing, Status::Hashing)
            | (Status::Error, Status::Error)
    )
}

struct FilterBy {
    key: String,
    value: String,
}

fn parse_filter_by(entries: &[String]) -> error::Result<Vec<FilterBy>> {
    let mut out = Vec::new();
    for entry in entries {
        let (k, v) = entry.split_once('=').ok_or_else(|| {
            error::Error::Scgi(format!(
                "invalid --filter-by value '{entry}': expected KEY=VALUE"
            ))
        })?;
        match k {
            "state" => {
                parse_status_filter(v).ok_or_else(|| {
                    error::Error::Scgi(format!(
                        "invalid state '{v}': expected one of stopped, seeding, downloading, hashing, error"
                    ))
                })?;
            }
            "active" | "complete" => {
                parse_bool_filter(v).ok_or_else(|| {
                    error::Error::Scgi(format!(
                        "invalid boolean '{v}': expected true/false, 1/0, or yes/no"
                    ))
                })?;
            }
            "directory" => {}
            _ => {
                return Err(error::Error::Scgi(format!(
                    "unknown filter key '{k}': supported keys are state, active, complete, directory"
                )));
            }
        }
        out.push(FilterBy { key: k.to_string(), value: v.to_string() });
    }
    Ok(out)
}

fn apply_filters(torrents: Vec<Torrent>, filter: Option<&str>, filters: &[FilterBy]) -> Vec<Torrent> {
    torrents
        .into_iter()
        .filter(|t| {
            if let Some(needle) = filter {
                if !t.name.to_lowercase().contains(&needle.to_lowercase()) {
                    return false;
                }
            }
            for f in filters {
                let pass = match f.key.as_str() {
                    "state" => {
                        let wanted = parse_status_filter(&f.value).unwrap();
                        status_eq(&t.status, &wanted)
                    }
                    "active" => {
                        let wanted = parse_bool_filter(&f.value).unwrap();
                        (t.is_active == 1) == wanted
                    }
                    "complete" => {
                        let wanted = parse_bool_filter(&f.value).unwrap();
                        (t.complete == 1) == wanted
                    }
                    "directory" => t.directory.to_lowercase().contains(&f.value.to_lowercase()),
                    _ => true,
                };
                if !pass {
                    return false;
                }
            }
            true
        })
        .collect()
}

fn cmd_list(
    client: &Client,
    json: bool,
    filter: Option<&str>,
    filter_by: &[String],
    width: Option<u16>,
) -> error::Result<()> {
    let filters = parse_filter_by(filter_by)?;
    let torrents = apply_filters(client.list_torrents()?, filter, &filters);
    if json {
        println!("{}", serde_json::to_string_pretty(&torrents)?);
    } else {
        format::print_torrent_list(&torrents, width);
    }
    Ok(())
}

fn cmd_add(
    client: &Client,
    torrent_path: &std::path::Path,
    download_location: Option<&str>,
    start: bool,
    rehash: bool,
) -> error::Result<()> {
    let data = std::fs::read(torrent_path)?;
    client.add_torrent(&data, download_location, start, rehash)?;
    println!("Torrent added.");
    Ok(())
}

fn cmd_show(client: &Client, hash_prefix: &str, json: bool, width: Option<u16>) -> error::Result<()> {
    let hash = match client.resolve_hash(hash_prefix) {
        Ok(h) => h,
        Err(error::Error::AmbiguousMatch(_)) => {
            let prefix_upper = hash_prefix.to_uppercase();
            let matches: Vec<Torrent> = client
                .list_torrents()?
                .into_iter()
                .filter(|t| t.hash.to_uppercase().starts_with(&prefix_upper))
                .collect();
            format::print_torrent_list(&matches, width);
            return Err(error::Error::AmbiguousMatch(hash_prefix.to_string()));
        }
        Err(e) => return Err(e),
    };
    let torrents = client.list_torrents()?;
    let torrent = torrents
        .into_iter()
        .find(|t| t.hash == hash)
        .ok_or_else(|| error::Error::NoMatch(hash_prefix.to_string()))?;
    let files = client.get_files(&hash)?;
    let peers = client.get_peers(&hash)?;
    let trackers = client.get_trackers(&hash)?;

    if json {
        let detail = TorrentDetail {
            torrent,
            files,
            trackers,
            peers,
        };
        println!("{}", serde_json::to_string_pretty(&detail)?);
    } else {
        format::print_torrent_detail(&torrent, &files, &trackers, &peers);
    }
    Ok(())
}

fn cmd_start(client: &Client, hash_prefix: &str, width: Option<u16>) -> error::Result<()> {
    let hash = match client.resolve_hash(hash_prefix) {
        Ok(h) => h,
        Err(error::Error::AmbiguousMatch(_)) => {
            let prefix_upper = hash_prefix.to_uppercase();
            let matches: Vec<Torrent> = client
                .list_torrents()?
                .into_iter()
                .filter(|t| t.hash.to_uppercase().starts_with(&prefix_upper))
                .collect();
            format::print_torrent_list(&matches, width);
            return Err(error::Error::AmbiguousMatch(hash_prefix.to_string()));
        }
        Err(e) => return Err(e),
    };
    client.start_torrent(&hash)?;
    println!("Torrent started.");
    Ok(())
}

fn cmd_rm(client: &Client, hash_prefix: &str, width: Option<u16>) -> error::Result<()> {
    let hash = match client.resolve_hash(hash_prefix) {
        Ok(h) => h,
        Err(error::Error::AmbiguousMatch(_)) => {
            let prefix_upper = hash_prefix.to_uppercase();
            let matches: Vec<Torrent> = client
                .list_torrents()?
                .into_iter()
                .filter(|t| t.hash.to_uppercase().starts_with(&prefix_upper))
                .collect();
            format::print_torrent_list(&matches, width);
            return Err(error::Error::AmbiguousMatch(hash_prefix.to_string()));
        }
        Err(e) => return Err(e),
    };
    client.remove_torrent(&hash)?;
    println!("Torrent removed.");
    Ok(())
}

fn cmd_rehash(client: &Client, hash_prefix: &str, width: Option<u16>) -> error::Result<()> {
    let hash = match client.resolve_hash(hash_prefix) {
        Ok(h) => h,
        Err(error::Error::AmbiguousMatch(_)) => {
            let prefix_upper = hash_prefix.to_uppercase();
            let matches: Vec<Torrent> = client
                .list_torrents()?
                .into_iter()
                .filter(|t| t.hash.to_uppercase().starts_with(&prefix_upper))
                .collect();
            format::print_torrent_list(&matches, width);
            return Err(error::Error::AmbiguousMatch(hash_prefix.to_string()));
        }
        Err(e) => return Err(e),
    };
    client.rehash_torrent(&hash)?;
    println!("Torrent rehash started.");
    Ok(())
}

fn cmd_stop(client: &Client, hash_prefix: &str, width: Option<u16>) -> error::Result<()> {
    let hash = match client.resolve_hash(hash_prefix) {
        Ok(h) => h,
        Err(error::Error::AmbiguousMatch(_)) => {
            let prefix_upper = hash_prefix.to_uppercase();
            let matches: Vec<Torrent> = client
                .list_torrents()?
                .into_iter()
                .filter(|t| t.hash.to_uppercase().starts_with(&prefix_upper))
                .collect();
            format::print_torrent_list(&matches, width);
            return Err(error::Error::AmbiguousMatch(hash_prefix.to_string()));
        }
        Err(e) => return Err(e),
    };
    client.stop_torrent(&hash)?;
    println!("Torrent stopped.");
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let cfg = config::load_config();
    let url = cli.url.or(cfg.url).unwrap_or_else(|| {
        eprintln!("error: --url is required (or set RTCLI_URL or configure ~/.config/rtcli/config.toml)");
        process::exit(2);
    });
    let client = Client::new(url);

    let width = cli.width;
    let result = match cli.command {
        Command::List { json, filter, filter_by } => {
            cmd_list(&client, json, filter.as_deref(), &filter_by, width)
        }
        Command::Show { hash, json } => cmd_show(&client, &hash, json, width),
        Command::Add { torrent, download_location, start, hash } => {
            cmd_add(&client, &torrent, download_location.as_deref(), start, hash)
        }
        Command::Start { hash } => cmd_start(&client, &hash, width),
        Command::Stop { hash } => cmd_stop(&client, &hash, width),
        Command::Rm { hash } => cmd_rm(&client, &hash, width),
        Command::Rehash { hash } => cmd_rehash(&client, &hash, width),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
