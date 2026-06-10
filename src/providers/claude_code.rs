use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::config::Budget;
use crate::model::{TokenCounts, UsageEvent, WindowUsage};
use crate::providers::Provider;
use crate::windows;

/// Reads token usage from Claude Code's local transcripts under
/// `~/.claude/projects/**/*.jsonl`. Fully offline; no credentials touched.
pub struct ClaudeCode {
    projects_dir: PathBuf,
    budget: Budget,
}

impl ClaudeCode {
    pub fn from_home(budget: Budget) -> Self {
        let projects_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude")
            .join("projects");
        Self {
            projects_dir,
            budget,
        }
    }

    #[cfg(test)]
    pub fn with_dir(projects_dir: PathBuf) -> Self {
        Self {
            projects_dir,
            budget: Budget::default(),
        }
    }

    fn collect(&self) -> Result<Vec<UsageEvent>, String> {
        let mut events = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![self.projects_dir.clone()];

        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir)
                .map_err(|e| format!("failed to read {}: {e}", dir.display()))?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|ext| ext == "jsonl") {
                    collect_file(&path, &mut events, &mut seen)?;
                }
            }
        }

        events.sort_by_key(|e| e.timestamp);
        Ok(events)
    }
}

impl Provider for ClaudeCode {
    fn name(&self) -> &'static str {
        "claude-code"
    }

    fn detected(&self) -> bool {
        self.projects_dir.is_dir()
    }

    fn windows(&self, now: DateTime<Utc>) -> Result<Vec<WindowUsage>, String> {
        let events = self.collect()?;
        Ok(vec![
            windows::session_block("claude-code", &events, now, self.budget.five_hour_tokens),
            windows::rolling_week("claude-code", &events, now, self.budget.weekly_tokens),
        ])
    }
}

fn collect_file(
    path: &Path,
    events: &mut Vec<UsageEvent>,
    seen: &mut HashSet<String>,
) -> Result<(), String> {
    let file = File::open(path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else { break };
        if let Some(event) = parse_line(&line, seen) {
            events.push(event);
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct TranscriptLine {
    #[serde(rename = "type")]
    line_type: String,
    timestamp: Option<DateTime<Utc>>,
    message: Option<TranscriptMessage>,
}

#[derive(Deserialize)]
struct TranscriptMessage {
    id: Option<String>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

/// Streamed responses repeat the same message id with identical usage
/// across several transcript lines, so the id set is the dedupe key.
fn parse_line(line: &str, seen: &mut HashSet<String>) -> Option<UsageEvent> {
    if !line.contains("\"usage\"") {
        return None;
    }

    let parsed: TranscriptLine = serde_json::from_str(line).ok()?;
    if parsed.line_type != "assistant" {
        return None;
    }

    let timestamp = parsed.timestamp?;
    let message = parsed.message?;
    let usage = message.usage?;
    let id = message.id?;
    if !seen.insert(id) {
        return None;
    }

    Some(UsageEvent {
        timestamp,
        tokens: TokenCounts {
            input: usage.input_tokens,
            output: usage.output_tokens,
            cache_creation: usage.cache_creation_input_tokens,
            cache_read: usage.cache_read_input_tokens,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASSISTANT_LINE: &str = r#"{"type":"assistant","timestamp":"2026-06-09T17:20:00.000Z","message":{"id":"msg_1","model":"claude-fable-5","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":30,"cache_read_input_tokens":20}}}"#;

    #[test]
    fn parses_assistant_usage_line() {
        let mut seen = HashSet::new();
        let event = parse_line(ASSISTANT_LINE, &mut seen).expect("should parse");
        assert_eq!(event.tokens.input, 100);
        assert_eq!(event.tokens.output, 50);
        assert_eq!(event.tokens.cache_creation, 30);
        assert_eq!(event.tokens.cache_read, 20);
        assert_eq!(event.tokens.total(), 200);
    }

    #[test]
    fn collects_events_from_nested_jsonl_files() {
        let dir = std::env::temp_dir().join(format!("limitbar-test-{}", std::process::id()));
        let nested = dir.join("project-a");
        std::fs::create_dir_all(&nested).expect("create temp dirs");
        std::fs::write(nested.join("session.jsonl"), format!("{ASSISTANT_LINE}\n")).expect("write");
        std::fs::write(dir.join("notes.txt"), "ignored").expect("write");

        let events = ClaudeCode::with_dir(dir.clone())
            .collect()
            .expect("collect");
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tokens.total(), 200);
    }

    #[test]
    fn dedupes_repeated_message_ids() {
        let mut seen = HashSet::new();
        assert!(parse_line(ASSISTANT_LINE, &mut seen).is_some());
        assert!(parse_line(ASSISTANT_LINE, &mut seen).is_none());
    }

    #[test]
    fn ignores_non_assistant_lines() {
        let mut seen = HashSet::new();
        let user = r#"{"type":"user","timestamp":"2026-06-09T17:20:00.000Z","message":{"role":"user","content":"\"usage\""}}"#;
        assert!(parse_line(user, &mut seen).is_none());
    }

    #[test]
    fn ignores_lines_without_usage() {
        let mut seen = HashSet::new();
        let line = r#"{"type":"assistant","timestamp":"2026-06-09T17:20:00.000Z","message":{"id":"msg_2","model":"m"}}"#;
        assert!(parse_line(line, &mut seen).is_none());
    }
}
