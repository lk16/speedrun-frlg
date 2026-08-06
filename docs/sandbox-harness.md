# Sandbox & harness plan

Notes on what the sandboxed build environment needs, and on the
container-lifecycle design. Written before the sandbox exists; revise as it
gets built.

## 1. What to add to the sandbox

### Verification loop (the actual hard requirement)

- **An emulator we can drive headlessly.** mGBA is the right pick: C, builds in
  Docker, has a Lua scripting API and a `libmgba` C API we can link against.
  Vendor the *source* at a pinned commit plus all build deps (cmake, libpng,
  zlib, SDL2 optional) — we need a build that can run with no video/audio,
  advance N frames, feed a button mask per frame, dump RAM regions and
  screenshots. BizHawk is the community TAS standard for GBA but is C#/.NET and
  miserable in a headless container; decide up front which one the *final*
  artifact targets, because it determines the movie format.
- **Movie format decision + a writer/reader for it, pinned before milestone 1.**
  Whatever we choose (`.bk2`, `.vbm`, mGBA's own), the sandbox needs a spec doc
  and a round-trip test (write file → replay in emulator → same final RAM). Do
  not let this get discovered at the end; a route that's 4 hours of frames in
  the wrong format is dead work.
- **A ROM.** Best move: build it from `decompiled/` — add `agbcc` +
  `arm-none-eabi-binutils` + the pret build deps to the image, and the sandbox
  can produce a SHA1-matching ROM itself. Sidesteps the "you must supply a ROM"
  problem, and gets the next item for free.
- **The symbol map from that build** (`.map` / `.sym`). Single highest-leverage
  artifact: gives addresses for `gRngValue`, `gSaveBlock1/2`, `gBattleMons`,
  `gMain.vblankCounter`, map/warp state, etc. Without it, "why did my sim
  diverge" is unanswerable.
- **A differential harness, as a first-class deliverable, not a debugging
  afterthought.** Run Rust sim and emulator over the same input log, compare a
  defined RAM signature every frame, report *first divergence frame + diff*.
  Milestone 1 should be "harness reports zero divergence through the rival
  battle", not "we beat the rival".

### Toolchain realities under no-network

- **Vendored Rust deps.** `cargo` can't reach crates.io. Either `cargo vendor` a
  pre-picked dep set into the image with `.cargo/config.toml` offline mirroring,
  or ship a local registry mirror. Same for Python (`pip download` into a
  wheelhouse + `--no-index`). Otherwise turns get burned discovering that
  `cargo add rand` fails.
- **Pinned rustc + a warm target dir**, or every fresh sandbox pays a full cold
  rebuild.
- Real CPU allocation — search over frame-level input space is the compute sink,
  and it should be Rust, single-binary, resumable.

### State that must survive across sessions

- **A persistent artifacts volume separate from git.** Search trees, savestates,
  trace dumps, candidate input logs are large and churn — they must not be in
  the repo, but must outlive the container. Mount `/artifacts` and treat it as
  append-mostly with a manifest committed to git.
- **Emulator savestates at route checkpoints**, so segment N can be verified
  without replaying 400k frames. Final acceptance is still full power-on replay.
- **A machine-readable route ledger** (`route/best.json`: segment → input log
  hash → frame cost → verified-against-emulator bool). This is the objective
  function *and* the memory. Successive sessions improve entries; anything
  unverified is explicitly marked.
- **A journal/handoff doc committed every session** — what was tried, what
  failed, what's next. With context resets this is the only continuity there is.

### Rules/spec material

- `tas-rules.md` covers category rules. Add a **verification contract** doc:
  exact ROM SHA1s, emulator version + settings (BIOS vs HLE BIOS, RTC, save
  type), start condition (power-on with cleared save? existing save?), and stop
  condition. Emulator settings drift silently breaks replays.

### One thing that can't be sandboxed away

The model already has FRLG route knowledge from pretraining. We can forbid web
lookups but not recall. Decide explicitly whether "don't consult" means "don't
state route facts without a decomp citation" — that version is at least
enforceable (every routing claim in docs must cite a `decompiled/` path), and it
should be written into CLAUDE.md that way.

## 2. On the reset-every-X-minutes design

The instinct is right, the mechanism conflates three separate knobs.

- **Sandbox lifetime ≠ session lifetime ≠ work lifetime.** Context rot is fixed
  by starting a new *session*; killing the container isn't needed for that.
  Killing it additionally kills long-running compute, which is the thing we
  least want to lose.
- **Timer-based kill will land mid-run.** A 2-hour emulator verification or a
  long search doesn't fit a fixed window, and the agent will learn to avoid
  starting valuable long jobs — exactly the wrong incentive. Prefer: kill on
  *idle / turn budget exhausted*, or let the agent signal a checkpoint.
- **Don't lose uncommitted work — quarantine it.** Auto-commit everything to a
  `wip/<session-id>` branch on a timer (a hook, not agent discipline). Same
  "clean state from last good commit" property, but nothing is unrecoverable and
  mid-run state is inspectable. Discarding is cheap; reconstructing isn't.
- **Push works fine without network.** Mount a bare repo at `/origin` and let the
  sandbox `git push` over the filesystem. No network, no host-side pull loop, and
  a proper merge point with branch semantics we control.
- **Move long compute out of the ephemeral container.** A second, long-lived
  worker container sharing `/artifacts`, taking jobs from a queue directory, with
  resumable checkpoints. The agent container stays disposable; searches survive
  its death.
- **Make resets cheap to recover from.** The real cost isn't lost files, it's
  re-deriving orientation each cold start. Invest in the ledger + journal + a
  `make verify` one-liner; that's what makes a 20-minute sandbox productive
  instead of a 20-minute re-read of the decomp.

## First thing to build

The differential harness plus the symbol map. Everything else is scheduling;
that's the part that decides whether the project converges at all.
