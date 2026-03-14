# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

rtcli is a modern, portable CLI for controlling rtorrent via its JSON-RPC interface over SCGI. It supports both TCP and Unix socket connections. Written in Rust (edition 2024).

## Build & Development Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo run -- --url <url> list          # List torrents
cargo run -- --url <url> show <hash>   # Show torrent details
```

No tests exist yet. No linter or formatter configuration beyond standard `cargo fmt` and `cargo clippy`.

## Architecture

The data flow is linear:

```
CLI args → main.rs command routing → rpc.rs JSON-RPC calls → scgi.rs protocol transport → rtorrent daemon
                                                                        ↓
                          Terminal (table/JSON) ← format.rs ← torrent.rs parsing
```

**Key modules:**

- **cli.rs** — Clap-derived argument structs. Connection URL comes from `--url` flag or `RTCLI_URL` env var.
- **rpc.rs** — JSON-RPC 2.0 client. Uses rtorrent's `d.multicall2`, `f.multicall`, `p.multicall`, `t.multicall` methods. `resolve_hash()` supports hash prefix matching.
- **scgi.rs** — Raw SCGI protocol implementation. Detects TCP vs Unix socket from URL format and handles framing/response parsing.
- **torrent.rs** — Data models (`Torrent`, `TorrentFile`, `Peer`, `Tracker`, `Status`) and parsers that convert JSON-RPC response arrays into typed structs.
- **format.rs** — Terminal output: human-readable tables via `comfy-table` with color support, or JSON passthrough. Byte/rate/ratio formatting helpers.
- **error.rs** — `thiserror`-based error enum covering I/O, JSON, SCGI, RPC, and hash resolution errors.

**Output modes:** Every command supports `--json` for machine-readable output; default is colored ASCII tables.

## Distribution

Releases are built via cargo-dist (v0.31.0) with GitHub Actions. Targets: x86_64/aarch64 Linux (GNU + musl), x86_64/aarch64 macOS. Triggered by version tags.
