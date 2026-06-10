use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenCounts {
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
}

impl TokenCounts {
    pub fn total(self) -> u64 {
        self.input + self.output + self.cache_creation + self.cache_read
    }

    pub fn add(&mut self, other: TokenCounts) {
        self.input += other.input;
        self.output += other.output;
        self.cache_creation += other.cache_creation;
        self.cache_read += other.cache_read;
    }
}

/// One billed assistant turn, deduplicated by message id.
#[derive(Debug, Clone)]
pub struct UsageEvent {
    pub timestamp: DateTime<Utc>,
    pub tokens: TokenCounts,
}

/// Aggregated usage over one rate-limit window.
#[derive(Debug, Clone)]
pub struct WindowUsage {
    pub provider: &'static str,
    pub label: &'static str,
    pub tokens: TokenCounts,
    pub events: usize,
    /// When this window resets, if the window is active.
    pub resets_at: Option<DateTime<Utc>>,
    /// Budget from config, for percentage display.
    pub budget_tokens: Option<u64>,
}

impl WindowUsage {
    pub fn used_fraction(&self) -> Option<f64> {
        self.budget_tokens
            .filter(|b| *b > 0)
            .map(|b| self.tokens.total() as f64 / b as f64)
    }
}
