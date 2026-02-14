mod config;
mod presence;
mod wezterm;

use std::thread;
use std::time::Duration;

fn main() {
    eprintln!("wezterm-presence v{}", env!("CARGO_PKG_VERSION"));

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

    // Handle `install` subcommand: create startup shortcut
    if std::env::args().nth(1).as_deref() == Some("install") {
        install_startup();
        return;
    }

    let config = config::Config::load();
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

fn install_startup() {
    let exe_path = std::env::current_exe().expect("Failed to get executable path");

    let startup_dir = dirs::config_dir()
        .map(|p| {
            p.join("Microsoft")
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
