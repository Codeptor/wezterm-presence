use discord_rich_presence::{
    activity::{Activity, Assets, Timestamps},
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

    pub fn update(&mut self, state: &TerminalState, config: &Config) {
        if !self.connected {
            return;
        }

        let (details_text, small_icon) = config.resolve_process(&state.process_name);
        let state_text = &state.cwd;
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

    pub fn clear(&mut self) {
        if !self.connected {
            return;
        }
        let _ = self.client.clear_activity();
        let _ = self.client.close();
        self.connected = false;
    }

    pub fn reset_session_timer(&mut self) {
        self.session_start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
    }
}
