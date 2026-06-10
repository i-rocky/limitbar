pub mod claude_code;

use crate::model::UsageEvent;

/// A source of usage events for one LLM product.
///
/// Providers return raw per-turn events; window math lives in
/// `crate::windows` so every provider gets the same treatment.
pub trait Provider {
    fn name(&self) -> &'static str;
    fn collect(&self) -> Result<Vec<UsageEvent>, String>;
}

pub fn enabled_providers() -> Vec<Box<dyn Provider>> {
    vec![Box::new(claude_code::ClaudeCode::from_home())]
}
