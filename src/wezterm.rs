use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalState {
    pub process_name: String,
    pub cwd: String,
    pub tab_count: usize,
    pub pane_count: usize,
}

/// JSON structure written by WezTerm's Lua update-status handler.
#[derive(Debug, Deserialize)]
struct LuaState {
    process: String,
    cwd: String,
    tabs: usize,
    panes: usize,
}

/// Prettify a cwd path: collapse /home/<user>/... to ~/...
pub fn prettify_cwd(cwd: &str) -> String {
    // Strip file:// URI prefix if present
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

/// Get the state file path: %LOCALAPPDATA%\wezterm-presence\state.json
fn state_file_path() -> Option<std::path::PathBuf> {
    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    Some(std::path::Path::new(&local_app_data).join("wezterm-presence").join("state.json"))
}

/// Poll WezTerm state by reading the JSON file written by the Lua config.
/// Returns None if WezTerm isn't running or the file is stale.
pub fn poll() -> Option<TerminalState> {
    let path = state_file_path()?;

    // Check if file exists and is fresh (modified within last 10 seconds)
    let metadata = std::fs::metadata(&path).ok()?;
    let age = metadata.modified().ok()?.elapsed().ok()?;
    if age.as_secs() > 120 {
        // File is stale for 2+ minutes -- WezTerm probably closed
        return None;
    }

    let contents = std::fs::read_to_string(&path).ok()?;
    let lua_state: LuaState = serde_json::from_str(&contents).ok()?;

    if lua_state.process.is_empty() || lua_state.process == "unknown" {
        return None;
    }

    Some(TerminalState {
        process_name: lua_state.process,
        cwd: prettify_cwd(&lua_state.cwd),
        tab_count: lua_state.tabs,
        pane_count: lua_state.panes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lua_state() {
        let json = r#"{"process":"nvim","cwd":"/home/esoteric/project","tabs":3,"panes":4}"#;
        let state: LuaState = serde_json::from_str(json).unwrap();
        assert_eq!(state.process, "nvim");
        assert_eq!(state.cwd, "/home/esoteric/project");
        assert_eq!(state.tabs, 3);
        assert_eq!(state.panes, 4);
    }

    #[test]
    fn test_prettify_cwd_wsl() {
        assert_eq!(prettify_cwd("/home/esoteric/wezterm-presence"), "~/wezterm-presence");
        assert_eq!(prettify_cwd("/home/esoteric"), "~");
    }

    #[test]
    fn test_prettify_cwd_file_uri() {
        assert_eq!(
            prettify_cwd("file://DESKTOP-ABC/home/esoteric/wezterm-presence"),
            "~/wezterm-presence"
        );
        assert_eq!(
            prettify_cwd("file:///C:/Users/bhanu/projects"),
            "C:/Users/bhanu/projects"
        );
    }

    #[test]
    fn test_prettify_cwd_plain_path() {
        assert_eq!(prettify_cwd("/tmp/something"), "/tmp/something");
        assert_eq!(prettify_cwd("C:/Users/bhanu"), "C:/Users/bhanu");
    }
}
