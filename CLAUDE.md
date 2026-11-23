# speedrun-frlg

TASes of Pokémon FireRed and LeafGreen, routed inside a closed sandbox. The two versions are
typically one speedrun category, so a route is free to pick whichever version is faster —
measured, not assumed; `docs/rival-1/route.md` tracks which version each number belongs to. **Read
`docs/sandbox.md` before doing anything** — it is also the sandbox's own prompt file, and almost
nothing here is where a normal machine would put it. `README.md` says what the project is *for*.

On branch `human-run`, also read `human-run/PROMPT.md`: that branch simulates a human Round 2
run rather than building a TAS, and the prompt overrides the TAS workflow where they differ.

Start every sandbox with `bin/frlg-doctor`. It is the fastest way to find out that a mount is
missing, the image is the stock one, or the deps tree is stale.

## The one rule

The network is closed on purpose. Every routing claim you write down **cites a path under
`decompiled/`**. A claim you cannot cite is a guess and must be labelled one. You have FireRed
and LeafGreen knowledge from pretraining; the citation rule is what makes that harmless.

## Two verification tiers

- **Tier 1** — libmgba, in the sandbox, every iteration. `frlg route verify`.
- **Tier 2** — BizHawk replaying a `.bk2`, on the host, for acceptance. Queue a request in
  `$FRLG_ARTIFACTS/verify/queue/`; answers land in `.../results/`. Never block on it.
  `docs/rival-1/route.md` says what tier 2 can and cannot currently do.

Nothing is "accepted" on tier 1 alone, and the ledger (`route/rival-1/ledger.json`) records which tier
each segment has actually passed. Do not widen a claim past its evidence.

## What the generic advice gets wrong here

A `CLAUDE.md` further up the tree describes a normal sbx sandbox. Where it disagrees with this
file or with `docs/sandbox.md`, those win:

- **No installing anything.** `apt`, `pip install` from an index, `cargo add`, `npm`, `uv`,
  `git clone`, any download — all dead by design, not by accident. There is no sudo route around
  it. A new crate goes in `tools/vendor-manifest/Cargo.toml` on the *host*, followed by
  `tools/host-prep.sh` and a fresh sandbox. If you need one, say which and why, and carry on
  without it.
- **No network policy to widen.** `sbx policy allow network`, published ports,
  `host.docker.internal`, Docker networks: none apply. `api.anthropic.com` is the entire
  allowlist and the closed network *is* the project. If something is genuinely impossible
  without a host, stop and say which host and why — do not route around it.
- **No push, no PR.** github.com answers 403 here, so `gh` and `git push` do not work. Commit
  locally and often; only committed work survives the sandbox.
- **Python is standard library only.** No wheelhouse, no venv with pip. `struct`, `zipfile`,
  `hashlib`, `json`, `zlib` and `ctypes` cover what this project needs.
- **Do not run BizHawk here.** It needs Mono, OpenAL and Lua 5.4, none of which are installed.
  `$BIZHAWK_HOME` is mounted read-only as reference material only.

`cargo build`, `cargo test`, `cargo fmt` and `cargo clippy` all work and are expected before a
commit.

## Layout

| Path | What it is |
| --- | --- |
| `docs/sandbox.md` | the environment: mounts, image, network, disk, the two tiers |
| `docs/harness.md` | the tier-1 emulator harness and the `.ilog` format |
| `docs/tricks/` | cross-target tricks and externally sourced notes, decomp-checked |
| `docs/<target>/route.md` | a target's route, its evidence, and what is not optimised |
| `docs/<target>/journal/` | that target's lab notebook, one dated file per session entry |
| `crates/` | `mgba-sys`, `frlg-emu`, `frlg-rng`, `frlg-battle`, `frlg-route`, `frlg-cli` |
| `route/<target>/` | that target's committed input logs and `ledger.json` |
| `route/template.bk2` | the BizHawk movie template (target-independent) |
| `bin/`, `tools/` | sandbox-side helpers and host-side preparation |

`tools/` is host-only — those scripts need the network, mono, or docker, and none of the three
exists in the sandbox.

## Unattended runs: no deletions

Nobody is there to approve prompts. `.claude/settings.json` denies `rm`, `rmdir`, `find -delete`
and `git clean` outright, so they fail instead of waiting. Never delete: write each temp file under
a unique name in the scratchpad (stale ones are harmless; `scratch/` is wiped per sandbox anyway),
overwrite with `>` or the Write tool, and leave old generated files in place.

## Open-ended tasks: the watchdog cron

Any task that runs "forever" or until the human stops it (route and strategy searches such as
`human-run/`) must start a **10-minute watchdog** in the first turn, and keep it until the human
says to wrap up. Create it with `CronCreate` (every 10 minutes, recurring; it is session-only, so
recreate it after a restart or `/resume`; `CronList` shows whether it is alive; cancel it with
`CronDelete` only at wrap-up). Do not rely on agents waking themselves: they stall in "waiting on
background work" and background poll loops never wake them.

Cron prompt (use verbatim, adjust the target if it is not `human-run`):

> Progress and hygiene check (every 10 min). 1) Run `bin/frlg-watchdog` (live compute by worktree,
> stale poller shells, per-worktree scores.jsonl mtime and last commit), `git log --since='60 minutes ago'
> --oneline`, and `ListAgents`. 2) Dead work: an agent with no live process (frlg-human, descend.py,
> try.py, cargo) and no new scores.jsonl rows or commits for 20+ minutes is dead or stalled: SendMessage
> it to check its log, commit and report, or merge its committed branch and launch a replacement (TaskStop
> it first if it ignores two messages). 3) Cleanup: kill only orphaned or stuck processes and stale poller
> shells by exact PID (`kill <pid>`); never `pkill -f`, never kill claude/tini/sleep-infinity or a live
> agent's compute job or descent. TaskStop stale monitors of completed agents. 4) If no agent is running,
> or CPU is idle, start the next improvement round from `human-run/NEXT.md` in worktree-isolated agents,
> and merge finished branches into the target branch (merge first, resolve `scores.jsonl` conflicts by
> keeping both sides, run fmt/build/tests, commit). Never use rm. Report in 3 lines: progress yes/no,
> what was killed, what was started.

Rules the watchdog and the agents follow:

- **Cheap checks.** Run `bin/frlg-watchdog` first; it is read-only. Do not sleep-poll in the
  main turn (`sleep` chains are blocked); wait for notifications.
- **Poller shells.** Agents leave `until`/`while ... pgrep -f PATTERN` loops behind; when the pattern
  appears in the loop's own command line it matches itself and never exits. Kill those by exact PID
  and tell agents not to write them. Prefer polling a result file with a hard timeout.
- **Agents.** One topic per agent in its own worktree; tell each to `git merge <target branch>` first
  (worktrees can be cut from a stale commit), use a unique scratchpad file prefix (shared scratchpad
  files collided once), commit as it goes, never `pkill -f`, and not end its turn while a job it needs is
  running. Agents that finish with background work still running are not dead: check for a live process
  before resuming them.
- **Keep the handoff current.** Maintain a `NEXT.md` (state, model changes, ranked upcoming searches)
  in the target directory (`human-run/NEXT.md`); commit it at every merge milestone and at wrap-up, so a
  fresh session can start from it.
- **Wrap-up.** When the human asks to wrap up: let in-flight confirmations finish, start no new
  candidates, stop descents by exact PID, merge and verify (build, fmt, tests), update `NEXT.md`, commit,
  then `CronDelete` the watchdog.
- **Scores archive.** `human-run/scores.jsonl` must stay well under GitHub's 50 MB warning (100 MB is a
  hard push refusal). Once it passes ~30 MB, run `human-run/tools/scores.py archive` at wrap-up, after every
  agent branch is merged, and commit. A branch cut before an archive conflicts on `scores.jsonl`: keep only the
  rows that branch added, not its copy of the archived rows. Readers use `scores.lines()`.
