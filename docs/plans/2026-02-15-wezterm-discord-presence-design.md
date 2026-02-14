# WezTerm Discord Rich Presence - Design Document

**Date:** 2026-02-15
**Status:** Approved

## Overview

A Rust daemon that runs on Windows and displays WezTerm terminal activity as Discord Rich Presence. Shows the current foreground process, working directory, tab/pane count, and session duration.

## Architecture

```
WezTerm (Windows) <--poll 3s--> wezterm-presence (Rust, Windows) --IPC pipe--> Discord Client
```

- **Rust daemon** runs on Windows as a background process
- Polls `wezterm cli list --format json` every ~3 seconds
- Extracts active pane info: foreground process, cwd, tab count, pane count
- Sends rich presence updates to Discord via Windows named pipe (`\\.\pipe\discord-ipc-{i}`)
- Clears presence when WezTerm is not running
- Auto-starts via Windows startup folder shortcut

## Environment

- WezTerm runs on Windows, default domain is WSL:archlinux
- Discord runs on Windows
- Daemon runs on Windows (same side as Discord IPC)
- No changes to `.wezterm.lua` required

## Discord Presence Display

```
Playing WezTerm
+--------------------------------------+
| [WezTerm logo]  Using Claude Code    |  <- details (process description)
|  [claude icon]  ~/wezterm-presence   |  <- state (cwd) + small image
|                 2 tabs, 3 panes      |  <- large image hover text
|                 01:15 elapsed        |  <- timestamp
+--------------------------------------+
```

## Process Mappings

Configurable via `config.toml`. Defaults:

| Process | Display Text | Icon Key |
|---------|-------------|----------|
| claude | Using Claude Code | claude |
| nvim | Editing in Neovim | neovim |
| vim | Editing in Vim | vim |
| cargo | Building with Cargo | rust |
| zsh | In the Shell | terminal |
| bash | In the Shell | terminal |
| python | Running Python | python |
| node | Running Node.js | nodejs |
| git | Using Git | git |
| docker | Running Docker | docker |
| ssh | Connected via SSH | ssh |
| (unknown) | Running {process_name} | terminal |

## Config File

`config.toml` located next to the binary:

```toml
poll_interval = 3
discord_app_id = "YOUR_APP_ID"

[processes]
claude = { text = "Using Claude Code", icon = "claude" }
nvim = { text = "Editing in Neovim", icon = "neovim" }
vim = { text = "Editing in Vim", icon = "vim" }
cargo = { text = "Building with Cargo", icon = "rust" }
zsh = { text = "In the Shell", icon = "terminal" }
bash = { text = "In the Shell", icon = "terminal" }
python = { text = "Running Python", icon = "python" }
node = { text = "Running Node.js", icon = "nodejs" }
git = { text = "Using Git", icon = "git" }
docker = { text = "Running Docker", icon = "docker" }
ssh = { text = "Connected via SSH", icon = "ssh" }
```

## Behavior

- **WezTerm running:** Show active pane's process, cwd, tab/pane counts, session elapsed time
- **WezTerm closed:** Clear Discord presence entirely
- **Multiple tabs/panes:** Show info for the currently focused pane
- **Process change:** Update presence on next poll cycle (~3s)
- **Discord not running:** Retry connection periodically, no crash

## Dependencies (Rust crates)

- `discord-rich-presence` - Discord IPC communication
- `serde` / `serde_json` - JSON parsing of wezterm cli output
- `toml` - Config file parsing
- `tokio` or blocking loop - Main poll loop
- `dirs` - Finding config/startup paths

## Startup

Auto-start via shortcut in `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\` or an install command (`wezterm-presence install`).
