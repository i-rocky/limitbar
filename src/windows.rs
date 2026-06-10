use chrono::{DateTime, Duration, DurationRound, Utc};

use crate::model::{TokenCounts, UsageEvent, WindowUsage};

const SESSION_HOURS: i64 = 5;
const WEEK_DAYS: i64 = 7;

/// Usage inside the current 5-hour session block.
///
/// Mirrors how Claude's session windows behave: a block opens at the top
/// of the hour of the first request after the previous block expired and
/// lasts five hours. Returns an empty window when no block is active.
pub fn session_block(
    provider: &'static str,
    events: &[UsageEvent],
    now: DateTime<Utc>,
    budget: Option<u64>,
) -> WindowUsage {
    let mut block_start: Option<DateTime<Utc>> = None;
    let mut tokens = TokenCounts::default();
    let mut count = 0usize;

    for event in events {
        match block_start {
            Some(start) if event.timestamp < start + Duration::hours(SESSION_HOURS) => {}
            _ => {
                block_start = Some(floor_to_hour(event.timestamp));
                tokens = TokenCounts::default();
                count = 0;
            }
        }
        tokens.add(event.tokens);
        count += 1;
    }

    let active = block_start.filter(|start| now < *start + Duration::hours(SESSION_HOURS));
    if active.is_none() {
        tokens = TokenCounts::default();
        count = 0;
    }

    WindowUsage {
        provider,
        label: "5h".to_string(),
        tokens,
        events: count,
        fraction: None,
        resets_at: active.map(|start| start + Duration::hours(SESSION_HOURS)),
        budget_tokens: budget,
    }
}

/// Rolling usage over the past seven days.
pub fn rolling_week(
    provider: &'static str,
    events: &[UsageEvent],
    now: DateTime<Utc>,
    budget: Option<u64>,
) -> WindowUsage {
    let cutoff = now - Duration::days(WEEK_DAYS);
    let mut tokens = TokenCounts::default();
    let mut count = 0usize;

    for event in events.iter().filter(|e| e.timestamp >= cutoff) {
        tokens.add(event.tokens);
        count += 1;
    }

    WindowUsage {
        provider,
        label: "7d".to_string(),
        tokens,
        events: count,
        fraction: None,
        resets_at: None,
        budget_tokens: budget,
    }
}

fn floor_to_hour(ts: DateTime<Utc>) -> DateTime<Utc> {
    ts.duration_trunc(Duration::hours(1)).unwrap_or(ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(iso: &str) -> DateTime<Utc> {
        iso.parse().expect("valid timestamp")
    }

    fn event(iso: &str, total: u64) -> UsageEvent {
        UsageEvent {
            timestamp: at(iso),
            tokens: TokenCounts {
                input: total,
                ..Default::default()
            },
        }
    }

    #[test]
    fn session_block_counts_current_block_only() {
        let events = vec![
            event("2026-06-10T01:30:00Z", 100), // old block (expired by 09:00)
            event("2026-06-10T08:10:00Z", 40),  // current block starts 08:00
            event("2026-06-10T09:20:00Z", 60),
        ];
        let usage = session_block("p", &events, at("2026-06-10T09:30:00Z"), None);
        assert_eq!(usage.tokens.total(), 100);
        assert_eq!(usage.events, 2);
        assert_eq!(usage.resets_at, Some(at("2026-06-10T13:00:00Z")));
    }

    #[test]
    fn session_block_empty_when_idle_past_window() {
        let events = vec![event("2026-06-10T01:30:00Z", 100)];
        let usage = session_block("p", &events, at("2026-06-10T09:30:00Z"), None);
        assert_eq!(usage.tokens.total(), 0);
        assert_eq!(usage.events, 0);
        assert_eq!(usage.resets_at, None);
    }

    #[test]
    fn session_block_spans_full_five_hours() {
        let events = vec![
            event("2026-06-10T08:10:00Z", 10),
            event("2026-06-10T12:59:00Z", 20), // 08:00 + 5h ends 13:00
        ];
        let usage = session_block("p", &events, at("2026-06-10T12:59:30Z"), None);
        assert_eq!(usage.tokens.total(), 30);
    }

    #[test]
    fn rolling_week_filters_old_events() {
        let events = vec![
            event("2026-06-01T00:00:00Z", 100), // > 7 days old
            event("2026-06-08T00:00:00Z", 40),
        ];
        let usage = rolling_week("p", &events, at("2026-06-10T00:00:00Z"), None);
        assert_eq!(usage.tokens.total(), 40);
        assert_eq!(usage.events, 1);
    }

    #[test]
    fn used_fraction_requires_budget() {
        let usage = rolling_week(
            "p",
            &[event("2026-06-09T00:00:00Z", 50)],
            at("2026-06-10T00:00:00Z"),
            Some(200),
        );
        assert_eq!(usage.used_fraction(), Some(0.25));
        let no_budget = rolling_week("p", &[], at("2026-06-10T00:00:00Z"), None);
        assert_eq!(no_budget.used_fraction(), None);
    }
}
