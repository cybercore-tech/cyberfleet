use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// User-editable settings at `~/.config/cyberfleet/config.json`. Roots are
/// stored as strings (not expanded `PathBuf`s) so the file stays readable
/// and portable — `~/` is expanded at scan time, not at save time.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub roots: Vec<String>,
    pub ignore: Vec<String>,
    pub max_depth: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            roots: vec![],
            ignore: default_ignores(),
            max_depth: 4,
        }
    }
}

fn default_ignores() -> Vec<String> {
    [
        "node_modules",
        "target",
        "vendor",
        ".cache",
        "dist",
        "build",
        ".venv",
        "venv",
        "__pycache__",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config/cyberfleet/config.json")
}

/// Loads the config, or — on first run — seeds it by checking a shortlist of
/// conventional dev-directory names for ones that actually exist under
/// `$HOME`, so cyberfleet finds something useful without any setup. Fully
/// editable afterward (by hand, or with `a` in the TUI).
pub fn load_or_init() -> Result<Config> {
    let path = config_path();
    if path.exists() {
        let data = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data)?)
    } else {
        let cfg = Config {
            roots: autodetect_roots(),
            ..Config::default()
        };
        save(&cfg)?;
        Ok(cfg)
    }
}

fn autodetect_roots() -> Vec<String> {
    let Some(home) = dirs::home_dir() else {
        return vec![];
    };
    const CANDIDATES: &[&str] = &[
        "Devspace",
        "dev",
        "Developer",
        "projects",
        "Projects",
        "code",
        "src",
        ".sysops",
        "workspace",
        "Documents/GitHub",
        "repos",
    ];
    CANDIDATES
        .iter()
        .filter(|c| home.join(c).is_dir())
        .map(|c| format!("~/{c}"))
        .collect()
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

/// Expands a leading `~` (only that form — no `~user/...`) against `$HOME`.
/// Config roots always go through this before hitting the filesystem.
pub fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        dirs::home_dir().unwrap_or_default().join(rest)
    } else if p == "~" {
        dirs::home_dir().unwrap_or_default()
    } else {
        PathBuf::from(p)
    }
}
