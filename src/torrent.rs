use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize)]
pub enum Status {
    Seeding,
    Downloading,
    Stopped,
    Hashing,
    Error,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Seeding => write!(f, "Seeding"),
            Status::Downloading => write!(f, "Downloading"),
            Status::Stopped => write!(f, "Stopped"),
            Status::Hashing => write!(f, "Hashing"),
            Status::Error => write!(f, "Error"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Torrent {
    pub hash: String,
    pub name: String,
    pub size_bytes: u64,
    pub completed_bytes: u64,
    pub up_total: u64,
    pub down_total: u64,
    pub up_rate: u64,
    pub down_rate: u64,
    pub peers_connected: u64,
    pub peers_complete: u64,
    pub state: u64,
    pub is_active: u64,
    pub is_open: u64,
    pub complete: u64,
    pub ratio: u64,
    pub directory: String,
    pub message: String,
    pub status: Status,
}

#[derive(Debug, Clone, Serialize)]
pub struct TorrentFile {
    pub path: String,
    pub size_bytes: u64,
    pub completed_chunks: u64,
    pub size_chunks: u64,
    pub priority: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Peer {
    pub address: String,
    pub client_version: String,
    pub completed_percent: u64,
    pub down_rate: u64,
    pub up_rate: u64,
    pub is_encrypted: bool,
    pub is_incoming: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tracker {
    pub url: String,
    pub enabled: bool,
    pub open: bool,
    pub scrape_complete: u64,
    pub scrape_incomplete: u64,
}

fn val_str(arr: &[Value], idx: usize) -> Result<String> {
    arr.get(idx)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Scgi(format!("expected string at index {idx}")))
}

fn val_u64(arr: &[Value], idx: usize) -> Result<u64> {
    arr.get(idx)
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
        .ok_or_else(|| Error::Scgi(format!("expected integer at index {idx}")))
}

fn derive_status(message: &str, complete: u64, is_active: u64, state: u64) -> Status {
    if !message.is_empty() {
        return Status::Error;
    }
    if state == 0 && is_active == 0 {
        return Status::Stopped;
    }
    if complete == 1 && is_active == 1 {
        return Status::Seeding;
    }
    if is_active == 1 {
        return Status::Downloading;
    }
    Status::Stopped
}

pub fn parse_torrent(arr: &[Value]) -> Result<Torrent> {
    let hash = val_str(arr, 0)?;
    let name = val_str(arr, 1)?;
    let size_bytes = val_u64(arr, 2)?;
    let completed_bytes = val_u64(arr, 3)?;
    let up_total = val_u64(arr, 4)?;
    let down_total = val_u64(arr, 5)?;
    let up_rate = val_u64(arr, 6)?;
    let down_rate = val_u64(arr, 7)?;
    let peers_connected = val_u64(arr, 8)?;
    let peers_complete = val_u64(arr, 9)?;
    let state = val_u64(arr, 10)?;
    let is_active = val_u64(arr, 11)?;
    let is_open = val_u64(arr, 12)?;
    let complete = val_u64(arr, 13)?;
    let ratio = val_u64(arr, 14)?;
    let directory = val_str(arr, 15)?;
    let message = val_str(arr, 16)?;
    let status = derive_status(&message, complete, is_active, state);

    Ok(Torrent {
        hash,
        name,
        size_bytes,
        completed_bytes,
        up_total,
        down_total,
        up_rate,
        down_rate,
        peers_connected,
        peers_complete,
        state,
        is_active,
        is_open,
        complete,
        ratio,
        directory,
        message,
        status,
    })
}

pub fn parse_file(arr: &[Value]) -> Result<TorrentFile> {
    Ok(TorrentFile {
        path: val_str(arr, 0)?,
        size_bytes: val_u64(arr, 1)?,
        completed_chunks: val_u64(arr, 2)?,
        size_chunks: val_u64(arr, 3)?,
        priority: val_u64(arr, 4)?,
    })
}

pub fn parse_peer(arr: &[Value]) -> Result<Peer> {
    Ok(Peer {
        address: val_str(arr, 0)?,
        client_version: val_str(arr, 1)?,
        completed_percent: val_u64(arr, 2)?,
        down_rate: val_u64(arr, 3)?,
        up_rate: val_u64(arr, 4)?,
        is_encrypted: val_u64(arr, 5)? != 0,
        is_incoming: val_u64(arr, 6)? != 0,
    })
}

pub fn parse_tracker(arr: &[Value]) -> Result<Tracker> {
    Ok(Tracker {
        url: val_str(arr, 0)?,
        enabled: val_u64(arr, 1)? != 0,
        open: val_u64(arr, 2)? != 0,
        scrape_complete: val_u64(arr, 3)?,
        scrape_incomplete: val_u64(arr, 4)?,
    })
}
