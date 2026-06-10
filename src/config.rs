use std::path::PathBuf;

use serde::Deserialize;

/// `~/.config/limitbar/config.toml`
///
/// ```toml
/// [budgets.claude-code]
/// five_hour_tokens = 5000000
/// weekly_tokens = 60000000
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub budgets: Budgets,
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
