pub mod claude_code;
pub mod codex;

use chrono::{DateTime, Utc};

use crate::config::Config;
use crate::model::WindowUsage;

/// A source of rate-limit windows for one LLM product.
///
/// Providers only ever read data the user's own tooling already wrote
/// (transcripts, session logs, stored sessions). They never prompt for
/// credentials and never write anything.
pub trait Provider {
    fn name(&self) -> &'static str;

    /// Whether the product's data directory exists on this machine.
    fn detected(&self) -> bool;

    fn windows(&self, now: DateTime<Utc>) -> Result<Vec<WindowUsage>, String>;
}

/// Detected providers only — having an app not installed is normal,
/// not a warning.
pub fn enabled_providers(config: &Config) -> Vec<Box<dyn Provider>> {
    let all: Vec<Box<dyn Provider>> = vec![
        Box::new(claude_code::ClaudeCode::from_home(
            config.budgets.claude_code,
        )),
        Box::new(codex::Codex::from_home()),
    ];
    all.into_iter().filter(|p| p.detected()).collect()
}
