<p>
    <h1 align="center">Binocular</h1>
</p>

> Not as useful as a telescope, but helps in some situations.

<p align="center">
  <a href="#installation"><img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue" alt="Platform"></a>
  <a href="https://github.com/jpcrs/Binocular/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License"></a>
  <a href="https://github.com/jpcrs/Binocular/actions"><img src="https://img.shields.io/github/actions/workflow/status/jpcrs/Binocular/ci.yml?label=build" alt="Build"></a>
</p>

> If you're looking for the vscode integration, check out [`vscode/README.md`](vscode/README.md).

## Overview
Yet another TUI fuzzy finder. Fast and yada yada...

[https://raw.githubusercontent.com/jpcrs/binocular-hidden/main/assets/binocular.mp4](https://github.com/user-attachments/assets/9967e593-41b7-4d5c-8848-34a3dbf7fae1)

## Motivation
Not needing to configure my rg+fzf scripts in every new machine.

## Features
- **Kinda fast** — How fast? No idea. Faster than others? No idea. It uses `nucleo` for fuzzy search and SIMD accelerated substring search for exact serch.
- **Rich previews** — Syntax highlighting via tree-sitter, image rendering, PDF text extraction, archive listings, SQLite schema inspection, media metadata, hex dumps, etc.
- **Structured log viewer** — Real-time tailing of JSONL/logfmt streams with filtering, field discovery, and follow mode.
- **Vim-like navigation** — Some vim motions when in text preview, including visual mode, text objects, search, and editing.
- **Headless mode** — No TUI, straight to stdout.
- **Custom preview commands** — Plug in your own preview tool (`bat`, `cat`, etc).

Built-in previewers:
- **Source code** — Tree-sitter syntax highlighting for Rust, Python, JavaScript/TypeScript, Go, C/C++, C#, JSON, TOML, YAML, HTML, CSS...
- **Images** — JPEG, PNG, GIF, WebP, BMP, TIFF, ICO rendered inline in the terminal.
- **Archives** — ZIP, TAR (with deflate, bzip2, zstd) file listings.
- **PDFs** — Text extraction for content search and preview.
- **SQLite** — Schema and row preview for `.db` and `.sqlite` files.
- **Media** — Audio/video metadata (ID3, FLAC, Spotlight on macOS).
- **Binary** — Hex dump, entropy analysis, and printable string extraction.
- **Structured logs** — JSONL and logfmt parsing with live tail.

## Other Options

- [television](https://github.com/alexpasmantier/television) Mature, more customizable, probably no reason to use binocular over television at all.
- [rg](https://github.com/burntsushi/ripgrep)+[fzf](https://github.com/junegunn/fzf)+whatever - Be creative, no limits.

## Installation

#### Cargo

```bash
cargo install binocular
```

#### From source

```bash
git clone https://github.com/jpcrs/Binocular.git
cd Binocular/tui
cargo build --release
```

## Documentation

- [TUI Documentation](tui/README.md)
- [VS Code Extension](vscode/README.md)

## Credits
[Telescope](https://github.com/nvim-telescope/telescope.nvim) - Inspired the first version of binocular
[Television](https://github.com/alexpasmantier/television/) - Inspired the second version of binocular
[Nucleo](https://github.com/helix-editor/nucleo) - Fuzzy search
[Ratatui](https://github.com/ratatui/ratatui) - TUI Framework

## License

MIT © [jpcrs](https://github.com/jpcrs)
