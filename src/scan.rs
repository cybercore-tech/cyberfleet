use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Walks each root up to `max_depth`, collecting directories that contain a
/// `.git` entry (dir or file — covers plain repos, worktrees, and
/// submodules alike). Skips any directory whose name matches `ignore`, and
/// never descends into `.git` itself regardless of the ignore list — no
/// reason to ever walk a repo's object store.
pub fn discover_repos(roots: &[PathBuf], ignore: &[String], max_depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();

    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let walker = WalkDir::new(root)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                name != ".git" && !ignore.iter().any(|pat| pat.as_str() == name)
            });

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_dir() && is_repo_root(entry.path()) {
                found.push(entry.path().to_path_buf());
            }
        }
    }

    found.sort();
    found.dedup();
    found
}

fn is_repo_root(dir: &Path) -> bool {
    dir.join(".git").exists()
}
