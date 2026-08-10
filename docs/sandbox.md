# Sandbox environment

What is true of this repository inside a box sandbox. Read it before running anything; almost
nothing here is where a normal machine would put it.

Run `bin/frlg-doctor` first, in every new sandbox. It checks the three mounts, executes each
prebuilt tool, and confirms the network really is closed. The prebuilt tools were compiled on the
host against a different glibc than this image ships, so a mismatch is possible; doctor turns it
into one clear line instead of a confusing failure three steps later.

## Why the network is closed

The allowlist holds `api.anthropic.com` and nothing else. That is the point of the project, not a
limitation to route around: the run has to be derived from `decompiled/`, not recalled or looked
up. You have FRLG knowledge from pretraining and no sandbox can take it away, so the enforceable
form of the rule is **every routing claim you write down cites a path in `decompiled/`**. A claim
you cannot cite is a guess, and should be labelled as one.

Category rules and file-format facts are a different kind of knowledge and are allowed, but they
have to come from what is already committed here or mounted read-only — not from memory. If a rule
you need is not written down anywhere, write down what you assumed and why, so the next session can
check it.

## Where everything is

Mounts keep the path they have on the host, and those differ per machine, so never hardcode one.
The startup commands resolve all three and write `/etc/profile.d/10-frlg.sh`, which your shell
sources:

| Variable | What it points at |
| --- | --- |
| `FRLG_DECOMP_RO` | the pret/pokefirered checkout, read-only |
| `FRLG_DECOMP` | `~/decomp`, your writable copy of it, agbcc already in `tools/` |
| `FRLG_DEPS` | the prebuilt toolchain tree, read-only |
| `FRLG_ARTIFACTS` | the one writable mount, and the only thing that outlives this sandbox |
| `MGBA_PREFIX` | libmgba 0.10.5 — `lib/libmgba.so`, headers under `include/mgba` |
| `BIZHAWK_HOME` | BizHawk 2.11.1, read-only, as reference material (see below) |

`$FRLG_DEPS/MANIFEST` records what each of those was built from.

Writable: `$FRLG_ARTIFACTS`, your home, and the git clone. Nothing else. `$FRLG_DECOMP_RO` is the
host's own checkout — treat a write attempt there as a bug in what you are doing.

## Building the ROM

There is no ROM anywhere. Build it:

    cd ~/decomp
    make -j"$(nproc)" COMPARE=1
    make syms

`COMPARE=1` checks the result against `firered.sha1`, so a successful build is a ROM verified
byte-for-byte against the retail cartridge. LeafGreen is `GAME_VERSION=LEAFGREEN`. The build writes
`pokefirered.gba`, `pokefirered.map` (the link step) and `pokefirered.sym` (`make syms`).

Copy all three into `$FRLG_ARTIFACTS/rom/`. They are the one build output worth keeping, because
the build tree itself does not survive this sandbox.

`pokefirered.sym` is the highest-leverage artifact in the project. It gives you addresses for
`gRngValue`, `gSaveBlock1`/`gSaveBlock2`, `gBattleMons`, `gMain` and the rest, which is what turns
"the simulation diverged somewhere" into "the simulation diverged at frame N in this variable".
Without it, a divergence is unanswerable.

## Verification has two tiers, and only one of them is here

**Tier 1, in this sandbox, on every iteration.** libmgba at `$MGBA_PREFIX`, driven headless by our
own harness: load ROM, feed a key mask per frame, advance, read RAM, dump a PNG. Write the FFI by
hand — there is no clang here, so bindgen is not an option. This is the loop you optimise against.

**Tier 2, on the host, for acceptance.** BizHawk replays the `.bk2`. **Do not try to run BizHawk
here.** It needs Mono, OpenAL and Lua 5.4, none of which are installed, and relocating them into a
network-less container is not worth it for something off the inner loop.

`$BIZHAWK_HOME` is mounted anyway, read-only, because it is the *authority on the output format*.
Its `defctrl.json` and shipped config files give the real GBA button names and the SyncSettings
schema. Read them instead of guessing at the format.

To request a tier-2 run, drop the `.bk2` in `$FRLG_ARTIFACTS/verify/queue/`; results come back in
`$FRLG_ARTIFACTS/verify/results/`. A human on the host drives that, so it is not instant and may
not happen during your session. Never block on it — queue it, keep working, and check for results
next time you look.

## The output format

`.bk2`, because that is what gets watched. Two things reliably break a movie:

**SyncSettings.** If `route/template.bk2` exists, it is a real one-frame movie recorded in the
BizHawk build that will replay your work. Copy its `Header` and `SyncSettings` verbatim into
everything you emit. Inventing them is the single most likely way to produce a file that loads and
then desyncs.

**Column order.** The `Input Log` column order is not the game's key bit order.
`decompiled/include/gba/io_reg.h` defines `A_BUTTON 0x0001`, `B_BUTTON 0x0002`,
`SELECT_BUTTON 0x0004`, `START_BUTTON 0x0008`, … `R_BUTTON 0x0100`, `L_BUTTON 0x0200`,
`KEYS_MASK 0x03FF` — that is what the *game* reads, and what tier 1 should feed libmgba. The `.bk2`
ordering is BizHawk's, so confirm it against the files in `$BIZHAWK_HOME` and convert explicitly.
Keep the raw per-frame `u16` log as the canonical artifact and treat `.bk2` as an export of it;
that way a format mistake costs a re-export, not a re-route.

A `.bk2` is only real once the raw log it came from has passed tier 1 **and** the `.bk2` decodes
back to that same log. Write that round-trip check early.

## Rust

cargo works offline against a vendored crate tree; `~/.cargo/config.toml` is generated at startup
to point at it. `cargo add` cannot work here, and no amount of cleverness will make it. A new
dependency has to go into `tools/vendor-manifest/Cargo.toml` on the host, with `tools/host-prep.sh`
re-run and a fresh sandbox started. If you need one, say which crate and why, and carry on without
it — do not vendor a crate by hand into the repo.

`target/` lives on this sandbox's own disk and dies with it. sccache in `$FRLG_ARTIFACTS/cache`
absorbs most of the rebuild, which is why it is the one build cache allowed on the artifacts
volume.

## Python

Standard library only. `$FRLG_DEPS/wheels` is the offline wheelhouse and is currently empty; pip is
configured with `--no-index`, so an install of anything else fails by design. Adding a wheel is the
same host-side round trip as adding a crate.

## Disk

`$FRLG_ARTIFACTS` is a real directory on the host's disk and it is the one thing here that can eat
it. `bin/frlg-artifacts-gc` enforces a per-directory budget — sccache 4G, runs 4G, states 3G,
scratch 2G, rom 1G — and refuses past a hard 20 GiB ceiling. Run it after anything that writes a
lot, and again before you finish.

Savestates at route checkpoints are cheap and worth keeping; traces should be compressed and
rotated; `scratch/` is wiped at the start of every sandbox, so nothing you want to keep goes there.
Never dump video — that is what blows the budget, and a PNG at the divergence frame answers the
same question.

## What survives, and what to write down

Only committed work comes back from a sandbox. `$FRLG_ARTIFACTS` survives too, but it is not git
and nobody reviews it.

Context resets and sandboxes end mid-thought, so continuity is something you have to write, not
something you have. Two files earn their keep: a machine-readable route ledger (segment → input-log
hash → frame cost → whether it has passed tier 1 and tier 2) which is both the objective function
and the memory, and a short session journal saying what was tried, what failed, and what is next.
Anything unverified must be marked unverified — an entry that overstates its evidence is worse than
no entry.

Commit as you go. A verified segment that only exists in an uncommitted file is a segment you will
route again.

## Things that will not work

Do not spend turns discovering these: `apt`, `pip install` from an index, `cargo add`, `git clone`,
any download; running BizHawk; writing to `$FRLG_DECOMP_RO`; `pre-commit`, whose hook repos come
from GitHub. If one of them is genuinely the only way forward, stop and say which host you would
need allowed and why.
