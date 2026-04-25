use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rtcli", about = "CLI tool for rtorrent")]
pub struct Cli {
    /// rtorrent SCGI URL (unix socket path or host:port)
    #[arg(long, env = "RTCLI_URL", global = true)]
    pub url: Option<String>,

    /// Maximum table width in columns; 0 = unlimited (default: terminal width)
    #[arg(short = 'w', long, global = true)]
    pub width: Option<u16>,

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
        /// Filter by name (case-insensitive substring match)
        #[arg(long)]
        filter: Option<String>,
        /// Filter by attribute, format KEY=VALUE.
        /// Supported keys: state, active, complete, directory
        #[arg(long = "filter-by", value_name = "KEY=VALUE")]
        filter_by: Vec<String>,
    },
    /// Show torrent details
    Show {
        /// Torrent hash or hash prefix
        hash: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a torrent from a .torrent file
    Add {
        /// Path to .torrent file
        torrent: std::path::PathBuf,
        /// Download directory (overrides rtorrent default)
        #[arg(long)]
        download_location: Option<String>,
        /// Start the torrent immediately
        #[arg(long, default_value_t = false)]
        start: bool,
        /// Force a hash check after adding (ignored when --start is set)
        #[arg(long, default_value_t = false)]
        hash: bool,
    },
    /// Start an existing torrent
    Start {
        /// Torrent hash or hash prefix
        hash: String,
    },
    /// Stop an existing torrent
    Stop {
        /// Torrent hash or hash prefix
        hash: String,
    },
    /// Remove a torrent from rtorrent
    Rm {
        /// Torrent hash or hash prefix
        hash: String,
    },
    /// Force rehash a torrent
    Rehash {
        /// Torrent hash or hash prefix
        hash: String,
    },
    /// Show global statistics
    Stats {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
