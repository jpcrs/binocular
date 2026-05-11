<p>
    <h1 align="center">Binocular</h1>
</p>

> Not as useful as a telescope, but helps in some situations.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Commands](#commands)
- [Features](#features)
  - [Fuzzy & Exact Search](#fuzzy--exact-search)
  - [Rich Previews](#rich-previews)
  - [Git Integration](#git-integration)
  - [Structured Log Viewer](#structured-log-viewer)
  - [Diff Preview](#diff-preview)
  - [Vim Navigation](#vim-navigation)
  - [Custom Preview Commands](#custom-preview-commands)
- [Configuration](#configuration)
- [Headless Mode](#headless-mode)
- [Output Formats](#output-formats)
- [VS Code Integration](#vs-code-integration)

## Installation

#### Cargo
```bash
cargo install binocular-cli
```

#### From source
```bash
git clone https://github.com/jpcrs/Binocular.git
cd Binocular/tui
cargo build --release
```

The binary will be produced at `../target/release/binocular`. Make sure it's on your `$PATH`.

## Quick Start
```bash
# Default path search
binocular

# Search file names only (Path will not be considered in the search)
binocular files

# Grep file contents
binocular content

# Search directories
binocular dirs

# Search git history of a file
binocular git history src/main.rs

# List local branches
binocular git branches

# Inspect current branch commits
binocular git commits

# Tail structured logs
binocular log service.log
kubectl logs -f deployment/app | binocular log

# Diff two files
binocular diff old.rs new.rs
```

## Commands

| Command | Description |
|---|---|
| `binocular` | Default path search. |
| `binocular path [QUERY]` | Search full paths. |
| `binocular files [QUERY]` | Search file names only. |
| `binocular content [QUERY]` | Search file contents (alias: `grep`). |
| `binocular dirs [QUERY]` | Search directories only. |
| `binocular git history <FILE> [QUERY]` | Search committed line history for a tracked file. |
| `binocular git branches [QUERY]` | Search local branches. |
| `binocular git commits [QUERY]` | Search commits on the current branch (alias: `git logs`). |
| `binocular log [FILE...]` | Open the structured log viewer from stdin or tail log files directly. |
| `binocular diff <LEFT> <RIGHT>` | Open a direct diff preview for two files. |

### Global Options

- `-H, --headless` — Skip the TUI and print results directly.
- `--output-format <plain|jsonl>` — Format for interactive selection output.
- `--preview <command>` — Use a custom preview command.
- `--delimiter <string>` — Delimiter for preview placeholders (default: `:`).
- `--split <string>` — Split stdin lines into multiple items.
- `-e, --exact` — Switch from fuzzy to contiguous token matching.
- `-l, --location <DIR>` — Add search roots.
- `--no-hidden`, `--no-git-ignore`, `--no-ignore` — Adjust filesystem filtering.

## Features

### Rich Previews
The preview pane handles a wide variety of file types out of the box:

- **Source code** — Tree-sitter syntax highlighting for Rust, Python, JavaScript/TypeScript, Go, C/C++, C#, JSON, TOML, YAML, HTML, and CSS...
- **Images** — JPEG, PNG, GIF, WebP, BMP, TIFF, ICO rendered inline in the terminal.
- **Archives** — ZIP, TAR (with deflate, bzip2, zstd) file listings.
- **PDFs** — Text extraction for content search and preview.
- **SQLite** — Schema and row preview for `.db` and `.sqlite` files.
- **Media** — Audio/video metadata (ID3, FLAC, Spotlight on macOS) with embedded artwork.
- **Binary** — Hex dump, entropy analysis, and printable string extraction.
- **Structured logs** — JSONL and logfmt parsing with live tail support.

### Vim Navigation

The search box and the preview pane supports vim movements.

### Custom Preview Commands

```bash
binocular --preview "bat --style=plain --line-range={1}: {0}" content
```

## Configuration
Binocular writes a default config on first launch:

| Platform | Path |
|---|---|
| macOS / Linux (XDG) | `$XDG_CONFIG_HOME/binocular/config.toml` |
| macOS (fallback) | `~/.config/binocular/config.toml` |
| Windows | `%APPDATA%\binocular\config.toml` |

The bundled template lives at [`tui/config/default.toml`](config/default.toml).

Current config covers:

- **Keybindings** — Some keybindings
- **Log viewer** — `log.max_entries` and related settings.
- **Layout** — Persisted pane splits saved to `layout.toml` beside the config.

## Headless Mode

Use `--headless` (or `-H`) to bypass the TUI and stream results directly:

```bash

# Split each line into multiple items
printf 'a,b,c\n' | binocular --headless --split ','

```

## Output Formats

Interactive mode prints selections to stdout after the TUI exits:

| Source | Plain Output | JSONL (`--output-format jsonl`) |
|---|---|---|
| Path | absolute path | `{"kind":"path","path":"..."}` |
| Content | `abs_path:line[:column]` | `{"kind":"grep","path":"...","line":1}` |
| Git History | `commit:path:line` | `{"kind":"git_history","commit":"..."}` |
| Stdin | raw line | `{"kind":"stdin","text":"..."}` |
| Branch | branch name | `{"kind":"git_branch","name":"..."}` |
| Commit | commit hash | `{"kind":"git_commit","hash":"..."}` |

## More Docs

- [Repository Overview](../README.md)
- [Architecture](../Architecture.md) — Runtime topology, state ownership, and extension guide.
