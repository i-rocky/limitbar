use chrono::{DateTime, Local, Utc};

use crate::model::WindowUsage;

const BAR_WIDTH: usize = 16;

pub fn render_line(usage: &WindowUsage, now: DateTime<Utc>) -> String {
    let mut line = format!("{:<12} {:<3} ", usage.provider, usage.label);

    match usage.used_fraction() {
        Some(fraction) => {
            line.push_str(&bar(fraction));
            line.push_str(&format!(" {:>5.1}%", fraction * 100.0));
        }
        None => {
            line.push_str(&" ".repeat(BAR_WIDTH));
            line.push_str("       ");
        }
    }

    line.push_str(&format!(
        " {:>9} tokens / {:>4} reqs",
        compact(usage.tokens.total()),
        usage.events
    ));

    if let Some(resets_at) = usage.resets_at {
        let remaining = resets_at.signed_duration_since(now);
        let minutes = remaining.num_minutes().max(0);
        line.push_str(&format!(
            "  resets {} ({}h {:02}m)",
            resets_at.with_timezone(&Local).format("%H:%M"),
            minutes / 60,
            minutes % 60
        ));
    }

    line
}

fn bar(fraction: f64) -> String {
    let clamped = fraction.clamp(0.0, 1.0);
    let filled = (clamped * BAR_WIDTH as f64).round() as usize;
    let mut out = String::with_capacity(BAR_WIDTH * 3);
    for i in 0..BAR_WIDTH {
        out.push(if i < filled { '█' } else { '░' });
    }
    out
}

pub fn compact(value: u64) -> String {
    if value >= 1_000_000_000 {
        format!("{:.1}B", value as f64 / 1e9)
    } else if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1e6)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f64 / 1e3)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TokenCounts;

    #[test]
    fn compact_formats() {
        assert_eq!(compact(950), "950");
        assert_eq!(compact(1_500), "1.5k");
        assert_eq!(compact(2_400_000), "2.4M");
        assert_eq!(compact(3_100_000_000), "3.1B");
    }

    #[test]
    fn bar_is_fixed_width() {
        assert_eq!(bar(0.0).chars().count(), BAR_WIDTH);
        assert_eq!(bar(0.5).chars().count(), BAR_WIDTH);
        assert_eq!(bar(2.0).chars().count(), BAR_WIDTH);
    }

    #[test]
    fn render_line_without_budget_shows_tokens() {
        let usage = WindowUsage {
            provider: "claude-code",
            label: "7d",
            tokens: TokenCounts {
                input: 1_500_000,
                ..Default::default()
            },
            events: 3,
            resets_at: None,
            budget_tokens: None,
        };
        let line = render_line(&usage, Utc::now());
        assert!(line.contains("1.5M tokens"));
        assert!(!line.contains('%'));
    }
}
