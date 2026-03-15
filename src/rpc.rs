use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::scgi;
use crate::torrent::{
    Peer, Torrent, TorrentFile, Tracker, parse_file, parse_peer, parse_torrent, parse_tracker,
};

pub struct Client {
    url: String,
}

impl Client {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub fn call(&self, method: &str, params: Vec<Value>) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let body = serde_json::to_vec(&request)?;
        let response_bytes = scgi::send_request(&self.url, &body)?;
        let response: Value = serde_json::from_slice(&response_bytes)?;

        if let Some(err) = response.get("error") {
            return Err(Error::Rpc {
                code: err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1),
                message: err
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error")
                    .to_string(),
            });
        }

        response
            .get("result")
            .cloned()
            .ok_or_else(|| Error::Scgi("missing 'result' in JSON-RPC response".into()))
    }

    pub fn list_torrents(&self) -> Result<Vec<Torrent>> {
        let result = self.call(
            "d.multicall2",
            vec![
                json!(""),
                json!("main"),
                json!("d.hash="),
                json!("d.name="),
                json!("d.size_bytes="),
                json!("d.completed_bytes="),
                json!("d.up.total="),
                json!("d.down.total="),
                json!("d.up.rate="),
                json!("d.down.rate="),
                json!("d.peers_connected="),
                json!("d.peers_complete="),
                json!("d.state="),
                json!("d.is_active="),
                json!("d.is_open="),
                json!("d.complete="),
                json!("d.ratio="),
                json!("d.directory="),
                json!("d.message="),
            ],
        )?;

        let rows = result
            .as_array()
            .ok_or_else(|| Error::Scgi("expected array from d.multicall2".into()))?;

        rows.iter()
            .map(|row| {
                let arr = row
                    .as_array()
                    .ok_or_else(|| Error::Scgi("expected array row".into()))?;
                parse_torrent(arr)
            })
            .collect()
    }

    pub fn resolve_hash(&self, prefix: &str) -> Result<String> {
        let result = self.call("download_list", vec![json!("")])?;
        let hashes = result
            .as_array()
            .ok_or_else(|| Error::Scgi("expected array from download_list".into()))?;

        let prefix_upper = prefix.to_uppercase();
        let matches: Vec<String> = hashes
            .iter()
            .filter_map(|v| v.as_str())
            .filter(|h| h.to_uppercase().starts_with(&prefix_upper))
            .map(|h| h.to_string())
            .collect();

        match matches.len() {
            0 => Err(Error::NoMatch(prefix.to_string())),
            1 => Ok(matches.into_iter().next().unwrap()),
            _ => Err(Error::AmbiguousMatch(prefix.to_string())),
        }
    }

    pub fn add_torrent(
        &self,
        data: &[u8],
        download_location: Option<&str>,
        start: bool,
    ) -> Result<()> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let data_uri = format!("data:application/octet-stream;base64,{}", STANDARD.encode(data));

        let method = if start { "load.raw_start_verbose" } else { "load.raw_verbose" };

        let mut params: Vec<Value> = vec![json!(""), json!(data_uri)];
        if let Some(dir) = download_location {
            params.push(json!(format!("d.directory.set={dir}")));
        }

        self.call(method, params)?;
        Ok(())
    }

    pub fn start_torrent(&self, hash: &str) -> Result<()> {
        self.call("d.start", vec![json!(hash)])?;
        Ok(())
    }

    pub fn stop_torrent(&self, hash: &str) -> Result<()> {
        self.call("d.stop", vec![json!(hash)])?;
        Ok(())
    }

    pub fn get_files(&self, hash: &str) -> Result<Vec<TorrentFile>> {
        let result = self.call(
            "f.multicall",
            vec![
                json!(hash),
                json!(""),
                json!("f.path="),
                json!("f.size_bytes="),
                json!("f.completed_chunks="),
                json!("f.size_chunks="),
                json!("f.priority="),
            ],
        )?;

        let rows = result
            .as_array()
            .ok_or_else(|| Error::Scgi("expected array from f.multicall".into()))?;

        rows.iter()
            .map(|row| {
                let arr = row
                    .as_array()
                    .ok_or_else(|| Error::Scgi("expected array row".into()))?;
                parse_file(arr)
            })
            .collect()
    }

    pub fn get_peers(&self, hash: &str) -> Result<Vec<Peer>> {
        let result = self.call(
            "p.multicall",
            vec![
                json!(hash),
                json!(""),
                json!("p.address="),
                json!("p.client_version="),
                json!("p.completed_percent="),
                json!("p.down_rate="),
                json!("p.up_rate="),
                json!("p.is_encrypted="),
                json!("p.is_incoming="),
            ],
        )?;

        let rows = result
            .as_array()
            .ok_or_else(|| Error::Scgi("expected array from p.multicall".into()))?;

        rows.iter()
            .map(|row| {
                let arr = row
                    .as_array()
                    .ok_or_else(|| Error::Scgi("expected array row".into()))?;
                parse_peer(arr)
            })
            .collect()
    }

    pub fn get_trackers(&self, hash: &str) -> Result<Vec<Tracker>> {
        let result = self.call(
            "t.multicall",
            vec![
                json!(hash),
                json!(""),
                json!("t.url="),
                json!("t.is_enabled="),
                json!("t.is_open="),
                json!("t.scrape_complete="),
                json!("t.scrape_incomplete="),
            ],
        )?;

        let rows = result
            .as_array()
            .ok_or_else(|| Error::Scgi("expected array from t.multicall".into()))?;

        rows.iter()
            .map(|row| {
                let arr = row
                    .as_array()
                    .ok_or_else(|| Error::Scgi("expected array row".into()))?;
                parse_tracker(arr)
            })
            .collect()
    }
}
