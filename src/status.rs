use anyhow::{Context, Result};
use git2::{Repository, RepositoryState as GitState, Status, StatusOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A repo mid-merge/rebase/etc. needs a human, not just a push. Surfacing
/// this is the one thing plain `git status` buries in a paragraph of prose
/// that's easy to skim past — worth a dedicated, always-visible flag.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepoState {
    Clean,
    Merging,
    Reverting,
    CherryPicking,
    Bisecting,
    Rebasing,
    ApplyingMailbox,
}

impl RepoState {
    pub fn label(self) -> &'static str {
        match self {
            RepoState::Clean => "",
            RepoState::Merging => "MERGING",
            RepoState::Reverting => "REVERTING",
            RepoState::CherryPicking => "CHERRY-PICK",
            RepoState::Bisecting => "BISECT",
            RepoState::Rebasing => "REBASING",
            RepoState::ApplyingMailbox => "AM",
        }
    }
}

impl From<GitState> for RepoState {
    fn from(s: GitState) -> Self {
        match s {
            GitState::Clean => RepoState::Clean,
            GitState::Merge => RepoState::Merging,
            GitState::Revert | GitState::RevertSequence => RepoState::Reverting,
            GitState::CherryPick | GitState::CherryPickSequence => RepoState::CherryPicking,
            GitState::Bisect => RepoState::Bisecting,
            GitState::Rebase | GitState::RebaseInteractive | GitState::RebaseMerge => {
                RepoState::Rebasing
            }
            GitState::ApplyMailbox | GitState::ApplyMailboxOrRebase => RepoState::ApplyingMailbox,
        }
    }
}

pub struct RepoStatus {
    pub path: PathBuf,
    pub name: String,
    /// Branch name, or a short detached-HEAD commit id, or `None` for a
    /// brand new repo with no commits yet.
    pub branch: Option<String>,
    pub detached: bool,
    pub has_upstream: bool,
    pub ahead: usize,
    pub behind: usize,
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicted: usize,
    pub stashes: usize,
    pub state: RepoState,
    pub last_commit_time: Option<i64>,
    pub last_commit_summary: Option<String>,
    pub last_commit_author: Option<String>,
}

impl RepoStatus {
    pub fn is_dirty(&self) -> bool {
        self.staged + self.unstaged + self.untracked + self.conflicted > 0
    }

    /// The one boolean the bar widget's count and the TUI's sort order both
    /// key off: does a human need to look at this repo?
    pub fn needs_attention(&self) -> bool {
        self.is_dirty() || self.ahead > 0 || self.behind > 0 || self.state != RepoState::Clean
    }
}

pub fn repo_status(path: &Path) -> Result<RepoStatus> {
    let mut repo = Repository::open(path).with_context(|| format!("opening {}", path.display()))?;

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    // Scoped so `statuses` (which borrows `repo`) drops before the
    // `&mut repo` calls (stash_foreach, below) that follow it.
    let (staged, unstaged, untracked, conflicted) = {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(false)
            .include_ignored(false);
        let statuses = repo.statuses(Some(&mut opts))?;

        let mut staged = 0;
        let mut unstaged = 0;
        let mut untracked = 0;
        let mut conflicted = 0;
        for entry in statuses.iter() {
            let s = entry.status();
            if s.contains(Status::CONFLICTED) {
                conflicted += 1;
                continue;
            }
            if s.intersects(
                Status::INDEX_NEW
                    | Status::INDEX_MODIFIED
                    | Status::INDEX_DELETED
                    | Status::INDEX_RENAMED
                    | Status::INDEX_TYPECHANGE,
            ) {
                staged += 1;
            }
            if s.intersects(
                Status::WT_MODIFIED
                    | Status::WT_DELETED
                    | Status::WT_RENAMED
                    | Status::WT_TYPECHANGE,
            ) {
                unstaged += 1;
            }
            if s.contains(Status::WT_NEW) {
                untracked += 1;
            }
        }
        (staged, unstaged, untracked, conflicted)
    };

    let mut branch = None;
    let mut detached = false;
    let mut has_upstream = false;
    let mut ahead = 0;
    let mut behind = 0;
    let mut last_commit_time = None;
    let mut last_commit_summary = None;
    let mut last_commit_author = None;

    if let Ok(head) = repo.head() {
        let local_oid = head.target();
        if head.is_branch() {
            branch = head.shorthand().map(String::from);
            if let (Some(name), Some(local)) = (branch.as_deref(), local_oid)
                && let Ok(b) = repo.find_branch(name, git2::BranchType::Local)
                && let Ok(upstream) = b.upstream()
            {
                has_upstream = true;
                if let Some(up_oid) = upstream.get().target()
                    && let Ok((a, b)) = repo.graph_ahead_behind(local, up_oid)
                {
                    ahead = a;
                    behind = b;
                }
            }
        } else {
            detached = true;
            branch = local_oid.map(|o| o.to_string()[..7.min(o.to_string().len())].to_string());
        }

        if let Ok(commit) = head.peel_to_commit() {
            last_commit_time = Some(commit.time().seconds());
            last_commit_summary = commit.summary().map(String::from);
            last_commit_author = commit.author().name().map(String::from);
        }
    }

    let mut stashes = 0;
    let _ = repo.stash_foreach(|_, _, _| {
        stashes += 1;
        true
    });

    Ok(RepoStatus {
        path: path.to_path_buf(),
        name,
        branch,
        detached,
        has_upstream,
        ahead,
        behind,
        staged,
        unstaged,
        untracked,
        conflicted,
        stashes,
        state: RepoState::from(repo.state()),
        last_commit_time,
        last_commit_summary,
        last_commit_author,
    })
}

pub fn relative_time(unix_secs: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(unix_secs);
    let diff = (now - unix_secs).max(0);
    match diff {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{}m ago", diff / 60),
        3600..=86399 => format!("{}h ago", diff / 3600),
        86400..=604799 => format!("{}d ago", diff / 86400),
        604800..=2_591_999 => format!("{}w ago", diff / 604800),
        _ => format!("{}mo ago", diff / 2_592_000),
    }
}
