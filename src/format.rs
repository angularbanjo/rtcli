use std::io::IsTerminal;

use comfy_table::{Cell, Color, Table};
use terminal_size::{Width, terminal_size};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::torrent::{Peer, Status, Torrent, TorrentFile, Tracker};

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    const TIB: f64 = GIB * 1024.0;

    let b = bytes as f64;
    if b >= TIB {
        format!("{:.1} TiB", b / TIB)
    } else if b >= GIB {
        format!("{:.1} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.1} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn format_rate(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        return "0".to_string();
    }
    format!("{}/s", format_bytes(bytes_per_sec))
}

fn format_ratio(ratio_thousandths: u64) -> String {
    format!("{:.3}", ratio_thousandths as f64 / 1000.0)
}

fn status_color(status: &Status) -> Color {
    match status {
        Status::Seeding => Color::Green,
        Status::Downloading => Color::Blue,
        Status::Stopped => Color::DarkGrey,
        Status::Hashing => Color::Yellow,
        Status::Error => Color::Red,
    }
}

fn done_percent(completed: u64, total: u64) -> String {
    if total == 0 {
        return "0%".to_string();
    }
    format!("{}%", completed * 100 / total)
}

// Returns the max chars the name column may use before truncation is needed, or None
// if every name already fits without truncation.
//
// comfy_table row width = 1 + n_cols*3 + sum(col_widths), so overhead for 10 cols is 31.
fn name_column_limit(torrents: &[Torrent], term_width: usize) -> Option<usize> {
    const OVERHEAD: usize = 31;

    let size_w = torrents.iter().map(|t| format_bytes(t.size_bytes).len()).max().unwrap_or(0).max("Size".len());
    let status_w = torrents.iter().map(|t| t.status.to_string().len()).max().unwrap_or(0).max("Status".len());
    let done_w = torrents.iter().map(|t| done_percent(t.completed_bytes, t.size_bytes).len()).max().unwrap_or(0).max("Done".len());
    let up_w = torrents.iter().map(|t| format_bytes(t.up_total).len()).max().unwrap_or(0).max("Up".len());
    let down_w = torrents.iter().map(|t| format_bytes(t.down_total).len()).max().unwrap_or(0).max("Down".len());
    let ul_rate_w = torrents.iter().map(|t| format_rate(t.up_rate).len()).max().unwrap_or(0).max("UL Rate".len());
    let dl_rate_w = torrents.iter().map(|t| format_rate(t.down_rate).len()).max().unwrap_or(0).max("DL Rate".len());
    let peers_w = torrents.iter().map(|t| format!("{}/{}", t.peers_connected, t.peers_complete).len()).max().unwrap_or(0).max("Peers".len());
    let fixed_w = 8 + size_w + status_w + done_w + up_w + down_w + ul_rate_w + dl_rate_w + peers_w;

    let natural_name_w = torrents.iter().map(|t| t.name.width()).max().unwrap_or(0).max("Name".len());

    if OVERHEAD + fixed_w + natural_name_w <= term_width {
        return None;
    }

    let available = term_width.saturating_sub(OVERHEAD + fixed_w);
    if available <= "Name".len() {
        return None;
    }

    Some(available)
}

fn truncate_name(name: &str, max_width: usize) -> String {
    if name.width() <= max_width || max_width <= 3 {
        return name.to_string();
    }
    let target = max_width - 3;
    let mut cols = 0;
    let mut truncated = String::new();
    for ch in name.chars() {
        let ch_w = ch.width().unwrap_or(0);
        if cols + ch_w > target {
            break;
        }
        cols += ch_w;
        truncated.push(ch);
    }
    format!("{truncated}...")
}

pub fn print_stats(total: usize, uploaded: u64, downloaded: u64, ratio: f64, seeding_size: u64) {
    println!("Torrents:     {total}");
    println!("Uploaded:     {}", format_bytes(uploaded));
    println!("Downloaded:   {}", format_bytes(downloaded));
    println!("Ratio:        {ratio:.3}");
    println!("Seeding size: {}", format_bytes(seeding_size));
}

pub fn print_torrent_list(torrents: &[Torrent], width_hint: Option<u16>) {
    let use_color = std::io::stdout().is_terminal();

    let effective_width = match width_hint {
        Some(0) => None,
        Some(w) => Some(w as usize),
        None => terminal_size().map(|(Width(w), _)| w as usize),
    };
    let name_limit = effective_width.and_then(|w| name_column_limit(torrents, w));

    let mut table = Table::new();
    table.set_header(vec![
        "Hash", "Name", "Size", "Status", "Done", "Up", "Down", "UL Rate", "DL Rate", "Peers",
    ]);

    for t in torrents {
        let hash_short = &t.hash[..8.min(t.hash.len())];
        let name = match name_limit {
            Some(max_w) => truncate_name(&t.name, max_w),
            None => t.name.clone(),
        };
        let status_text = t.status.to_string();
        let status_cell = if use_color {
            Cell::new(&status_text).fg(status_color(&t.status))
        } else {
            Cell::new(&status_text)
        };

        table.add_row(vec![
            Cell::new(hash_short),
            Cell::new(name),
            Cell::new(format_bytes(t.size_bytes)),
            status_cell,
            Cell::new(done_percent(t.completed_bytes, t.size_bytes)),
            Cell::new(format_bytes(t.up_total)),
            Cell::new(format_bytes(t.down_total)),
            Cell::new(format_rate(t.up_rate)),
            Cell::new(format_rate(t.down_rate)),
            Cell::new(format!("{}/{}", t.peers_connected, t.peers_complete)),
        ]);
    }

    println!("{table}");
}

pub fn print_torrent_detail(t: &Torrent, files: &[TorrentFile], trackers: &[Tracker], peers: &[Peer]) {
    println!("Hash:       {}", t.hash);
    println!("Name:       {}", t.name);
    println!("Status:     {}", t.status);
    println!("Size:       {}", format_bytes(t.size_bytes));
    println!("Piece size: {}", format_bytes(t.piece_size));
    println!("Done:       {}", done_percent(t.completed_bytes, t.size_bytes));
    println!("Downloaded: {}", format_bytes(t.down_total));
    println!("Uploaded:   {}", format_bytes(t.up_total));
    println!("Ratio:      {}", format_ratio(t.ratio));
    println!("DL Rate:    {}", format_rate(t.down_rate));
    println!("UL Rate:    {}", format_rate(t.up_rate));
    println!("Peers:      {}/{}", t.peers_connected, t.peers_complete);
    println!("Directory:  {}", t.directory);
    if !t.message.is_empty() {
        println!("Message:    {}", t.message);
    }

    if !files.is_empty() {
        println!("\nFiles ({}):", files.len());
        let mut table = Table::new();
        table.set_header(vec!["Path", "Size", "Progress", "Priority"]);
        for f in files {
            let progress = if f.size_chunks == 0 {
                "0%".to_string()
            } else {
                format!("{}%", f.completed_chunks * 100 / f.size_chunks)
            };
            table.add_row(vec![
                Cell::new(&f.path),
                Cell::new(format_bytes(f.size_bytes)),
                Cell::new(progress),
                Cell::new(f.priority),
            ]);
        }
        println!("{table}");
    }

    if !trackers.is_empty() {
        println!("\nTrackers ({}):", trackers.len());
        let mut table = Table::new();
        table.set_header(vec!["URL", "Enabled", "Seeds", "Leeches"]);
        for tr in trackers {
            table.add_row(vec![
                Cell::new(&tr.url),
                Cell::new(if tr.enabled { "yes" } else { "no" }),
                Cell::new(tr.scrape_complete),
                Cell::new(tr.scrape_incomplete),
            ]);
        }
        println!("{table}");
    }

    if !peers.is_empty() {
        println!("\nPeers ({}):", peers.len());
        let mut table = Table::new();
        table.set_header(vec!["Address", "Client", "Done", "DL Rate", "UL Rate", "Encrypted"]);
        for p in peers {
            table.add_row(vec![
                Cell::new(&p.address),
                Cell::new(&p.client_version),
                Cell::new(format!("{}%", p.completed_percent)),
                Cell::new(format_rate(p.down_rate)),
                Cell::new(format_rate(p.up_rate)),
                Cell::new(if p.is_encrypted { "yes" } else { "no" }),
            ]);
        }
        println!("{table}");
    }
}
