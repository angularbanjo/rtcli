use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rtcli", about = "CLI tool for rtorrent")]
pub struct Cli {
    /// rtorrent SCGI URL (unix socket path or host:port)
    #[arg(long, env = "RTCLI_URL", global = true)]
    pub url: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List all torrents
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show torrent details
    Show {
        /// Torrent hash or hash prefix
        hash: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
