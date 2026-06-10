use chrono::{DateTime, Utc};

use crate::config::Config;
use crate::model::WindowUsage;
use crate::providers::enabled_providers;

/// Collects every detected provider into renderable windows.
/// Provider failures become warnings instead of killing the snapshot,
/// so one broken source never blanks the whole bar.
pub fn collect(config: &Config, now: DateTime<Utc>) -> (Vec<WindowUsage>, Vec<String>) {
    let mut usages = Vec::new();
    let mut warnings = Vec::new();

    for provider in enabled_providers(config) {
        match provider.windows(now) {
            Ok(windows) => usages.extend(windows),
            Err(err) => warnings.push(format!("{}: {err}", provider.name())),
        }
    }

    (usages, warnings)
}
