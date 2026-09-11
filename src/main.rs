mod app;
mod config;
mod git_ops;
mod scan;
mod status;
mod ui;

use anyhow::Result;
use app::{FetchJob, Mode, Pending};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

fn parse_args() -> std::result::Result<bool, String> {
    let mut summary = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--summary" => summary = true,
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                return Err(format!(
                    "cyberfleet: unrecognized argument '{other}'\nRun `cyberfleet --help` for usage."
                ));
            }
        }
    }
    Ok(summary)
}

fn print_usage() {
    println!(
        "cyberfleet — a multi-repo git status dashboard for Omarchy\n\
         \n\
         USAGE:\n\
         \x20   cyberfleet [OPTIONS]\n\
         \n\
         OPTIONS:\n\
         \x20   --summary   Print a one-line JSON status (for the bar widget) and exit\n\
         \x20   -h, --help  Print this help and exit\n\
         \n\
         Config: ~/.config/cyberfleet/config.json (roots to scan, ignore list, max depth)."
    );
}

/// Fast, local-only pass for the bar widget: discover + status every repo,
/// print one JSON line, exit. No network fetch — that stays an explicit TUI
/// action so the bar never blocks on a slow or offline remote.
fn print_summary() -> Result<()> {
    let cfg = config::load_or_init()?;
    let roots: Vec<_> = cfg.roots.iter().map(|r| config::expand_tilde(r)).collect();
    let paths = scan::discover_repos(&roots, &cfg.ignore, cfg.max_depth);
    let total = paths.len();
    let attention = paths
        .iter()
        .filter_map(|p| status::repo_status(p).ok())
        .filter(|s| s.needs_attention())
        .count();
    println!("{{\"attention\":{attention},\"total\":{total}}}");
    Ok(())
}

fn main() -> Result<()> {
    let summary_mode = match parse_args() {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };
    if summary_mode {
        return print_summary();
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new()?;
    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut app::App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Some(work) = app.pending.take() {
            app.perform(work)?;
            continue;
        }

        if let Some(job) = app.fetch_job.take() {
            suspend_and_fetch(terminal, app, job)?;
            continue;
        }

        if app.should_quit {
            return Ok(());
        }

        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::Normal => handle_normal(app, key.code, key.modifiers)?,
                Mode::Filter => handle_filter(app, key.code),
                Mode::AddRoot => handle_add_root(app, key.code)?,
                Mode::Detail => handle_detail(app, key.code),
                Mode::Help => handle_help(app, key.code),
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// Leaves the alternate screen and raw mode entirely, runs the fetch(es)
/// with a real inherited terminal (see `git_ops::fetch_inherited`), then
/// restores the TUI. See `app::FetchJob` for why this can't be done as
/// ordinary deferred `Pending` work inside the alternate screen.
fn suspend_and_fetch(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
    job: FetchJob,
) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    let indices: Vec<usize> = match job {
        FetchJob::One(i) => vec![i],
        FetchJob::All => (0..app.repos.len()).collect(),
    };
    let total = indices.len();

    for (n, idx) in indices.iter().enumerate() {
        let Some(r) = app.repos.get(*idx) else {
            continue;
        };
        println!("\n=== [{}/{total}] fetching {} ===", n + 1, r.name);
        let path = r.path.clone();
        match git_ops::fetch_inherited(&path) {
            Ok(status) if status.success() => {
                if let Ok(fresh) = status::repo_status(&path) {
                    app.repos[*idx] = fresh;
                }
            }
            Ok(status) => println!("git fetch exited with {status}"),
            Err(e) => println!("failed to run git fetch: {e}"),
        }
    }

    println!("\nDone — press Enter to return to cyberfleet.");
    let mut discard = String::new();
    let _ = io::stdin().read_line(&mut discard);

    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    terminal.clear()?;

    app.apply_sort_and_filter();
    app.status = Some(format!("fetch: {total} repo(s) done"));
    Ok(())
}

fn handle_normal(app: &mut app::App, code: KeyCode, mods: KeyModifiers) -> Result<()> {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.next(),
        KeyCode::Char('k') | KeyCode::Up => app.previous(),
        KeyCode::Char('/') => {
            app.mode = Mode::Filter;
        }
        KeyCode::Char('r') => {
            app.status = Some("rescanning...".to_string());
            app.pending = Some(Pending::Rescan);
        }
        KeyCode::Char('f') => {
            if let Some(idx) = app.selected_index() {
                app.fetch_job = Some(FetchJob::One(idx));
            }
        }
        KeyCode::Char('F') => {
            if !app.repos.is_empty() {
                app.fetch_job = Some(FetchJob::All);
            }
        }
        KeyCode::Char('o') | KeyCode::Enter => {
            if let Some(r) = app.selected_repo() {
                match git_ops::spawn_terminal_at(&r.path, None) {
                    Ok(()) => {}
                    Err(e) => app.status = Some(format!("error: {e}")),
                }
            }
        }
        KeyCode::Char('e') => {
            if let Some(r) = app.selected_repo() {
                let editor = std::env::var("EDITOR")
                    .or_else(|_| std::env::var("VISUAL"))
                    .unwrap_or_else(|_| "nvim".to_string());
                let cmd = format!("{editor} .");
                match git_ops::spawn_terminal_at(&r.path, Some(&cmd)) {
                    Ok(()) => {}
                    Err(e) => app.status = Some(format!("error: {e}")),
                }
            }
        }
        KeyCode::Char('d') => {
            if let Some(r) = app.selected_repo() {
                let header = format!(
                    "last commit: {} ({})\n\n",
                    r.last_commit_summary.as_deref().unwrap_or("-"),
                    r.last_commit_author.as_deref().unwrap_or("unknown author"),
                );
                let body = git_ops::status_text(&r.path)
                    .unwrap_or_else(|e| format!("error running git status: {e}"));
                app.detail_text = header + &body;
                app.mode = Mode::Detail;
            }
        }
        KeyCode::Char('a') => {
            app.mode = Mode::AddRoot;
            app.input_buffer.clear();
        }
        KeyCode::Char('i') => {
            if let Err(e) = app.ignore_selected() {
                app.status = Some(format!("error: {e}"));
            } else {
                app.status = Some("ignoring rescan...".to_string());
                app.pending = Some(Pending::Rescan);
            }
        }
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        _ => {}
    }
    Ok(())
}

fn handle_filter(app: &mut app::App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.filter_text.clear();
            app.apply_sort_and_filter();
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => app.mode = Mode::Normal,
        KeyCode::Backspace => {
            app.filter_text.pop();
            app.apply_sort_and_filter();
        }
        KeyCode::Char(c) => {
            app.filter_text.push(c);
            app.apply_sort_and_filter();
        }
        _ => {}
    }
}

fn handle_add_root(app: &mut app::App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Esc => {
            app.input_buffer.clear();
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            let raw = app.input_buffer.clone();
            app.add_root(&raw)?;
            app.input_buffer.clear();
            app.mode = Mode::Normal;
            app.status = Some("rescanning...".to_string());
            app.pending = Some(Pending::Rescan);
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => app.input_buffer.push(c),
        _ => {}
    }
    Ok(())
}

fn handle_detail(app: &mut app::App, code: KeyCode) {
    if matches!(code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('d')) {
        app.mode = Mode::Normal;
    }
}

fn handle_help(app: &mut app::App, code: KeyCode) {
    if matches!(code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')) {
        app.mode = Mode::Normal;
    }
}
