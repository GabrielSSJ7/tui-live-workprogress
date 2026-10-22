# questlog

Live, D&D-flavored terminal view of your work: every **project** is a realm on
an overworld map, every **demand** (card / issue / US) is a quest, every
**stage** is a room in that quest's dungeon. LLM harnesses (Claude Code, Codex,
anything that can run a command or write a file) report progress; the TUI
watches the files and redraws in under a second.

The picture *is* the state, so one glance is enough:

| Art | Color | Meaning |
|---|---|---|
| castle with banners | green | done |
| knight fighting a goblin (animated) | gold | active |
| dragon breathing fire | red | blocked |
| locked mountain in fog | grey | not started |

## Views

```
OVERWORLD (macro)        ⏎        REALM (project)          ⏎        DUNGEON (demand)
all realms as tiles   ───────▶  quest pipeline, battle,  ───────▶  stages as rooms, stage
quest board + log       esc     party sheet, combat log    esc     list, blockers, quest log
```

- **Overworld**: tiles on a serpentine road (gold once the realm before is done), a quest
  board with one bar and `✓ ⚔ ⊘ ◇` counters per realm, the global activity log. A
  `⊘N` on an active realm means N quests inside it are blocked.
- **Realm**: `[DONE] / [ACTIVE] / [BLOCKED] / [TODO]` pipeline with bars and "stage 2/4 · Build
  · 6m ago" (or the blocker in red); the selected quest's battle; the party sheet with the
  hero's current focus, progress, stage strip, blockers, who is waiting.
- **Dungeon**: rooms `✓ CLEARED → ⚔ 45% → ◇ DARK`, `▼ HERO HERE` over the current stage,
  notes per stage, the quest's own log.

Header everywhere: breadcrumb, XP/level, active realms, blocked count, time since the last
update (`↻ 12s`), clock, and `⚠ N broken file(s)` if any file fails to parse.

| Key | Action |
|---|---|
| `←↑↓→` / `hjkl` | move (in a realm, `←→` jumps to the neighbouring realm) |
| `Enter` / `Space` | zoom in |
| `Esc` / `Backspace` | zoom out |
| `r` | force reload |
| `q` / `Ctrl-C` | quit |

## Install

```sh
./install.sh          # builds and installs ~/.local/bin/questlog
questlog demo         # sample realms, if the data dir is empty
questlog
```

`install.sh` also installs `questlog-toggle`, a Hyprland scratchpad toggle bound to
**SUPER+ALT+Q** (snippet printed by the installer): first press opens questlog in kitty,
later presses hide/show it without killing it, so it keeps watching live.

Try the demo without touching real data: `QUESTLOG_DIR=/tmp/questlog-demo questlog demo`.

## Reporting progress (for LLMs)

The full instructions live in [`docs/QUESTLOG-CLAUDE.md`](docs/QUESTLOG-CLAUDE.md)
(`questlog guide` prints them with your real data dir). For Claude Code, symlink it to
`~/.claude/QUESTLOG.md`, import it with `@QUESTLOG.md` at the top of `~/.claude/CLAUDE.md`,
and add a `SessionStart` hook running `questlog brief`, which injects the open quests into
every new session so ids get reused instead of reinvented. The short version:

```sh
questlog quest   api US-1234 "JWT auth" Design Implement Tests Review
questlog stage   api US-1234 Implement active 40
questlog focus   api US-1234 "Wiring the refresh-token middleware"
questlog block   api US-1234 "PO must confirm token TTL" "PO"
questlog unblock api US-1234
questlog done    api US-1234
questlog log     api US-1234 file "Created auth/token.go"
questlog realm   api "Backend Citadel" 1
```

Every command writes atomically (temp file + rename) and appends to the activity log.

## Data layout

```
~/.local/tui-live-workprogress/        ($QUESTLOG_DIR overrides)
├── log.jsonl                          one JSON event per line
└── <project>/
    ├── project.json                   optional {"name": "...", "order": 1}
    └── <demand>.json                  {"title", "owner", "focus", "stages": [...],
                                        "blockers": [{"what","who"}], "waiting": [...]}
```

Status and progress are derived from stages unless set explicitly; status words accept
aliases (`wip`, `busy`, `doing`, `completed`, ...). A broken file never takes the map
down: it shows up as `⚠` in the header and in the activity log.

## Development

```sh
cargo test                                         # all tests
cargo run --example snapshot -- /tmp/shots 160 48  # render each view to HTML
```
