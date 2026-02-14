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

    fn config_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("config.toml")))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    pub fn resolve_process(&self, name: &str) -> (String, String) {
        if let Some(mapping) = self.processes.get(name) {
            (mapping.text.clone(), mapping.icon.clone())
        } else {
            (format!("Running {}", name), "terminal".to_string())
        }
    }
}

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
