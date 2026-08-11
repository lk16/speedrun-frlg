# speedrun-frlg

TASes of Pokémon FireRed and LeafGreen, routed inside a closed sandbox. The two versions are
typically one speedrun category, so a route is free to pick whichever version is faster —
measured, not assumed; `docs/route.md` tracks which version each number belongs to. **Read
`docs/sandbox.md` before doing anything** — it is also the sandbox's own prompt file, and almost
nothing here is where a normal machine would put it. `README.md` says what the project is *for*.

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
  `docs/route.md` says what tier 2 can and cannot currently do.

Nothing is "accepted" on tier 1 alone, and the ledger (`route/ledger.json`) records which tier
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
| `docs/route.md` | the route itself, its evidence, and what is not optimised |
| `docs/journal.md` | what was tried, what failed, what is next |
| `crates/` | `mgba-sys`, `frlg-emu`, `frlg-route`, `frlg-cli` |
| `route/` | committed input logs, `ledger.json`, `template.bk2` |
| `bin/`, `tools/` | sandbox-side helpers and host-side preparation |

`tools/` is host-only — those scripts need the network, mono, or docker, and none of the three
exists in the sandbox.
