use serde::Deserialize;
use std::collections::HashSet;
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct Pane {
    #[allow(dead_code)]
    pub window_id: u64,
    pub tab_id: u64,
    #[allow(dead_code)]
    pub pane_id: u64,
    #[allow(dead_code)]
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

pub fn extract_process_name(title: &str) -> &str {
    let first_token = title.split_whitespace().next().unwrap_or(title);
    first_token.rsplit('/').next().unwrap_or(first_token)
}

pub fn prettify_cwd(cwd: &str) -> String {
    let path = if let Some(stripped) = cwd.strip_prefix("file:///") {
        stripped
    } else if let Some(after_scheme) = cwd.strip_prefix("file://") {
        if let Some(slash_pos) = after_scheme.find('/') {
            &after_scheme[slash_pos..]
        } else {
            after_scheme
        }
    } else {
        cwd
    };

    if let Some(rest) = path.strip_prefix("/home/") {
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
    pub fn from_panes(panes: Vec<Pane>) -> Self {
        let tab_count = panes.iter().map(|p| p.tab_id).collect::<HashSet<_>>().len();
        let pane_count = panes.len();
        let active = &panes[0];
        let process_name = extract_process_name(&active.title).to_string();
        let cwd = prettify_cwd(&active.cwd);
        Self { process_name, cwd, tab_count, pane_count }
    }
}

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
        assert_eq!(state.process_name, "nvim");
        assert_eq!(state.cwd, "~/wezterm-presence");
    }
}
