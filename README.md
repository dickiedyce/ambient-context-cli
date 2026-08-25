# Ambient Context

A macOS CLI daemon that keeps a written record of what you work on, for your
own LLM to read.

While running, Ambient Context reads the text of whichever window you have
focused (via the macOS accessibility tree, every few seconds) and appends it
to a plain markdown file: one file per day, in a folder you choose. Point
Claude Code or any other agent at that folder and it can answer "what did I
work on Tuesday?", build memory about your projects, or write your standup
for you.

- **No screenshots, no video.** Reads text through the accessibility API,
  nothing else.
- **Nothing leaves your machine.** No account, no server, no telemetry, no
  bundled model. No network calls.
- **Files you own.** Plain markdown in a folder you chose. Move them, grep
  them, delete them.
- **Redaction before writing.** Password managers and private browsing
  windows are never captured. Password fields are skipped at the source,
  and credentials, API keys and card-shaped numbers are scrubbed before
  anything touches disk.
- **Built to be read by an LLM.** Lines are deduplicated across the day,
  interface junk is filtered out, and each block records the document path
  or URL it was looking at so your agent can open the real thing instead
  of trusting fragments. The folder carries an `AGENTS.md` explaining the
  format to whatever reads it.

Requires macOS 14+ on Apple Silicon.

## Install

You need [Rust](https://rustup.rs) and Xcode Command Line Tools.

```bash
git clone https://github.com/dickiedyce/ambient-context-cli
cd ambient-context-cli/cli
cargo build --release
cp target/release/ambient-context /usr/local/bin/
```

Build and install the accessibility helper:

```bash
cd src-ax
swift build -c release
cp .build/release/ambient-context-ax /usr/local/bin/
```

## Usage

```bash
ambient-context start           # start capturing (foreground)
ambient-context start & disown  # start and detach

ambient-context stop            # stop
ambient-context status          # check if running

ambient-context today           # print path to today's file
ambient-context snapshot        # one-off accessibility snapshot

ambient-context logs            # recent logs
ambient-context logs -f         # follow logs
```

## First run

1. Grant Accessibility permission when prompted — this is the permission
   that lets the tool read window text, and nothing works without it.
2. The default capture folder is `~/Ambient Context`, outside `~/Documents`
   so iCloud does not sync your records off the machine. Override with
   `--folder` or in `~/.config/ambient-context/config.toml`.

## What a day file looks like

```markdown
---
date: 2026-08-25
captured_by: Ambient Context 0.2.0
---

## 09:41–10:05 · Chrome · API design notes

url: https://example.com/docs/api

<text seen in that window, first time it appeared today>
```

Block headings are the day's timeline. Body lines are written once per day
no matter how often they are seen, so the file stays small enough to hand
to an LLM whole. `AGENTS.md` in the capture folder documents the format
and how to read it well.

## App compatibility

- Chromium and Electron apps (Chrome, Slack, VS Code, Obsidian, Figma...)
  only build their accessibility tree when asked, so the first seconds of
  capture in those apps are thin and fill in on later passes.
- GPU-rendered terminals (Kitty, Alacritty) expose little or no text.
  Terminal.app and iTerm2 work.

## Tests

```bash
cd cli && cargo test
```

## Privacy

The tool reads only the focused window: never background windows, other
displays or minimised windows, and never while the screen is locked. It
excludes password managers and private browsing entirely, skips secure
input fields at the accessibility level, and pattern-scrubs secrets before
writing. Everything it produces is plaintext on your own disk, and the
capture folder is excluded from capture so it cannot observe itself. If
you find a hole in any of this, please open an issue.
