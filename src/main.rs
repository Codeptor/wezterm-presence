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
