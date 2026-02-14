# WezTerm Discord Rich Presence - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust daemon that polls WezTerm CLI and displays terminal activity as Discord Rich Presence on Windows.

**Architecture:** A single Rust binary runs on Windows. It polls `wezterm cli list --format json` every 3 seconds to get active pane info (process, cwd, tab/pane counts), then sends updates to Discord via the IPC named pipe. When WezTerm isn't running, it clears the presence.

**Tech Stack:** Rust, discord-rich-presence crate, serde/serde_json, toml, std::process::Command (synchronous — no async runtime needed)

**Cross-compilation:** Developed in WSL, cross-compiled to Windows via `cargo build --target x86_64-pc-windows-gnu` (requires `mingw-w64` on Arch).

---

## Pre-requisite: Discord Developer Application

Before any code runs, you need a Discord Application ID:

1. Go to https://discord.com/developers/applications
2. Click "New Application", name it "WezTerm"
3. Copy the **Application ID** (this is the client ID)
4. Go to "Rich Presence" → "Art Assets"
5. Upload icons named: `wezterm`, `terminal`, `neovim`, `vim`, `rust`, `python`, `nodejs`, `git`, `docker`, `ssh`, `claude`
6. Save the Application ID for `config.toml`

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs` (placeholder)
- Create: `.gitignore`

**Step 1: Initialize the Cargo project**

Run: `cargo init /home/esoteric/wezterm-presence --name wezterm-presence`

**Step 2: Set up Cargo.toml with dependencies**

```toml
[package]
name = "wezterm-presence"
version = "0.1.0"
edition = "2021"
description = "Discord Rich Presence for WezTerm terminal"

[dependencies]
discord-rich-presence = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
dirs = "6"
```

**Step 3: Create .gitignore**

```
/target
*.exe
```

**Step 4: Install cross-compilation toolchain**

Run: `sudo pacman -S mingw-w64-gcc --needed && rustup target add x86_64-pc-windows-gnu`

**Step 5: Verify it compiles for Windows**

Run: `cargo build --target x86_64-pc-windows-gnu`
Expected: Compiles successfully (empty main)

**Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs .gitignore
git commit -m "feat: scaffold Rust project with dependencies"
```

---

### Task 2: Config Module

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs` (add mod declaration)

**Step 1: Write the test for config parsing**

Add to bottom of `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_config() {
        let toml_str = r#"
            poll_interval = 5
            discord_app_id = "123456789"

            [processes]
            nvim = { text = "Editing in Neovim", icon = "neovim" }
            zsh = { text = "In the Shell", icon = "terminal" }
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.poll_interval, 5);
        assert_eq!(config.discord_app_id, "123456789");
        assert_eq!(config.processes.len(), 2);
        assert_eq!(config.processes["nvim"].text, "Editing in Neovim");
        assert_eq!(config.processes["nvim"].icon, "neovim");
    }

    #[test]
    fn test_default_config_has_entries() {
        let config = Config::default();
        assert_eq!(config.poll_interval, 3);
        assert!(config.processes.contains_key("claude"));
        assert!(config.processes.contains_key("nvim"));
        assert!(config.processes.contains_key("zsh"));
    }

    #[test]
    fn test_resolve_known_process() {
        let config = Config::default();
        let (text, icon) = config.resolve_process("nvim");
        assert_eq!(text, "Editing in Neovim");
        assert_eq!(icon, "neovim");
    }

    #[test]
    fn test_resolve_unknown_process() {
        let config = Config::default();
        let (text, icon) = config.resolve_process("weirdtool");
        assert_eq!(text, "Running weirdtool");
        assert_eq!(icon, "terminal");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib config`
Expected: FAIL — module doesn't exist yet

**Step 3: Implement the config module**

`src/config.rs`:

```rust
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessMapping {
    pub text: String,
    pub icon: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,
    pub discord_app_id: String,
    #[serde(default = "default_processes")]
    pub processes: HashMap<String, ProcessMapping>,
}

fn default_poll_interval() -> u64 {
    3
}

fn default_processes() -> HashMap<String, ProcessMapping> {
    let mut m = HashMap::new();
    let entries = [
        ("claude", "Using Claude Code", "claude"),
        ("nvim", "Editing in Neovim", "neovim"),
        ("vim", "Editing in Vim", "vim"),
        ("cargo", "Building with Cargo", "rust"),
        ("rustc", "Compiling Rust", "rust"),
        ("zsh", "In the Shell", "terminal"),
        ("bash", "In the Shell", "terminal"),
        ("fish", "In the Shell", "terminal"),
        ("python", "Running Python", "python"),
        ("python3", "Running Python", "python"),
        ("node", "Running Node.js", "nodejs"),
        ("git", "Using Git", "git"),
        ("docker", "Running Docker", "docker"),
        ("ssh", "Connected via SSH", "ssh"),
        ("htop", "Monitoring System", "terminal"),
        ("btop", "Monitoring System", "terminal"),
        ("make", "Running Make", "terminal"),
    ];
    for (name, text, icon) in entries {
        m.insert(name.to_string(), ProcessMapping {
            text: text.to_string(),
            icon: icon.to_string(),
        });
    }
    m
}

impl Default for Config {
    fn default() -> Self {
        Self {
            poll_interval: default_poll_interval(),
            discord_app_id: String::new(),
            processes: default_processes(),
        }
    }
}

impl Config {
    /// Load config from file, falling back to defaults for missing fields.
    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("Warning: failed to parse config at {}: {}", path.display(), e);
                eprintln!("Using default config.");
                Self::default()
            }),
            Err(_) => {
                eprintln!("No config found at {}. Using defaults.", path.display());
                Self::default()
            }
        }
    }

    /// Returns the path to config.toml next to the executable.
    fn config_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("config.toml")))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    /// Resolve a process name to (display_text, icon_key).
    /// Falls back to "Running {name}" with "terminal" icon for unknown processes.
    pub fn resolve_process(&self, name: &str) -> (String, String) {
        if let Some(mapping) = self.processes.get(name) {
            (mapping.text.clone(), mapping.icon.clone())
        } else {
            (format!("Running {}", name), "terminal".to_string())
        }
    }
}
```

Add to `src/main.rs`:

```rust
mod config;

fn main() {
    println!("wezterm-presence starting...");
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib config`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add config module with TOML parsing and process mappings"
```

---

### Task 3: WezTerm CLI Polling Module

**Files:**
- Create: `src/wezterm.rs`
- Modify: `src/main.rs` (add mod declaration)

**Step 1: Write tests for JSON parsing**

Add to bottom of `src/wezterm.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"[
        {
            "window_id": 0,
            "tab_id": 0,
            "pane_id": 0,
            "workspace": "default",
            "size": { "rows": 40, "cols": 140 },
            "title": "nvim src/main.rs",
            "cwd": "file://DESKTOP-ABC/home/esoteric/wezterm-presence"
        },
        {
            "window_id": 0,
            "tab_id": 1,
            "pane_id": 1,
            "workspace": "default",
            "size": { "rows": 40, "cols": 140 },
            "title": "zsh",
            "cwd": "file://DESKTOP-ABC/home/esoteric"
        },
        {
            "window_id": 0,
            "tab_id": 1,
            "pane_id": 2,
            "workspace": "default",
            "size": { "rows": 20, "cols": 140 },
            "title": "cargo build",
            "cwd": "file://DESKTOP-ABC/home/esoteric/wezterm-presence"
        }
    ]"#;

    #[test]
    fn test_parse_panes() {
        let panes: Vec<Pane> = serde_json::from_str(SAMPLE_JSON).unwrap();
        assert_eq!(panes.len(), 3);
        assert_eq!(panes[0].title, "nvim src/main.rs");
        assert_eq!(panes[0].tab_id, 0);
    }

    #[test]
    fn test_extract_process_name_simple() {
        assert_eq!(extract_process_name("zsh"), "zsh");
        assert_eq!(extract_process_name("bash"), "bash");
    }

    #[test]
    fn test_extract_process_name_with_args() {
        assert_eq!(extract_process_name("nvim src/main.rs"), "nvim");
        assert_eq!(extract_process_name("cargo build"), "cargo");
        assert_eq!(extract_process_name("python3 script.py"), "python3");
    }

    #[test]
    fn test_extract_process_name_with_path() {
        assert_eq!(extract_process_name("/usr/bin/nvim"), "nvim");
        assert_eq!(extract_process_name("/usr/bin/nvim src/main.rs"), "nvim");
    }

    #[test]
    fn test_extract_process_name_ssh_prompt() {
        // WSL titles sometimes look like "user@host: ~/dir"
        assert_eq!(extract_process_name("esoteric@DESKTOP: ~/projects"), "esoteric@DESKTOP:");
    }

    #[test]
    fn test_prettify_cwd() {
        assert_eq!(
            prettify_cwd("file://DESKTOP-ABC/home/esoteric/wezterm-presence"),
            "~/wezterm-presence"
        );
        assert_eq!(
            prettify_cwd("file://DESKTOP-ABC/home/esoteric"),
            "~"
        );
        assert_eq!(
            prettify_cwd("file:///C:/Users/bhanu/projects"),
            "C:/Users/bhanu/projects"
        );
    }

    #[test]
    fn test_build_terminal_state() {
        let panes: Vec<Pane> = serde_json::from_str(SAMPLE_JSON).unwrap();
        let state = TerminalState::from_panes(panes);
        assert_eq!(state.tab_count, 2);
        assert_eq!(state.pane_count, 3);
        // First pane is "active" (heuristic: first in list)
        assert_eq!(state.process_name, "nvim");
        assert_eq!(state.cwd, "~/wezterm-presence");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib wezterm`
Expected: FAIL — module doesn't exist yet

**Step 3: Implement the wezterm module**

`src/wezterm.rs`:

```rust
use serde::Deserialize;
use std::collections::HashSet;
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct Pane {
    pub window_id: u64,
    pub tab_id: u64,
    pub pane_id: u64,
    pub workspace: String,
    pub title: String,
    pub cwd: String,
}

#[derive(Debug, Clone)]
pub struct TerminalState {
    pub process_name: String,
    pub cwd: String,
    pub tab_count: usize,
    pub pane_count: usize,
}

/// Extract the process name from a pane title.
/// Titles look like: "nvim src/main.rs", "zsh", "/usr/bin/python3 script.py", "user@host: ~/dir"
pub fn extract_process_name(title: &str) -> &str {
    let first_token = title.split_whitespace().next().unwrap_or(title);
    // Strip path prefix: "/usr/bin/nvim" -> "nvim"
    first_token.rsplit('/').next().unwrap_or(first_token)
}

/// Convert a WezTerm cwd URI to a human-readable path.
/// "file://HOSTNAME/home/user/project" -> "~/project"
/// "file:///C:/Users/bhanu/projects" -> "C:/Users/bhanu/projects"
pub fn prettify_cwd(cwd: &str) -> String {
    // Strip the "file://" scheme and hostname
    let path = if cwd.starts_with("file:///") {
        // Windows path: file:///C:/Users/...
        &cwd[8..] // strip "file:///"
    } else if cwd.starts_with("file://") {
        // WSL path: file://HOSTNAME/home/user/...
        let after_scheme = &cwd[7..]; // strip "file://"
        // Skip the hostname part
        if let Some(slash_pos) = after_scheme.find('/') {
            &after_scheme[slash_pos..]
        } else {
            after_scheme
        }
    } else {
        cwd
    };

    // Replace /home/<user> with ~
    if let Some(rest) = path.strip_prefix("/home/") {
        // /home/esoteric/projects -> ~/projects
        if let Some(slash_pos) = rest.find('/') {
            let after_user = &rest[slash_pos..];
            if after_user == "/" {
                return "~".to_string();
            }
            return format!("~{}", after_user);
        } else {
            return "~".to_string();
        }
    }

    path.to_string()
}

impl TerminalState {
    /// Build state from a list of panes. Uses the first pane as the "active" one.
    pub fn from_panes(panes: Vec<Pane>) -> Self {
        let tab_count = panes.iter().map(|p| p.tab_id).collect::<HashSet<_>>().len();
        let pane_count = panes.len();

        // Use first pane as the active one (wezterm cli list shows focused first)
        let active = &panes[0];
        let process_name = extract_process_name(&active.title).to_string();
        let cwd = prettify_cwd(&active.cwd);

        Self {
            process_name,
            cwd,
            tab_count,
            pane_count,
        }
    }
}

/// Poll WezTerm CLI and return the current terminal state.
/// Returns None if WezTerm is not running or has no panes.
pub fn poll() -> Option<TerminalState> {
    let output = Command::new("wezterm")
        .args(["cli", "list", "--format", "json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let panes: Vec<Pane> = serde_json::from_str(&stdout).ok()?;

    if panes.is_empty() {
        return None;
    }

    Some(TerminalState::from_panes(panes))
}
```

Add to `src/main.rs`:

```rust
mod wezterm;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib wezterm`
Expected: All 7 tests PASS

**Step 5: Commit**

```bash
git add src/wezterm.rs src/main.rs
git commit -m "feat: add wezterm CLI polling and pane state parsing"
```

---

### Task 4: Discord Presence Module

**Files:**
- Create: `src/presence.rs`
- Modify: `src/main.rs` (add mod declaration)

**Step 1: Implement the presence module**

Note: Discord IPC requires a running Discord client, so this module is tested via integration (manual).
Unit tests cover the activity-building logic only.

`src/presence.rs`:

```rust
use discord_rich_presence::{
    activity::{self, Activity, Assets, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;
use crate::wezterm::TerminalState;

pub struct Presence {
    client: DiscordIpcClient,
    connected: bool,
    session_start: i64,
}

impl Presence {
    pub fn new(app_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = DiscordIpcClient::new(app_id)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        Ok(Self {
            client,
            connected: false,
            session_start: now,
        })
    }

    /// Try to connect to Discord. Returns true if connected.
    pub fn connect(&mut self) -> bool {
        if self.connected {
            return true;
        }
        match self.client.connect() {
            Ok(_) => {
                self.connected = true;
                eprintln!("Connected to Discord.");
                true
            }
            Err(e) => {
                eprintln!("Discord not available: {}", e);
                false
            }
        }
    }

    /// Update the Discord presence with the current terminal state.
    pub fn update(&mut self, state: &TerminalState, config: &Config) {
        if !self.connected {
            return;
        }

        let (details_text, small_icon) = config.resolve_process(&state.process_name);
        let state_text = format!("{}", state.cwd);
        let large_text = format!("{} tabs \u{00b7} {} panes", state.tab_count, state.pane_count);

        let activity = Activity::new()
            .details(&details_text)
            .state(&state_text)
            .timestamps(Timestamps::new().start(self.session_start))
            .assets(
                Assets::new()
                    .large_image("wezterm")
                    .large_text(&large_text)
                    .small_image(&small_icon)
                    .small_text(&state.process_name),
            );

        if let Err(e) = self.client.set_activity(activity) {
            eprintln!("Failed to set activity: {}", e);
            self.connected = false;
        }
    }

    /// Clear the Discord presence (when WezTerm is closed).
    pub fn clear(&mut self) {
        if !self.connected {
            return;
        }
        if let Err(e) = self.client.clear_activity() {
            eprintln!("Failed to clear activity: {}", e);
            self.connected = false;
        }
    }

    /// Reset the session start timestamp (e.g. when WezTerm restarts).
    pub fn reset_session_timer(&mut self) {
        self.session_start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
    }
}
```

Add to `src/main.rs`:

```rust
mod presence;
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/presence.rs src/main.rs
git commit -m "feat: add Discord presence module with activity updates"
```

---

### Task 5: Main Loop

**Files:**
- Modify: `src/main.rs`

**Step 1: Implement the main loop**

`src/main.rs`:

```rust
mod config;
mod presence;
mod wezterm;

use std::thread;
use std::time::Duration;

fn main() {
    eprintln!("wezterm-presence v{}", env!("CARGO_PKG_VERSION"));

    let config = config::Config::load();

    if config.discord_app_id.is_empty() || config.discord_app_id == "YOUR_APP_ID" {
        eprintln!("Error: discord_app_id not set in config.toml");
        eprintln!("Create a Discord application at https://discord.com/developers/applications");
        eprintln!("Then set discord_app_id in config.toml next to this executable.");
        std::process::exit(1);
    }

    let poll_interval = Duration::from_secs(config.poll_interval);
    let mut presence = match presence::Presence::new(&config.discord_app_id) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create Discord IPC client: {}", e);
            std::process::exit(1);
        }
    };

    let mut was_running = false;

    eprintln!("Polling every {}s. Press Ctrl+C to stop.", config.poll_interval);

    loop {
        // Try to connect if not connected
        if !presence.connect() {
            thread::sleep(Duration::from_secs(10));
            continue;
        }

        match wezterm::poll() {
            Some(state) => {
                if !was_running {
                    eprintln!("WezTerm detected.");
                    presence.reset_session_timer();
                    was_running = true;
                }
                presence.update(&state, &config);
            }
            None => {
                if was_running {
                    eprintln!("WezTerm closed. Clearing presence.");
                    presence.clear();
                    was_running = false;
                }
            }
        }

        thread::sleep(poll_interval);
    }
}
```

**Step 2: Verify it compiles for Windows**

Run: `cargo build --target x86_64-pc-windows-gnu`
Expected: Compiles successfully, binary at `target/x86_64-pc-windows-gnu/debug/wezterm-presence.exe`

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add main polling loop with connect/update/clear lifecycle"
```

---

### Task 6: Default Config File Generation

**Files:**
- Modify: `src/config.rs`
- Modify: `src/main.rs`

**Step 1: Add config generation method**

Add to `impl Config` in `src/config.rs`:

```rust
    /// Write a default config.toml to the given path.
    pub fn write_default(path: &std::path::Path) -> std::io::Result<()> {
        let content = r#"# WezTerm Discord Rich Presence Config
# Get your Discord Application ID from https://discord.com/developers/applications
discord_app_id = "YOUR_APP_ID"

# Poll interval in seconds
poll_interval = 3

# Process name -> display text and icon mappings
# Icon names must match assets uploaded to your Discord application
[processes]
claude = { text = "Using Claude Code", icon = "claude" }
nvim = { text = "Editing in Neovim", icon = "neovim" }
vim = { text = "Editing in Vim", icon = "vim" }
cargo = { text = "Building with Cargo", icon = "rust" }
rustc = { text = "Compiling Rust", icon = "rust" }
zsh = { text = "In the Shell", icon = "terminal" }
bash = { text = "In the Shell", icon = "terminal" }
fish = { text = "In the Shell", icon = "terminal" }
python = { text = "Running Python", icon = "python" }
python3 = { text = "Running Python", icon = "python" }
node = { text = "Running Node.js", icon = "nodejs" }
git = { text = "Using Git", icon = "git" }
docker = { text = "Running Docker", icon = "docker" }
ssh = { text = "Connected via SSH", icon = "ssh" }
htop = { text = "Monitoring System", icon = "terminal" }
btop = { text = "Monitoring System", icon = "terminal" }
make = { text = "Running Make", icon = "terminal" }
"#;
        std::fs::write(path, content)
    }
```

**Step 2: Add `init` subcommand to main.rs**

Update the beginning of `main()` in `src/main.rs` — add before the config loading:

```rust
    // Handle `init` subcommand: generate default config.toml
    if std::env::args().nth(1).as_deref() == Some("init") {
        let path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("config.toml")))
            .unwrap_or_else(|| std::path::PathBuf::from("config.toml"));
        if path.exists() {
            eprintln!("config.toml already exists at {}", path.display());
            std::process::exit(1);
        }
        config::Config::write_default(&path).expect("Failed to write config.toml");
        eprintln!("Created default config at {}", path.display());
        eprintln!("Edit it to set your discord_app_id, then run wezterm-presence.");
        return;
    }
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles

**Step 4: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add 'init' subcommand to generate default config.toml"
```

---

### Task 7: Auto-Start Installation

**Files:**
- Modify: `src/main.rs`

**Step 1: Add `install` subcommand**

Add another subcommand branch in `main()`, right after the `init` handler:

```rust
    // Handle `install` subcommand: create startup shortcut
    if std::env::args().nth(1).as_deref() == Some("install") {
        install_startup();
        return;
    }
```

Add this function to `src/main.rs`:

```rust
fn install_startup() {
    let exe_path = std::env::current_exe().expect("Failed to get executable path");

    // Create a .vbs script that launches the exe without a visible console window
    let startup_dir = dirs::config_dir()
        .map(|p| {
            p.parent()
                .unwrap()
                .join("Roaming")
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Startup")
        })
        .expect("Failed to find startup directory");

    let vbs_path = startup_dir.join("wezterm-presence.vbs");
    let vbs_content = format!(
        "Set WshShell = CreateObject(\"WScript.Shell\")\nWshShell.Run \"\"\"{}\"\"\", 0, False",
        exe_path.display()
    );

    std::fs::write(&vbs_path, vbs_content).expect("Failed to write startup script");
    eprintln!("Installed auto-start to: {}", vbs_path.display());
    eprintln!("wezterm-presence will start automatically on login.");
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add 'install' subcommand for Windows auto-start"
```

---

### Task 8: Build, Test, and Release

**Files:**
- No new files

**Step 1: Run all unit tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Build release binary for Windows**

Run: `cargo build --release --target x86_64-pc-windows-gnu`
Expected: Binary at `target/x86_64-pc-windows-gnu/release/wezterm-presence.exe`

**Step 3: Manual integration test**

1. Copy `wezterm-presence.exe` to a folder on Windows (e.g. `C:\Tools\wezterm-presence\`)
2. Run `wezterm-presence.exe init` to generate `config.toml`
3. Edit `config.toml` — set your `discord_app_id`
4. Open WezTerm and Discord
5. Run `wezterm-presence.exe` — verify Discord shows your terminal activity
6. Close WezTerm — verify presence clears
7. Run `wezterm-presence.exe install` to add auto-start

**Step 4: Commit any fixes from testing**

```bash
git add -A
git commit -m "fix: adjustments from integration testing"
```

**Step 5: Tag the release**

```bash
git tag v0.1.0
```
