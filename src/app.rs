use crate::config::{self, Config};
use crate::scan;
use crate::status::{self, RepoStatus};
use anyhow::Result;

pub enum Mode {
    Normal,
    Filter,
    AddRoot,
    Detail,
    Help,
}

/// Blocking work deferred by one redraw so a key handler never freezes the
/// screen for however long it takes. Same pattern as cyberplug's `Pending`.
/// Notably NOT where `git fetch` lives — see `FetchJob` below for why.
pub enum Pending {
    Rescan,
}

/// `git fetch` against an SSH remote can need a passphrase or a host-key
/// confirmation, and `ssh` asks for those by opening `/dev/tty` directly —
/// which, while the TUI owns the terminal in raw/alternate-screen mode,
/// means the prompt has nowhere sane to go and the whole app hangs with the
/// keystrokes meant for it silently swallowed. So a fetch is never run
/// in-place like `Pending` work: the main loop suspends the TUI entirely
/// (leaves the alternate screen, disables raw mode), runs `git fetch` with
/// the real inherited terminal so any prompt behaves normally, then résumes.
pub enum FetchJob {
    One(usize),
    All,
}

pub struct App {
    pub config: Config,
    pub repos: Vec<RepoStatus>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub filter_text: String,
    pub input_buffer: String,
    pub mode: Mode,
    pub status: Option<String>,
    pub should_quit: bool,
    pub pending: Option<Pending>,
    pub fetch_job: Option<FetchJob>,
    pub detail_text: String,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = config::load_or_init()?;
        Ok(Self {
            config,
            repos: vec![],
            filtered: vec![],
            selected: 0,
            filter_text: String::new(),
            input_buffer: String::new(),
            mode: Mode::Normal,
            status: Some("scanning...".to_string()),
            should_quit: false,
            pending: Some(Pending::Rescan),
            fetch_job: None,
            detail_text: String::new(),
        })
    }

    pub fn perform(&mut self, work: Pending) -> Result<()> {
        match work {
            Pending::Rescan => {
                self.rescan();
                let attention = self.repos.iter().filter(|r| r.needs_attention()).count();
                self.status = Some(format!(
                    "{} repos — {} need attention",
                    self.repos.len(),
                    attention
                ));
            }
        }
        Ok(())
    }

    pub fn rescan(&mut self) {
        let roots: Vec<_> = self
            .config
            .roots
            .iter()
            .map(|r| config::expand_tilde(r))
            .collect();
        let paths = scan::discover_repos(&roots, &self.config.ignore, self.config.max_depth);
        self.repos = paths
            .iter()
            .filter_map(|p| status::repo_status(p).ok())
            .collect();
        self.apply_sort_and_filter();
    }

    /// Needs-attention repos first, then alphabetical — the whole point of
    /// a dashboard is that the thing you should look at is at the top.
    pub fn apply_sort_and_filter(&mut self) {
        self.repos.sort_by(|a, b| {
            b.needs_attention()
                .cmp(&a.needs_attention())
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        let needle = self.filter_text.to_lowercase();
        self.filtered = self
            .repos
            .iter()
            .enumerate()
            .filter(|(_, r)| needle.is_empty() || r.name.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn selected_repo(&self) -> Option<&RepoStatus> {
        self.filtered
            .get(self.selected)
            .and_then(|&i| self.repos.get(i))
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.filtered.get(self.selected).copied()
    }

    pub fn next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = if self.selected == 0 {
                self.filtered.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn add_root(&mut self, raw: &str) -> Result<()> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        if !self.config.roots.iter().any(|r| r == trimmed) {
            self.config.roots.push(trimmed.to_string());
            config::save(&self.config)?;
        }
        Ok(())
    }

    pub fn ignore_selected(&mut self) -> Result<()> {
        if let Some(r) = self.selected_repo() {
            let name = r.name.clone();
            if !self.config.ignore.iter().any(|i| i == &name) {
                self.config.ignore.push(name);
                config::save(&self.config)?;
            }
        }
        Ok(())
    }
}
