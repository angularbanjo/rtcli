use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
pub struct Config {
    pub url: Option<String>,
}

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rtcli").join("config.toml"))
}

pub fn load_config() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Config::default(),
        Err(e) => {
            eprintln!("warning: could not read {}: {e}", path.display());
            return Config::default();
        }
    };
    match toml::from_str(&contents) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warning: could not parse {}: {e}", path.display());
            Config::default()
        }
    }
}
