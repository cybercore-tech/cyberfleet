# Security Policy

Cyberfleet reads local git repository state via `git2` (libgit2) and shells
out to the real `git` binary for the two operations that need your actual
credentials — `git fetch` and `git status` — plus your `$TERMINAL`/`$EDITOR`
to open a repo (see [docs — Architecture notes](README.md#how-it-works)). If
you find a security issue — a shell-injection risk in how a path or command
is built, an unsafe fallback in `cyberfleet-toggle`'s binary bootstrap, or
anything else — please report it privately rather than opening a public
issue.

## Reporting a vulnerability

Email **darkstardevx@gmail.com** (primary) or, as a backup,
**cybercore.sh@gmail.com**. Include:

- the affected file/commit and a minimal repro or PoC
- what you'd expect to happen instead
- how you'd rate the impact (your best guess is fine)

Expect an acknowledgement within a few days. Please don't include exploit
details in a public GitHub issue or PR until a fix has shipped.

## Supported versions

Only the latest tagged release and `main` are supported. There's no LTS
branch at this stage of the project.
