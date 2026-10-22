# 🗺️ QUESTLOG — mandatory progress reporting

> **Priority: HIGH. Applies to every session, every project, every harness.**
> The user watches a live TUI (`questlog`) that shows every project and demand
> as a D&D map. It is only as true as what you report. Silent work is invisible
> work: if you did it and did not report it, for the user it did not happen.

Data dir: `~/.local/tui-live-workprogress` (override with `$QUESTLOG_DIR`). Run `questlog brief` to see
what is open right now.

---

## 1. The contract (read this even if you skip the rest)

1. **Session start** → run `questlog brief`. Find the quest you are about to work on.
   If it exists, reuse its project slug and demand id **exactly**. If not, create it.
2. **Before writing code** for a task → the demand exists, with all known stages.
3. **Exactly one stage `active`** per demand while you work on it, with a percent.
4. **Every meaningful step** → update the stage (`stage ... active <pct>`), the focus,
   and log notable events (files created, tests run, decisions made).
5. **Blocked on a person / another demand** → `questlog block` **with who unblocks it**.
   Unblocked → `questlog unblock`.
6. **Stage finished** → `stage ... done`. **Demand finished** → `questlog done`.
7. **Session end / handing back to the user** → the TUI must match reality:
   correct active stage, correct percent, correct focus, blockers listed.

Reporting is cheap (one command, milliseconds). Never batch it up "for later".
Never ask the user for permission to report. Never skip it because the task is small:
a 5-minute fix is still a quest with one or two stages.

---

## 2. Vocabulary

| Term | Meaning | Example |
|---|---|---|
| **project** (realm) | A product / repository / long-lived area of work. | `api`, `frontend`, `tui-live-workprogress` |
| **demand** (quest) | One deliverable: card, issue, user story, bug, spike. | `US-1234`, `BUG-88`, `fix-login-timeout` |
| **stage** (room) | One step inside the demand, in order. | `Design`, `Implement`, `Tests`, `Review` |
| **status** | `todo` · `active` · `blocked` · `done` | aliases: `wip`, `busy`, `doing`, `completed` |
| **focus** | One short line: what is happening *right now*. | `"Wiring the refresh-token middleware"` |
| **blocker** | What stops the demand + who removes it. | `"PO must confirm token TTL"` → `PO` |
| **waiting** | Who is waiting on this demand to finish. | `frontend/W-7` |

### Naming rules (consistency is what makes the map readable)

- **Project slug**: the git repository directory name, lowercase
  (`~/code/delivery-api` → `delivery-api`). If the user names a realm explicitly, use theirs.
  Outside a repo, use the client / product name the user uses.
- **Demand id**: the tracker id when one exists (`US-1234`, `JIRA-55`, `#412` → `GH-412`).
  Otherwise a short kebab slug of the goal: `fix-login-timeout`, `add-csv-export`.
  If the project follows the demand-folder convention (`docs/demandas/<id>/`), use that
  same `<id>`.
- Allowed characters in ids: letters, digits, `-`, `_`, `.`. No spaces, no `/`.
- **Stage names**: short, Capitalized, verb or noun. Keep them stable: renaming a stage
  creates a new one.
- **Never invent a second id for the same work.** Check `questlog brief` first.

---

## 3. Commands (preferred interface: atomic writes, logs itself)

```sh
questlog brief                                        # open quests, compact (start here)
questlog quest   <project> <demand> "<title>" [Stage ...]   # create / retitle, add stages
questlog stage   <project> <demand> <Stage> <status> [percent]
questlog focus   <project> <demand> "<what is happening now>"
questlog block   <project> <demand> "<what blocks>" ["<who unblocks>"]
questlog unblock <project> <demand>
questlog done    <project> <demand>                   # all stages done, blockers cleared
questlog log     <project> <demand|-> <kind> "<message>"
questlog realm   <project> "<Display Name>" [order]    # pretty name / map order
questlog where                                        # print the data dir
```

- `quest` is idempotent: running it again keeps existing stages and appends new ones.
- `stage` creates the stage if it does not exist (appended at the end).
- `percent` is 0–100 and means *progress inside that stage*. `40` and `40%` both work.
- Every mutating command appends to the activity log automatically; use `log` only for
  extra events.
- Quote any argument with spaces.
- A non-zero exit means the input was rejected; the error says what was expected. Fix and retry.

### Log kinds

| kind | When | Example message |
|---|---|---|
| `start` | Began a demand or resumed it after a pause | `Resumed: refresh-token rotation` |
| `progress` | Notable step inside a stage | `Middleware wired, 3 handlers left` |
| `file` | Created / significantly changed a file | `Created auth/token.go` |
| `test` | Ran tests (say the result) | `go test ./auth: 14 passed` |
| `fail` | Something failed and needs attention | `Migration 0042 fails on empty table` |
| `blocked` / `unblocked` | (automatic via block/unblock) | |
| `done` | (automatic via `done` / stage done) | |
| `note` | Decision, finding, context the user should see | `Chose RS256: PO confirmed key rotation` |

Messages: one line, past tense or present progressive, concrete, ≤ 80 chars.
No secrets, tokens, passwords or customer data in any message.

---

## 4. Choosing stages

Create all known stages up front so the percent means something. Adjust later
(add with `stage`, they append). Templates — pick and adapt:

| Kind of work | Stages |
|---|---|
| Feature | `Analyze` `Design` `Implement` `Tests` `Review` |
| Bug fix | `Reproduce` `Diagnose` `Fix` `Regression test` `Verify` |
| Refactor | `Map` `Extract` `Migrate callers` `Tests` `Cleanup` |
| Spike / research | `Questions` `Research` `Prototype` `Write-up` |
| Data migration | `Analyze` `Migrations` `Backfill` `Verify` `Cleanup` |
| Review / PR feedback | `Read` `Address comments` `Re-test` `Reply` |
| Docs | `Outline` `Write` `Review` |
| Infra / deploy | `Plan` `Configure` `Dry run` `Rollout` `Monitor` |

Percent guidance inside the active stage: 10 = started, 25 = approach clear,
50 = half the pieces in place, 75 = works, polishing, 90 = only verification left.
Update it at least at every one of those thresholds you cross.

---

## 5. Full walkthrough

User: *"Implement JWT refresh tokens in the api (card US-1234)."*

```sh
questlog brief
questlog quest api US-1234 "JWT refresh tokens" Analyze Implement Tests Review
questlog stage api US-1234 Analyze active 10
questlog focus api US-1234 "Reading current auth middleware"
# ... reading code ...
questlog stage api US-1234 Analyze done
questlog stage api US-1234 Implement active 25
questlog focus api US-1234 "Adding token rotation to auth/token.go"
questlog log   api US-1234 file "Created auth/refresh.go"
questlog stage api US-1234 Implement active 75
# need a product decision:
questlog block api US-1234 "Refresh TTL undefined (7d or 30d?)" "PO"
questlog focus api US-1234 "Waiting PO on refresh TTL; tests meanwhile"
# user answers: 30d
questlog unblock api US-1234
questlog log   api US-1234 note "PO: refresh TTL = 30d"
questlog stage api US-1234 Implement done
questlog stage api US-1234 Tests active 50
questlog log   api US-1234 test "go test ./auth: 14 passed"
questlog stage api US-1234 Tests done
questlog stage api US-1234 Review active 10
questlog focus api US-1234 "PR !231 opened, waiting review"
# merged:
questlog done  api US-1234
```

---

## 6. Situations

- **Resuming work from a previous session**: `questlog brief`, then `log ... start "Resumed: ..."`,
  set the right stage `active`, refresh the focus.
- **Several demands in one session**: each has its own quest. Only the one you are touching
  right now has an `active` stage that moves; leave the others as they are.
- **User changes scope mid-way**: add or rename stages (new names append; mark obsolete
  ones `done` with a `note` explaining). Retitle with `quest`.
- **Work abandoned / cancelled**: `log ... note "Cancelled: <reason>"` and `done` it, so it
  does not haunt the map as active.
- **You are blocked by another demand** (yours or someone else's): block with the demand
  reference as *what* and its owner as *who*: `block docs DOC-1 "Final schema (api/US-1234)" "Claude"`.
  On the other demand, add the waiter to `waiting` (edit the JSON, see §8).
- **Tests fail / build breaks**: `log ... fail "<what failed>"` immediately; keep the stage
  `active` (not `blocked` — blocked means waiting on someone else).
- **Subagents / parallel agents**: the orchestrator reports. Subagents may `log` their own
  notable events under the same project/demand; never create parallel demands for one task.
- **Pure Q&A, no work product** (explaining code, answering a question): no quest needed.
  Anything that changes files, configs, tickets or produces a deliverable: quest needed.
- **The command is not installed** (`questlog: command not found`): write the files directly
  (§8). Do not silently skip reporting.

---

## 7. Anti-patterns (each one makes the map lie)

- ❌ Reporting only at the end. The TUI is live; the user glances at it while you work.
- ❌ Leaving a stage `active` after moving on. Close it (`done`) or set it back to `todo`.
- ❌ Two `active` stages in one demand.
- ❌ `blocked` without a *who*. A blocker nobody can remove is just a complaint.
- ❌ Percent jumping 10 → 100 in one step on long work.
- ❌ Inventing a new demand id when one already exists for this work.
- ❌ Vague focus (`"working"`, `"coding"`). Say *what*: `"Adding retry to S3 upload"`.
- ❌ Marking `done` before verification (tests, build, user confirmation when asked for).
- ❌ Secrets, tokens, customer data in titles, focus or log messages.

---

## 8. File format (only when the CLI is unavailable)

Layout:

```
~/.local/tui-live-workprogress/log.jsonl                    append one JSON object per line
~/.local/tui-live-workprogress/<project>/project.json       optional {"name": "Backend Citadel", "order": 1}
~/.local/tui-live-workprogress/<project>/<demand>.json      one demand per file
```

Demand file:

```json
{
  "title": "JWT auth",
  "owner": "claude",
  "focus": "Wiring the refresh-token middleware",
  "stages": [
    {"name": "Design", "status": "done"},
    {"name": "Implement", "status": "active", "progress": 40, "note": "middleware left"},
    {"name": "Tests", "status": "todo"}
  ],
  "blockers": [{"what": "PO must confirm token TTL", "who": "PO"}],
  "waiting": ["frontend/W-7"]
}
```

- `id` defaults to the file name. `status` / `progress` at the top level are optional
  overrides; normally they are derived from stages (any blocker → blocked, all stages
  done → done).
- Write the whole file at once: write `.<name>.tmp` in the same dir, then rename over the
  target. Files starting with `.` are ignored by the TUI, so a temp file is never shown.
- Log line (RFC 3339 or `YYYY-MM-DD HH:MM:SS` local):

```
{"ts":"2026-10-22T10:55:12-03:00","project":"api","demand":"US-1234","kind":"file","msg":"Created auth/token.go"}
```

- A file that fails to parse is shown as `⚠ broken file` in the TUI header. If you see one
  in `questlog brief`, fix it.

---

## 9. What the user sees (so you know why it matters)

- **Overworld**: every project as a tile. Castle = done, knight vs goblin = active,
  dragon = blocked, locked mountain = not started. `⊘N` = N blocked quests inside.
- **Realm**: the project's quests as a pipeline with bars, the selected quest's battle,
  and a party sheet whose speech bubble is **your `focus`**.
- **Dungeon**: the quest's stages as rooms; `▼ HERO HERE` sits on the **active stage**.
- Header: `↻ 12s` = time since the last update. If it says hours while you are working,
  you are not reporting.
