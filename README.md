# limitbar

Always-visible usage meter for LLM rate-limit windows.

LLM subscriptions meter you in invisible windows — Claude's 5-hour session
and weekly caps, Codex's 5-hour and weekly limits — and you only find out
when you hit the wall. `limitbar` keeps the current windows on screen: in
your terminal, or as a tiny translucent click-through bar floating above
your editor.

```
claude-code  5h   ████████░░░░░░░░  52.3%    109.9M tokens /  793 reqs  resets 15:00 (3h 52m)
claude-code  7d   ███░░░░░░░░░░░░░  21.0%      1.7B tokens / 4940 reqs
codex        5h   ██░░░░░░░░░░░░░░  14.0%  resets 11:38 (0h 53m)
codex        7d   █░░░░░░░░░░░░░░░   7.0%
```

## No login. Ever.

limitbar never asks you to authenticate and never talks to a server on its
own. If you already use Claude Code or Codex CLI, those tools have written
everything limitbar needs onto your own disk — transcripts and session
logs. limitbar just reads them. Tools that aren't installed are silently
skipped.

## Running it

There is no setup. Install the binary, run it:

```sh
limitbar              # print current windows once and exit
limitbar -w 10        # live-refresh in the terminal every 10s
limitbar --overlay    # floating always-on-top click-through bar
```

On Linux the deb also installs a launcher ("limitbar" in your app menu),
which starts the overlay with no terminal attached. From a shell, use
`limitbar --overlay &`.

### The overlay

Click-through by default — your clicks land on whatever is underneath.
**Hold Ctrl+Alt while hovering it** to unlock: the bar grows an accent
border and becomes draggable; release Ctrl+Alt and it locks back to
click-through. The position is remembered across restarts.

Appearance lives in `~/.config/limitbar/config.toml`:

```toml
[overlay]
background = "#0d1117"   # RRGGBB or RRGGBBAA
opacity = 0.85           # multiplied onto the background alpha
text = "#e6edf3"
font_size = 12.0
```

(Global key polling needs X11 on Linux; on Wayland the unlock gesture is
currently unavailable.)

## Install

### Debian/Ubuntu (apt)

```sh
curl -fsSL https://apt.clapbox.net/rocky-oss.gpg \
  | sudo tee /usr/share/keyrings/rocky-oss.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/rocky-oss.gpg] https://apt.clapbox.net stable main" \
  | sudo tee /etc/apt/sources.list.d/rocky-oss.list
sudo apt update && sudo apt install limitbar
```

### macOS / Linux (Homebrew)

```sh
brew tap i-rocky/tap
brew install limitbar
```

### Windows (Scoop)

```powershell
scoop bucket add rocky https://github.com/i-rocky/scoop-bucket
scoop install limitbar
```

### From source

```sh
cargo install --git https://github.com/i-rocky/limitbar --features overlay
```

The apt repository itself lives at [i-rocky/apt](https://github.com/i-rocky/apt)
(docs: [i-rocky.github.io/apt](https://i-rocky.github.io/apt/)).

Prebuilt release binaries (overlay included) are also on
[GitHub Releases](https://github.com/i-rocky/limitbar/releases).

## Providers

| Provider | Status | Data source | Accuracy |
|---|---|---|---|
| `claude-code` | ✅ | `~/.claude/projects/**/*.jsonl` | exact token counts, deduped per billed response; window % needs a budget in config (Anthropic doesn't publish the caps) |
| `codex` | ✅ | `~/.codex/sessions/**/*.jsonl` | **official** used-percentages logged by Codex itself — no estimation |
| `cursor` | planned | Cursor's stored session + their usage API | needs a network call with the token Cursor already saved; same zero-prompt rule |
| `antigravity` | planned | local app data | format not yet mapped |

Adding a provider is one file implementing the `Provider` trait in
`src/providers/` — read what the app already wrote, return windows.

## Configuration (optional)

`~/.config/limitbar/config.toml`:

```toml
[budgets.claude-code]
five_hour_tokens = 500000000
weekly_tokens = 3000000000
```

Claude doesn't publish its token budgets, so percentages for `claude-code`
appear once you set budgets calibrated to your plan. Without them you still
get raw totals and reset times. Codex needs no budget — its percentages are
official.

## Development

```sh
cargo test
cargo clippy --all-targets --features overlay -- -D warnings
```

## License

MIT
