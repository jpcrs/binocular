<p>
    <h1 align="center">Binocular for vscode</h1>
</p>

<p align="center">
  <a href="https://github.com/jpcrs/Binocular/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License"></a>
</p>

## Overview

Integrates [Binocular](https://github.com/jpcrs/Binocular) into VS Code.

<p align="center">
  <img width="855" height="556" alt="vscode" src="https://github.com/user-attachments/assets/0262cb5d-bea2-449a-a3a9-4812152cb7c6" />
</p>


## Installation

1. Install the extension from the VS Code Marketplace (or build from `vscode/`).
2. Ensure the `binocular` binary is on your `$PATH`, or set `binocular.binaryPath` in your VS Code settings.

## Commands

| Command | Title |
|---|---|
| `binocular.searchPath` | **Binocular: Search Paths** |
| `binocular.searchFiles` | **Binocular: Search Files** |
| `binocular.searchContent` | **Binocular: Search Content** |
| `binocular.searchContentWithSelection` | **Binocular: Search Content With Selection** |
| `binocular.searchDirectories` | **Binocular: Search Directories** |
| `binocular.searchPathConfigured` | **Binocular: Search Paths (Configured Folders)** |
| `binocular.searchFilesConfigured` | **Binocular: Search Files (Configured Folders)** |
| `binocular.searchContentConfigured` | **Binocular: Search Content (Configured Folders)** |
| `binocular.searchContentWithSelectionConfigured` | **Binocular: Search Content With Selection (Configured Folders)** |
| `binocular.searchDirectoriesConfigured` | **Binocular: Search Directories (Configured Folders)** |
| `binocular.runCustomCommand` | **Binocular: Run Custom Command** |


## Settings

| Setting | Type | Default | Description |
|---|---|---|---|
| `binocular.binaryPath` | `string` | `""` | Explicit path to the `binocular` binary. Falls back to `PATH` when empty. |
| `binocular.searchPaths` | `array` | `[]` | List of folders to search via the *Configured Folders* commands. |
| `binocular.useExact` | `boolean` | `false` | When enabled, adds the `--exact` flag to all search commands. |
| `binocular.customCommands` | `array` | `[]` | Custom commands that can be launched in a terminal editor. |

### Configured Folders

Set `binocular.searchPaths` to define a fixed list of directories that the *Configured Folders* command variants will search instead of the current workspace:

```json
{
  "binocular.searchPaths": [
    "/Users/jpcrs/Projects/dotfiles",
    "/Users/jpcrs/Projects/binocular"
  ]
}
```

Let **Binocular: Search Files (Configured Folders)** to search those folders regardless of which workspace is currently open.

### Exact Matching

Enable `binocular.useExact` to always pass the `--exact` flag to every search command. This makes every search token match as a contiguous substring instead of fuzzy matching:

```json
{
  "binocular.useExact": true
}
```

### Custom Commands

Define your own launcher entries for tools you use alongside Binocular:

```json
{
  "binocular.customCommands": [
    {
      "title": "lazygit",
      "command": "lazygit"
    }
  ]
}
```

Properties:

- `title` *(required)* — Display name in the command palette.
- `command` *(required)* — The executable to run.
- `id` *(optional)* — Stable identifier for direct keybinding targeting.
- `args` *(optional)* — Array of arguments passed to the command.
- `cwd` *(optional)* — Working directory strategy (`activeWorkspace`, `firstWorkspace`, `activeFileDir`).
- `env` *(optional)* — Extra environment variables.
- `useShell` *(optional)* — Run through a shell (`false` by default).

### Keybinding Example

Target a custom command directly by `id` (or `title`/`command` as fallback):

```json
{
  "key": "ctrl+alt+g",
  "command": "binocular.runCustomCommand",
  "args": "lazygit"
}
```
