mod cli;
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
use torrent::{Peer, Torrent, TorrentFile, Tracker};

#[derive(Serialize)]
struct TorrentDetail {
    #[serde(flatten)]
    torrent: Torrent,
    files: Vec<TorrentFile>,
    trackers: Vec<Tracker>,
    peers: Vec<Peer>,
}

fn cmd_list(client: &Client, json: bool) -> error::Result<()> {
    let torrents = client.list_torrents()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&torrents)?);
    } else {
        format::print_torrent_list(&torrents);
    }
    Ok(())
}

fn cmd_show(client: &Client, hash_prefix: &str, json: bool) -> error::Result<()> {
    let hash = client.resolve_hash(hash_prefix)?;
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

fn main() {
    let cli = Cli::parse();
    let url = match cli.url {
        Some(u) => u,
        None => {
            eprintln!("error: --url is required (or set RTCLI_URL)");
            process::exit(2);
        }
    };
    let client = Client::new(url);

    let result = match cli.command {
        Command::List { json } => cmd_list(&client, json),
        Command::Show { hash, json } => cmd_show(&client, &hash, json),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
