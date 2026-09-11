use anyhow::{Result, anyhow};
use std::path::Path;
use std::process::Command;

/// Runs `git fetch` with the parent's real stdio inherited (not captured) —
/// deliberate, so an SSH passphrase or host-key prompt lands on an actual
/// terminal the user can answer, instead of a pipe nobody's reading. Only
/// call this while the TUI is suspended (see `app::FetchJob`); called while
/// the alternate screen owns the terminal, the exact same prompt would hang
/// the app with its own keystrokes silently swallowed by `ssh`.
pub fn fetch_inherited(path: &Path) -> Result<std::process::ExitStatus> {
    Ok(Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("fetch")
        .status()?)
}

/// Plain `git status` text for the detail popup — shown verbatim, so it
/// looks exactly like what running the command yourself would show.
pub fn status_text(path: &Path) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("status")
        .output()?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Opens a floating terminal at `path`, optionally running `run` there
/// instead of a plain shell (used for "open in $EDITOR": launching the
/// editor inside a fresh terminal works uniformly whether it's a TUI editor
/// like nvim or a GUI one like `code`, without needing to tell them apart).
pub fn spawn_terminal_at(path: &Path, run: Option<&str>) -> Result<()> {
    let terminal = std::env::var("TERMINAL").ok().or_else(detect_terminal);
    let terminal = terminal.ok_or_else(|| anyhow!("no terminal found; set $TERMINAL"))?;
    let path_str = path.to_string_lossy().to_string();

    let mut cmd = Command::new("setsid");
    cmd.arg("uwsm-app").arg("--");
    match terminal.as_str() {
        "alacritty" => {
            cmd.args(["alacritty", "--working-directory", &path_str]);
            if let Some(r) = run {
                cmd.args(["-e", "bash", "-lc", r]);
            }
        }
        "kitty" => {
            cmd.args(["kitty", "--directory", &path_str]);
            if let Some(r) = run {
                cmd.args(["bash", "-lc", r]);
            }
        }
        "ghostty" => {
            cmd.arg("ghostty")
                .arg(format!("--working-directory={path_str}"));
            if let Some(r) = run {
                cmd.args(["-e", "bash", "-lc", r]);
            }
        }
        "foot" => {
            cmd.args(["foot", "-D", &path_str]);
            if let Some(r) = run {
                cmd.args(["bash", "-lc", r]);
            }
        }
        _ => {
            // xdg-terminal-exec — the spec Omarchy/uwsm actually points
            // $TERMINAL at by default, so this is the branch that really
            // runs here, not just a generic fallback. Its working-directory
            // flag is `--dir=`, not `--working-directory=`: per its spec, an
            // unrecognized dashed option is *silently discarded* rather than
            // an error, which is exactly how the wrong flag name quietly
            // produced a terminal in the wrong directory instead of failing
            // loudly. Manually passing `-e` is equally pointless here — the
            // spec says any execution argument given on the command line
            // gets stripped, since it inserts the correct one itself for
            // whatever terminal it resolves to — so the command goes after
            // a bare `--` instead.
            cmd.arg("xdg-terminal-exec")
                .arg(format!("--dir={path_str}"))
                .arg("--title=Cyberfleet");
            if let Some(r) = run {
                cmd.arg("--").arg("bash").arg("-lc").arg(r);
            }
        }
    }
    cmd.spawn()?;
    Ok(())
}

fn detect_terminal() -> Option<String> {
    for candidate in ["ghostty", "alacritty", "kitty", "foot"] {
        let found = Command::new("which")
            .arg(candidate)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if found {
            return Some(candidate.to_string());
        }
    }
    None
}
