# The route: power-on to a beaten rival

12209 frames (~3m24s at 59.7275 Hz) from reset to `gBattleOutcome == B_OUTCOME_WON`, with Squirtle.
Tier 1 only: mGBA agrees. BizHawk has not seen it, and cannot until the host has a GBA BIOS — see
[Tier 2](#tier-2) for what is left before it can.

The count was 11873 until 2026-08-11, when tier 1 was re-pinned from mGBA 0.10.5 to the exact
commit BizHawk bundles (`94b1578f`, see `docs/harness.md`). Segments 01–07 replay identically on
the new core; the battle RNG stream does not, so `08-battle-win` re-searched its start delay and
the chosen battle is now 336 frames longer. That is the pin surfacing a real emulation delta
*before* tier 2 had to find it, which is the pin doing its job.

    frlg route build       # run the segments, write route/logs/*.ilog and route/ledger.json
    frlg route verify      # replay the committed logs from reset and check every claim
    frlg route status      # print the ledger
    frlg route tune        # sweep the route-level knobs, scored on total frames

## How a segment is written

A segment is Rust (`crates/frlg-route/src/segments.rs`), not a list of frame numbers. It drives a
`Recorder`, which appends exactly one key mask per frame it advances, so "mash A until the naming
screen appears" still comes out as a replayable log. It ends at an *observable* -- a RAM condition
with a decomp citation -- and the builder refuses to record a segment that ran to completion without
reaching it.

Walking is not written down at all. `nav::walk_to` searches for the path in the emulator: hold a
direction from a savestate, see where the player lands, Dijkstra over the results with frame cost as
the metric. The collision map, the warp table and the turn-in-place rule are already in the ROM, so
nothing here has to restate them and get one of them wrong.

## The segments

| Segment | Frames | Ends | Observable |
| --- | ---: | ---: | --- |
| `01-boot` | 347 | 347 | `CB2_NewGameScene` -- NEW GAME taken |
| `02-intro-oak` | 1840 | 2187 | `CB2_NamingScreen` -- Oak's speech and the boy/girl choice done |
| `03-names` | 1442 | 3629 | `CB2_Overworld` in the bedroom, both names entered |
| `04-house` | 455 | 4084 | map is Pallet Town (3.0) |
| `05-to-lab` | 1359 | 5443 | map is Oak's lab (4.3) |
| `06-starter` | 2496 | 7939 | `gPlayerPartyCount == 1`, `VAR_STARTER_MON` set, lab scene var 3 |
| `07-battle-start` | 473 | 8412 | `gMain.inBattle` |
| `08-battle-win` | 3797 | 12209 | `gBattleOutcome == B_OUTCOME_WON` |

Map ids are `(group, number)` indices into `decompiled/data/maps/map_groups.json`.

## The three things the route has to get right

**Oak's interruption is the way into the lab.** Walking to Pallet Town `(12,1)` fires
`PalletTown_EventScript_OakTriggerLeft` (`decompiled/data/maps/PalletTown/map.json`, `coord_events`),
which ends in `warp MAP_PALLET_TOWN_PROFESSOR_OAKS_LAB`. The route walks onto that tile and then
mashes A through the scene.

**Two prompts want different answers.** `..._EventScript_ConfirmStarterChoice` asks YES/NO to the
starter, and A takes YES. `EventScript_ChoseStarter` then asks YES/NO to a *nickname*, where YES
costs an entire naming screen. So `06-starter` mashes A only until the mon is in the party -- the
`givemon` happens before the nickname prompt -- and switches to B for the rest, which answers no and
still advances every message
(`decompiled/data/maps/PalletTown_ProfessorOaksLab/scripts.inc`).

**The battle trigger is inert until the rival has his.** The `coord_events` on row `y=8` only fire
`..._EventScript_RivalBattleTrigger*` when the lab scene var is 3, which the rival taking his ball
sets. `06-starter` therefore does not end when the player has a mon; it ends when the scene does.

## What the evidence is

`frlg route verify` starts one emulator at reset and replays the committed `.ilog` files in order.
For each it checks the file's digest against the ledger, then asks the segment's own `reached`
predicate whether the game is where the segment says. It fills in `tier1` from what it saw rather
than copying what the builder claimed. `crates/frlg-route/tests/route.rs` is the same check as a
test, and also compares every segment's RAM fingerprint against the ledger.

The eight logs joined into one file (`frlg log cat`) replay to the same fingerprint as the
segmented run, `884098b71ea9e75bd992894371f510ce4c1f5675`, ending on `gBattleOutcome = 1`,
`gPlayerPartyCount = 1`, Squirtle at level 6.

## What the RNG does in this battle, and what it does not

`Random()` is an LCG over `gRngValue`, returning the top 16 bits
(`decompiled/src/random.c`). Three things in a normal battle consume it:

- **Criticals**: `!(Random() % sCriticalHitChance[critChance])`, base chance 1 in 16
  (`decompiled/src/battle_script_commands.c:1199`, table at `:588`).
- **Damage variance**: 85-100%, as `100 - (Random() % 16)`
  (`decompiled/src/battle_script_commands.c:1558`).
- **Accuracy**: `(Random() % 100 + 1) > calc` (`:1093`).

The first of those is switched off here. `gBattleTypeFlags` reads `0x1C` at the start of the rival
battle -- `BATTLE_TYPE_IS_MASTER | BATTLE_TYPE_TRAINER | BATTLE_TYPE_FIRST_BATTLE`
(`decompiled/include/constants/battle.h:45`) -- and the crit condition carries
`&& (!(gBattleTypeFlags & BATTLE_TYPE_FIRST_BATTLE) || BtlCtrl_OakOldMan_TestState2Flag(1))`. So
until that tutorial flag is set, this battle cannot crit at all, and its whole spread is the damage
roll and accuracy.

**The battle is not luck-independent, and the route no longer pretends otherwise.** Delaying the
same A mash by a single frame flipped it from a win to a loss and back, over twelve consecutive
delays -- six wins, six losses, strictly alternating. `08-battle-win` therefore searches: it tries
16 start delays, keeps the shortest one that wins, and prints how many of them won. A route that
merely happens to win goes on to happen to lose the moment anything upstream moves by a frame.

## Which starter, measured

All three win under the same mash. Built end-to-end, one build each, **on mGBA 0.10.5** — the
absolute totals predate the 2026-08-11 core re-pin (Squirtle is 12209 on the current core) and
the battles would re-roll if remeasured, which does not change the conclusion below:

| Starter | Battle | Total | Attacks that landed |
| --- | ---: | ---: | ---: |
| Squirtle | 3461 | 11873 | 4 on the rival, 4 back |
| Charmander | 3681 | 12179 | 5 on the rival, 5 back |
| Bulbasaur | 3700 | 12194 | 5 on the rival, 5 back |

Read that as one sample each, not as a ranking. The rival always takes the counter to your pick, so
none of the three has a type edge; what separates them here is which damage rolls the stream
happened to hand out. Once the battle is manipulated properly the ordering can change, and the
comparison should be redone against manipulated battles rather than mashed ones.

## Frames saved locally can cost more than they save

`06-starter` held UP for 8 frames to turn towards the ball. One frame is enough, and trimming the
other seven saved 6 frames in that segment -- and cost **391** in the battle, because every frame
before a battle moves `gRngValue` and the battle that came out of the new stream needed two more
attacks. Net: 385 frames slower.

So knobs like that one are not a segment's decision. They live in `Tuning`, are recorded in the
ledger, and are swept end-to-end by `frlg route tune`, which builds the whole route per variant and
scores it on total frames to the win. All eight values, each a complete build (measured on mGBA
0.10.5; the sweep has not been redone since the 2026-08-11 core re-pin):

| `turn_hold` | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| total frames | 12258 | 12831 | 12270 | 12831 | 12036 | 12111 | 12037 | **11873** |

The untrimmed 8 wins, and every trim is between 163 and 958 frames worse. The spread has no shape to
it -- it is not "shorter is worse", it is the battle re-rolling -- which is the point: in front of an
RNG-sensitive fight, local greed is not merely unhelpful, it is uninformative. Measure through the
fight or do not measure.

## What is not optimised

The battle is now chosen rather than lucky, and one knob has been swept. Everything else is
untouched, and the frame counts above are the baseline the next route has to beat. Largest first:

- **`08-battle-win`, 3461 frames.** The search only varies *when* the mash starts. It never varies
  what the mash does -- move choice, or waiting a frame between turns to move the damage roll. Since
  criticals are off, the whole lever is the 85-100% roll, and a turn-by-turn search over small delays
  is the obvious next machine.
- **`02`/`03`, 3282 frames together.** Mashing A on the naming screen types whatever letter the
  cursor starts on **until the name fills up** -- watching the 2026-08-11 tier-2 replay made it
  visible just how bad that is: a full seven-character wall of A's, typed one press at a time,
  *and paid again on every later text box that prints the player or rival name*. Two unmeasured
  alternatives, cheapest first: a **one-character name** (type one letter, then END), and a
  **preset name** (Options on the naming screen are two D-pad presses away). Both have to be
  measured through the battle, per the finding above.
- **`06-starter`, 2496 frames.** Text speed is never set; the route does not open OPTIONS. The
  same replay-watching session makes this one look bigger than it did on paper: every message box
  in the run scrolls at MEDIUM. Whether the OPTIONS detour pays for itself over ~40 message boxes
  (plus the per-name-letter cost above) is arithmetic nobody has done here.
- **Starter choice.** Measured once each, mashed, not manipulated. Redo it against manipulated
  battles before treating Squirtle as settled.
- **LeafGreen is built and has never been raced.** The sandbox builds `pokeleafgreen.gba`
  byte-exact (`docs/sandbox.md`) and the harness only needs a ROM path and symbols, but every
  number in this file is FireRed. Version differences up to the rival fight are believed small
  (label: guess -- nobody has cited or measured them); the RNG stream will differ regardless, so
  the honest comparison is a full build-and-tune per version, same as the starter question.

## Tier 2

**Status (2026-08-11, evening): the pipeline runs end to end, and the first watched replay
desyncs.** The World BIOS is installed (sha1-verified), the route was rebuilt booting from it
(12222 frames, ledger `bios: "bios:300c20df…"`, 16/16 battle delays win), the export queued, and
EmuHawk actually played the movie on the host — power-on, menus, naming, into the bedroom. Then
it stopped lining up: the player never walks downstairs and the run stalls in the bedroom, so the
divergence is at or before the `03-names`→`04-house` movement. Observed by eye on the GUI; no
frame number yet — the runner's Lua status/RAM-compare path has still never completed, so
frame-level diagnosis is the open tier-2 item now. Two runner bugs were found and fixed on the
way: `--lua` was passed relative (EmuHawkMono.sh cd's to its own directory first), and
`--userdata` is not a data directory at all but movie metadata whose parser exits 1 on a bare
path (`--config` is the right flag).

**Settled: the format.** `route/template.bk2` is committed. It is a real one-frame BizHawk 2.11.1
movie, written by BizHawk's own `Bk2Movie` serialiser under mono, and it carries the two things
that were not derivable from anything mounted in the sandbox:

    LogKey  #Tilt X|Tilt Y|Tilt Z|Light Sensor|Up|Down|Left|Right|Start|Select|B|A|L|R|Power|
    empty   |    0,    0,    0,    0,...........|

and `SyncSettings.json`, the mGBA core's stock defaults (`SkipBios` true, `OverrideSaveType` -1,
`OverridePokemonRomhackDetect` true, …). Copy both verbatim. Regenerate with
`tools/bk2-template.sh` on the host whenever `BIZHAWK_VER` moves; it needs no GUI and no BIOS,
because it never instantiates a core.

Note the four leading analogue columns. `defctrl.json` lists ten buttons and the temptation is to
emit ten columns; the real GBA controller definition begins with Tilt X/Y/Z and the light sensor,
and `Power` trails the buttons rather than leading them. That is exactly the mistake the template
exists to prevent.

**Settled since (2026-08-11):**

- **The `.bk2` writer exists**: `frlg route export` (`crates/frlg-route/src/bk2.rs`). It
  concatenates the ledger's logs (refusing any whose digest the ledger does not vouch for),
  copies every template entry verbatim except `Input Log.txt`, and **round-trips the result** —
  the written movie is decoded back to key masks and compared before the export is reported;
  a mismatch deletes the file. The button mnemonics (`U D L R S s B A l r P`) were read from
  BizHawk's own `ControllerDefinition.MnemonicsCache` under mono, not guessed, and an exported
  route reads back through BizHawk's own `Bk2Movie.Load` with the right frame count. The `.ilog`
  files remain canonical; the `.bk2` is an export.
- **The cores no longer differ.** Tier 1 is pinned to `94b1578f`, the exact commit BizHawk 2.11.1
  bundles. `docs/harness.md` has what the port took; the re-pin moved the battle (route header
  above).

**Settled (2026-08-11 evening): the BIOS.** The World BIOS
(sha1 `300c20df6731a33952ded8c436f7f186d25d3492`, 16384 bytes) is at
`$BIZHAWK_HOME/Firmware/GBA_bios.rom`. Tier 1 boots from it the moment it exists
(`frlg_emu::boot_with_default_bios`): sha1-pinned, intro skipped — the same `GBASkipBIOS` path
BizHawk's own glue takes. The ledger records the boot per build (`"hle"` or `"bios:<sha1>"`),
`frlg route verify` refuses to replay logs under a different boot than they were built with, and
`frlg route export` warns loudly on an HLE-built route. `bin/frlg-doctor` checks the file and its
sha1 at startup.

**Still open, in order:**

1. **The desync.** The first watched replay stalls in the bedroom (see Status above). Both tiers
   run the same mGBA commit and the same BIOS boot, so the remaining suspects are BizHawk-side
   core configuration (`SyncSettings` beyond `SkipBios`, RTC handling) and input-delivery timing
   (when BizHawk latches a movie frame's keys vs. when `setKeys`+`runFrame` does). Getting the
   runner's Lua reporting to complete would turn "stalls in the bedroom" into a frame number and
   a RAM diff, which is the difference between suspecting and knowing.
2. **The runner's Lua path is still unexercised end to end** (`tools/verify-runner.lua`,
   memory-domain names especially). It has loaded and played a movie but never written a
   complete status/RAM report.

### Requesting a run, and reading the answer

`tools/verify-runner.sh` drains the queue on the host. Both sides depend on this contract, so it
lives here rather than in either script:

    in   $FRLG_ARTIFACTS/verify/queue/<id>.bk2     the movie to replay
         $FRLG_ARTIFACTS/verify/queue/<id>.json    optional, what the sandbox expects:
                                                   {"ilog_sha1", "ram_hash", "frames"}
    out  $FRLG_ARTIFACTS/verify/results/<id>.json

    {
      "id":                "08-battle-win",
      "bk2_sha1":          "…",   "ilog_sha1":  "…",   "rom_sha1": "…",
      "bizhawk_version":   "2.11.1",
      "verdict":           "pass" | "desync" | "error",
      "desync_frame":      null,
      "ram_hash":          "…",   "expected_ram_hash": "…",
      "finished_at":       "2026-08-11T18:04:00+02:00",
      "notes":             "replayed 12209 frames; fingerprint matches"
    }

`ram_hash` is the same fingerprint tier 1 computes — sha1 over EWRAM then IWRAM
(`docs/harness.md`) — which is what makes the two tiers comparable rather than merely both
green. The runner always writes a result, including for its own failures, and always removes the
queue entry: a request that died in a dialog must not look like one nobody has picked up yet.
`bin/frlg-doctor` prints the newest verdicts at startup.

The runner has **never completed a real replay** — it cannot, until the BIOS exists — so treat
its BizHawk-side details (`tools/verify-runner.lua`, the memory-domain names in particular) as
the least-tested code in this repository.
