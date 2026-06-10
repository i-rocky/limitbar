use std::path::PathBuf;

use serde::Deserialize;

/// `~/.config/limitbar/config.toml`
///
/// ```toml
/// [budgets.claude-code]
/// five_hour_tokens = 5000000
/// weekly_tokens = 60000000
///
/// [overlay]
/// background = "#0d1117"
/// opacity = 0.85
/// text = "#e6edf3"
/// font_size = 12.0
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub budgets: Budgets,
    #[serde(default)]
    pub overlay: Overlay,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Overlay {
    /// Window background as #RRGGBB or #RRGGBBAA.
    pub background: Option<String>,
    /// Background opacity 0.0–1.0, multiplied onto the background alpha.
    pub opacity: Option<f32>,
    /// Text color as #RRGGBB or #RRGGBBAA.
    pub text: Option<String>,
    pub font_size: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Budgets {
    #[serde(rename = "claude-code", default)]
    pub claude_code: Budget,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Budget {
    pub five_hour_tokens: Option<u64>,
    pub weekly_tokens: Option<u64>,
}

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("limitbar").join("config.toml"))
}

pub fn load() -> Result<Config, String> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };
    if !path.is_file() {
        return Ok(Config::default());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    toml::from_str(&raw).map_err(|e| format!("invalid config {}: {e}", path.display()))
}

/// Runtime state (e.g. the dragged overlay position) lives in its own file
/// so writing it back never touches the user's hand-edited config.
pub fn state_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("limitbar").join("state.toml"))
}

#[derive(Debug, Clone, Copy, Default, Deserialize, serde::Serialize)]
pub struct State {
    pub overlay_position: Option<[f32; 2]>,
}

pub fn load_state() -> State {
    let Some(path) = state_path() else {
        return State::default();
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| toml::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_state(state: &State) {
    let Some(path) = state_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = toml::to_string(state) {
        let _ = std::fs::write(&path, raw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_budgets() {
        let config: Config = toml::from_str(
            r#"
            [budgets.claude-code]
            five_hour_tokens = 1000
            weekly_tokens = 2000
            "#,
        )
        .expect("valid config");
        assert_eq!(config.budgets.claude_code.five_hour_tokens, Some(1000));
        assert_eq!(config.budgets.claude_code.weekly_tokens, Some(2000));
    }

    #[test]
    fn empty_config_defaults() {
        let config: Config = toml::from_str("").expect("valid config");
        assert_eq!(config.budgets.claude_code.five_hour_tokens, None);
    }
}
