use chrono::{DateTime, Utc};

use crate::config::Config;
use crate::model::WindowUsage;
use crate::providers::enabled_providers;
use crate::windows;

/// Collects every enabled provider into renderable windows.
/// Provider failures become warnings instead of killing the snapshot,
/// so one broken source never blanks the whole bar.
pub fn collect(config: &Config, now: DateTime<Utc>) -> (Vec<WindowUsage>, Vec<String>) {
    let mut usages = Vec::new();
    let mut warnings = Vec::new();

    for provider in enabled_providers() {
        match provider.collect() {
            Ok(events) => {
                let budget = &config.budgets.claude_code;
                usages.push(windows::session_block(
                    provider.name(),
                    &events,
                    now,
                    budget.five_hour_tokens,
                ));
                usages.push(windows::rolling_week(
                    provider.name(),
                    &events,
                    now,
                    budget.weekly_tokens,
                ));
            }
            Err(err) => warnings.push(format!("{}: {err}", provider.name())),
        }
    }

    (usages, warnings)
}
