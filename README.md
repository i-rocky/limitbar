# limitbar

Always-visible usage meter for LLM rate-limit windows.

LLM subscriptions meter you in invisible windows — Claude's 5-hour session
and weekly caps being the prime example — and you only find out when you hit
the wall. `limitbar` keeps the current windows on screen: in your terminal,
or as a tiny translucent click-through bar floating above your editor.

```
claude-code  5h   ████████░░░░░░░░  52.3%    109.9M tokens /  793 reqs  resets 15:00 (3h 52m)
claude-code  7d   ███░░░░░░░░░░░░░  21.0%      1.7B tokens / 4940 reqs
```

## Usage

```sh
limitbar              # print current windows once
limitbar -w 10        # live-refresh in the terminal every 10s
limitbar --overlay    # floating always-on-top click-through bar
```

The overlay requires building with the `overlay` feature:

```sh
cargo install --git https://github.com/i-rocky/limitbar --features overlay
```

## Providers

### claude-code

Reads Claude Code's local transcripts (`~/.claude/projects/**/*.jsonl`) —
fully offline, no credentials touched, no network calls. Token counts are
exact (deduplicated per billed response); the 5-hour session block mirrors
how Claude's windows open (top of the hour of the first request after the
previous block expires).

**Honesty note:** Anthropic does not publish the token budgets behind the
session/weekly caps, so absolute percentages need a budget you supply in
config. Without one, limitbar shows raw totals and reset times — which is
usually what you act on anyway.

More providers (Codex CLI, API rate-limit headers) are planned; the
`Provider` trait in `src/providers/` is the seam.

## Configuration

`~/.config/limitbar/config.toml` (all optional):

```toml
[budgets.claude-code]
five_hour_tokens = 500000000
weekly_tokens = 3000000000
```

With budgets set, the gauges show percentages; without, raw totals.

## Development

```sh
cargo test
cargo clippy --all-targets --features overlay -- -D warnings
```

## License

MIT
