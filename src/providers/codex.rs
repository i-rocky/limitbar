use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::model::{TokenCounts, WindowUsage};
use crate::providers::Provider;

/// Reads rate-limit state from Codex CLI's local session logs under
/// `~/.codex/sessions/YYYY/MM/DD/*.jsonl`. Codex records the official
/// used percentage for each window with every `token_count` event, so
/// no budget estimation is needed. Fully offline; no credentials touched.
pub struct Codex {
    sessions_dir: PathBuf,
}

impl Codex {
    pub fn from_home() -> Self {
        let sessions_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".codex")
            .join("sessions");
        Self { sessions_dir }
    }

    #[cfg(test)]
    pub fn with_dir(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }
}

impl Provider for Codex {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn detected(&self) -> bool {
        self.sessions_dir.is_dir()
    }

    fn windows(&self, now: DateTime<Utc>) -> Result<Vec<WindowUsage>, String> {
        let mut files = collect_jsonl(&self.sessions_dir)?;
        // Session filenames embed their start time, so path order is
        // chronological; scan from the newest file backwards.
        files.sort();

        for path in files.iter().rev() {
            if let Some(snapshot) = last_rate_limits(path) {
                return Ok(snapshot.into_windows(now));
            }
        }

        Err("no rate-limit events in any Codex session".to_string())
    }
}

fn collect_jsonl(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| format!("failed to read {}: {e}", dir.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "jsonl") {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn last_rate_limits(path: &Path) -> Option<RateLimitSnapshot> {
    let file = File::open(path).ok()?;
    let mut latest = None;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else { break };
        if let Some(snapshot) = parse_line(&line) {
            latest = Some(snapshot);
        }
    }
    latest
}

#[derive(Debug, PartialEq)]
struct RateLimitSnapshot {
    primary: Option<RateLimitWindow>,
    secondary: Option<RateLimitWindow>,
}

#[derive(Debug, PartialEq)]
struct RateLimitWindow {
    used_percent: f64,
    window_minutes: u64,
    resets_at: Option<DateTime<Utc>>,
}

impl RateLimitSnapshot {
    fn into_windows(self, now: DateTime<Utc>) -> Vec<WindowUsage> {
        [self.primary, self.secondary]
            .into_iter()
            .flatten()
            .map(|window| window.into_usage(now))
            .collect()
    }
}

impl RateLimitWindow {
    fn into_usage(self, now: DateTime<Utc>) -> WindowUsage {
        // A reset in the past means the logged percentage belongs to an
        // expired window; the live value is back to zero.
        let stale = self.resets_at.is_some_and(|resets| resets <= now);
        WindowUsage {
            provider: "codex",
            label: window_label(self.window_minutes),
            tokens: TokenCounts::default(),
            events: 0,
            fraction: Some(if stale {
                0.0
            } else {
                self.used_percent / 100.0
            }),
            resets_at: self.resets_at.filter(|_| !stale),
            budget_tokens: None,
        }
    }
}

fn window_label(minutes: u64) -> String {
    match minutes {
        300 => "5h".to_string(),
        10080 => "7d".to_string(),
        m if m % 1440 == 0 => format!("{}d", m / 1440),
        m if m % 60 == 0 => format!("{}h", m / 60),
        m => format!("{m}m"),
    }
}

#[derive(Deserialize)]
struct SessionLine {
    payload: Option<Payload>,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "type")]
    payload_type: Option<String>,
    rate_limits: Option<RateLimits>,
}

#[derive(Deserialize)]
struct RateLimits {
    primary: Option<RawWindow>,
    secondary: Option<RawWindow>,
}

#[derive(Deserialize)]
struct RawWindow {
    used_percent: Option<f64>,
    window_minutes: Option<u64>,
    resets_at: Option<i64>,
}

fn parse_line(line: &str) -> Option<RateLimitSnapshot> {
    if !line.contains("\"rate_limits\"") {
        return None;
    }

    let parsed: SessionLine = serde_json::from_str(line).ok()?;
    let payload = parsed.payload?;
    if payload.payload_type.as_deref() != Some("token_count") {
        return None;
    }
    let limits = payload.rate_limits?;

    let convert = |raw: RawWindow| -> Option<RateLimitWindow> {
        Some(RateLimitWindow {
            used_percent: raw.used_percent?,
            window_minutes: raw.window_minutes?,
            resets_at: raw.resets_at.and_then(|s| DateTime::from_timestamp(s, 0)),
        })
    };

    Some(RateLimitSnapshot {
        primary: limits.primary.and_then(convert),
        secondary: limits.secondary.and_then(convert),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN_COUNT_LINE: &str = r#"{"timestamp":"2026-04-10T05:45:17.634Z","type":"event_msg","payload":{"type":"token_count","info":null,"rate_limits":{"limit_id":"codex","primary":{"used_percent":14.0,"window_minutes":300,"resets_at":1775816320},"secondary":{"used_percent":7.0,"window_minutes":10080,"resets_at":1776358279}}}}"#;

    fn at(iso: &str) -> DateTime<Utc> {
        iso.parse().expect("valid timestamp")
    }

    #[test]
    fn parses_rate_limit_line() {
        let snapshot = parse_line(TOKEN_COUNT_LINE).expect("should parse");
        let primary = snapshot.primary.expect("primary window");
        assert_eq!(primary.used_percent, 14.0);
        assert_eq!(primary.window_minutes, 300);
        let secondary = snapshot.secondary.expect("secondary window");
        assert_eq!(secondary.window_minutes, 10080);
    }

    #[test]
    fn active_window_keeps_percentage_and_reset() {
        let snapshot = parse_line(TOKEN_COUNT_LINE).expect("should parse");
        // resets_at 1775816320 is 2026-04-10T05:38:40Z; pick `now` before it.
        let windows = snapshot.into_windows(at("2026-04-10T05:00:00Z"));
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].label, "5h");
        assert_eq!(windows[0].used_fraction(), Some(0.14));
        assert!(windows[0].resets_at.is_some());
        assert_eq!(windows[1].label, "7d");
        assert_eq!(windows[1].used_fraction(), Some(0.07));
    }

    #[test]
    fn expired_window_reads_as_zero() {
        let snapshot = parse_line(TOKEN_COUNT_LINE).expect("should parse");
        let windows = snapshot.into_windows(at("2027-01-01T00:00:00Z"));
        assert_eq!(windows[0].used_fraction(), Some(0.0));
        assert_eq!(windows[0].resets_at, None);
    }

    #[test]
    fn ignores_other_lines() {
        assert!(
            parse_line(r#"{"type":"session_meta","payload":{"type":"session_meta"}}"#).is_none()
        );
    }

    #[test]
    fn window_labels() {
        assert_eq!(window_label(300), "5h");
        assert_eq!(window_label(10080), "7d");
        assert_eq!(window_label(2880), "2d");
        assert_eq!(window_label(120), "2h");
        assert_eq!(window_label(90), "90m");
    }

    #[test]
    fn newest_session_wins() {
        let dir = std::env::temp_dir().join(format!("limitbar-codex-{}", std::process::id()));
        let day = dir.join("2026").join("04").join("10");
        std::fs::create_dir_all(&day).expect("create temp dirs");
        let old = TOKEN_COUNT_LINE.replace("14.0", "99.0");
        std::fs::write(day.join("rollout-2026-04-10T01-00-00-a.jsonl"), old).expect("write");
        std::fs::write(
            day.join("rollout-2026-04-10T05-45-17-b.jsonl"),
            TOKEN_COUNT_LINE,
        )
        .expect("write");

        let provider = Codex::with_dir(dir.clone());
        assert!(provider.detected());
        let windows = provider
            .windows(at("2026-04-10T05:00:00Z"))
            .expect("windows");
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(windows[0].used_fraction(), Some(0.14));
    }
}
