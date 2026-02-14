# wezterm-presence

Discord Rich Presence for WezTerm. Shows what you're doing in your terminal on your Discord profile.

## Features

- Per-process display with custom icons (neovim, cargo, claude, python, etc.)
- Working directory, tab count, pane count, and elapsed time
- System tray icon with quit menu
- Auto-starts with WezTerm
- Only updates Discord when state changes (no flickering)
- Clears presence when WezTerm closes

## How It Works

WezTerm's Lua config writes terminal state (process, cwd, tabs, panes) to a JSON file on every status bar update. A Rust daemon on Windows reads this file and pushes it to Discord via IPC. For WSL panes, zsh hooks set a WezTerm user-var with the current process name since `get_foreground_process_name()` returns `wslhost.exe`.

```
zsh hooks -> WezTerm user-var -> Lua writes JSON -> Rust daemon -> Discord IPC
```

## Setup

### 1. Build

Requires Rust and `x86_64-pc-windows-gnu` target (for cross-compiling from WSL):

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu --release
```

Copy `target/x86_64-pc-windows-gnu/release/wezterm-presence.exe` somewhere on your Windows PATH.

### 2. Discord Application

Create an application at [discord.com/developers](https://discord.com/developers/applications). Upload Rich Presence art assets with these keys:

| Key | Description |
|-----|-------------|
| `wezterm` | Large image (WezTerm logo) |
| `claude` | Claude Code |
| `neovim` | Neovim |
| `vim` | Vim |
| `rust` | Rust (cargo, rustc) |
| `terminal` | Generic terminal (zsh, bash, fish) |
| `python` | Python |
| `nodejs` | Node.js |
| `git` | Git |
| `docker` | Docker |
| `ssh` | SSH |

The default Discord App ID is hardcoded. To use your own, create a `config.toml` next to the exe:

```sh
wezterm-presence.exe init
```

### 3. WezTerm Lua Config

Add to your `.wezterm.lua`:

```lua
local wezterm = require 'wezterm'

-- Auto-start wezterm-presence.exe if not already running
local handle = io.popen('tasklist /FI "IMAGENAME eq wezterm-presence.exe" /NH 2>nul')
local result = handle:read('*a')
handle:close()
if not result:find('wezterm%-presence%.exe') then
  os.execute('start "" /b "C:\\path\\to\\wezterm-presence.exe"')
end

-- State file path
local presence_state_path = (os.getenv('LOCALAPPDATA') or '') .. '\\wezterm-presence\\state.json'
os.execute('mkdir "' .. (os.getenv('LOCALAPPDATA') or '') .. '\\wezterm-presence" 2>nul')

wezterm.on('update-status', function(window, pane)
  local user_vars = pane:get_user_vars()
  local process = user_vars.WEZTERM_PROG or ''

  if process == '' then
    process = pane:get_foreground_process_name() or 'unknown'
    process = process:match('[^/\\]+$') or process
    if process == 'wslhost.exe' then
      process = 'zsh'
    end
  end

  local cwd = pane:get_current_working_dir()
  local cwd_str = ''
  if cwd then
    cwd_str = cwd.file_path or tostring(cwd)
  end

  local tab_count = 0
  for _ in pairs(window:mux_window():tabs()) do
    tab_count = tab_count + 1
  end

  local pane_count = 0
  for _, tab in pairs(window:mux_window():tabs()) do
    for _ in pairs(tab:panes()) do
      pane_count = pane_count + 1
    end
  end

  local json = string.format(
    '{"process":"%s","cwd":"%s","tabs":%d,"panes":%d}',
    process:gsub('\\', '\\\\'):gsub('"', '\\"'),
    cwd_str:gsub('\\', '/'):gsub('"', '\\"'),
    tab_count,
    pane_count
  )

  local f = io.open(presence_state_path, 'w')
  if f then
    f:write(json)
    f:close()
  end
end)
```

### 4. WSL Process Detection (optional)

If you use WSL, add these hooks to your `.zshrc` (or `.bashrc` equivalent) so the presence shows the actual running command instead of `wslhost.exe`:

```zsh
# WezTerm Presence - Process Tracking
__wezterm_set_user_var() {
  printf "\033]1337;SetUserVar=%s=%s\007" "$1" "$(printf '%s' "$2" | base64 -w0)"
}
__wezterm_preexec() {
  local cmd="${1%% *}"
  __wezterm_set_user_var WEZTERM_PROG "$cmd"
}
__wezterm_precmd() {
  __wezterm_set_user_var WEZTERM_PROG "zsh"
}
autoload -Uz add-zsh-hook
add-zsh-hook preexec __wezterm_preexec
add-zsh-hook precmd __wezterm_precmd
```

### 5. Auto-Start on Login (optional)

```sh
wezterm-presence.exe install
```

Creates a VBS script in the Windows Startup folder.

## Config

Optional `config.toml` placed next to the exe:

```toml
poll_interval = 1
discord_app_id = "YOUR_APP_ID"

[processes]
claude = { text = "Using Claude Code", icon = "claude" }
nvim = { text = "Editing in Neovim", icon = "neovim" }
cargo = { text = "Building with Cargo", icon = "rust" }
zsh = { text = "In the Shell", icon = "terminal" }
# Add your own...
```

## Architecture

```
+-----------+   user-var   +------------+   JSON file   +----------------+   IPC   +---------+
| zsh hooks | -----------> | WezTerm    | ------------> | wezterm-       | ------> | Discord |
| (preexec/ |   OSC 1337   | Lua config |  state.json   | presence.exe   |  named  |         |
|  precmd)  |              | update-    |               | (Rust daemon)  |  pipe   |         |
+-----------+              | status     |               +----------------+         +---------+
                           +------------+
```

## License

MIT
