# The route: power-on to a beaten rival

10946 frames (~3m03s at 59.7275 Hz) from reset to `gBattleOutcome == B_OUTCOME_WON`, with
Squirtle, on FireRed. Tier 1 verified from reset; **tier 2 for this movie is queued
(`route-10946f-b1a0875a77e9`), not yet replayed**. The 2026-08-11 tier-2 pass — BizHawk
replaying the whole movie to the same fingerprint, frame for frame — belongs to the previous,
12713-frame build of this same route; see [Tier 2](#tier-2). Rebuilding resets the tier-2
stamp, and should.

The route is rebuilt whenever the boot or the core moves, and the total has moved with it:
11873 (mGBA 0.10.5, HLE BIOS) → 12209 (2026-08-11, tier 1 re-pinned to the exact mGBA commit
BizHawk bundles, `94b1578f`, `docs/harness.md`) → 12222 (real-BIOS boot, intro skipped) →
12713 (2026-08-12: real-BIOS boot with the ~272-frame boot animation *played*, because that
is the only boot BizHawk uses for a movie — the desync fix below) → **10946** (2026-08-12:
the first pure routing win — one-character player name, preset rival name, text speed FAST
and battle animations off; nothing about the emulator moved). Segments survive core re-pins
shifted but intact; the battle RNG stream never does, so `09-battle-win` re-searches its
start delay each time. Each re-pin surfacing a real delta *before* tier 2 had to find it is the
pinning doing its job.

LeafGreen builds byte-exact in the same tree and the harness only needs a ROM path and symbols;
the two versions are typically one speedrun category, so the plan is to route both and keep
whichever is faster. Every number in this file is FireRed until a LeafGreen build exists.

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
| `01-boot` | 619 | 619 | `CB2_NewGameScene` -- NEW GAME taken (includes the ~272-frame BIOS animation) |
| `02-intro-oak` | 1842 | 2461 | `CB2_NamingScreen` -- Oak's speech and the boy/girl choice done |
| `03-names` | 1238 | 3699 | `CB2_Overworld` in the bedroom, player name 1 char, rival name 3 (KAZ) |
| `04-options` | 197 | 3896 | `optionsTextSpeed == FAST`, `optionsBattleSceneOff` set, back on the field |
| `05-house` | 431 | 4327 | map is Pallet Town (3.0) |
| `06-to-lab` | 1209 | 5536 | map is Oak's lab (4.3) |
| `07-starter` | 1702 | 7238 | `gPlayerPartyCount == 1`, `VAR_STARTER_MON` set, lab scene var 3 |
| `08-battle-start` | 386 | 7624 | `gMain.inBattle` |
| `09-battle-win` | 3322 | 10946 | `gBattleOutcome == B_OUTCOME_WON` (8/16 start delays win; delay 1 kept) |

Against the 12713-frame predecessor: `03-names` types one letter and takes START's shortcut to
OK (`decompiled/src/naming_screen.c:1485`) instead of filling seven, and picks KAZ off the
rival's preset menu (`sRivalNameChoices`, `decompiled/src/oak_speech.c:647`) instead of a second
naming screen; `04-options` is new and costs 197 frames; and every segment after it is cheaper
because its message boxes print at 1 frame per character instead of 4
(`sTextSpeedFrameDelays`, `decompiled/src/new_menu_helpers.c:27-32`) and the battle plays no
attack animations (`optionsBattleSceneOff` -> `HITMARKER_NO_ANIMATIONS`,
`decompiled/src/battle_main.c:2259`). The detour repays itself about nine times over before the battle even starts
(`07-starter` alone dropped 794 frames), and the battle — a fresh RNG stream, re-searched —
came out 696 frames shorter on top.

Map ids are `(group, number)` indices into `decompiled/data/maps/map_groups.json`.

## The things the route has to get right

**The player never gets a preset menu; the rival always does.** The intro's naming flow is
asymmetric: `Task_OakSpeech_YourNameWhatIsIt` fades straight into the naming screen
(`decompiled/src/oak_speech.c:1352-1379`) — the player's preset menu only exists on the
say-NO re-ask path, which costs a round trip to reach. The rival's menu is the first thing
asked (`Task_OakSpeech_MoveRivalDisplayNameOptions` → `PrintNameChoiceOptions`,
`oak_speech.c:2117`), its rows are literal (`sRivalNameChoices` row 3 is KAZ), and it wraps,
so two UPs reach KAZ from the top. On the naming screen itself, START jumps the cursor to OK
(`HandleKeyboardEvent`, `decompiled/src/naming_screen.c:1485`) and a one-character name is
accepted (`SaveInputText`, `:1851`). One letter, START, A: seven fewer characters than the
old mash, paid back on every message box that prints the name.

**Single-frame taps die in the start menu.** Measured on this core: while the start menu is
up, `gMain.newKeys` goes stale for runs of 2-3 frames — the game skips input reads — so a
1-frame press can land on a frame nobody reads and vanish. The same is true of the field for
~20 frames after a walk-in transition (`Task_ExitNonDoor` still running when
`gPlayerAvatar` first says the player can step). `04-options` therefore never taps: every
press is a mash-until-effect — mash UP until `sStartMenuCursorPos` reads 3, mash RIGHT until
the menu's working value reads FAST — which stops on the frame the effect lands and cannot
overshoot, because the next registrable edge is frames away.

**Oak's interruption is the way into the lab.** Walking to Pallet Town `(12,1)` fires
`PalletTown_EventScript_OakTriggerLeft` (`decompiled/data/maps/PalletTown/map.json`, `coord_events`),
which ends in `warp MAP_PALLET_TOWN_PROFESSOR_OAKS_LAB`. The route walks onto that tile and then
mashes A through the scene.

**Two prompts want different answers.** `..._EventScript_ConfirmStarterChoice` asks YES/NO to the
starter, and A takes YES. `EventScript_ChoseStarter` then asks YES/NO to a *nickname*, where YES
costs an entire naming screen. So `07-starter` mashes A only until the mon is in the party -- the
`givemon` happens before the nickname prompt -- and switches to B for the rest, which answers no and
still advances every message
(`decompiled/data/maps/PalletTown_ProfessorOaksLab/scripts.inc`).

**The battle trigger is inert until the rival has his.** The `coord_events` on row `y=8` only fire
`..._EventScript_RivalBattleTrigger*` when the lab scene var is 3, which the rival taking his ball
sets. `07-starter` therefore does not end when the player has a mon; it ends when the scene does.

## What the evidence is

`frlg route verify` starts one emulator at reset and replays the committed `.ilog` files in order.
For each it checks the file's digest against the ledger, then asks the segment's own `reached`
predicate whether the game is where the segment says. It fills in `tier1` from what it saw rather
than copying what the builder claimed. `crates/frlg-route/tests/route.rs` is the same check as a
test, and also compares every segment's RAM fingerprint against the ledger.

The nine logs joined into one file (`frlg log cat`) replay to the same fingerprint as the
segmented run, `5f82b4e397ec072dfbcfe3648bd21d32b572da76`, ending on `gBattleOutcome = 1`,
`gPlayerPartyCount = 1`. `frlg route export` re-proves that join on every export: it replays the
combined movie from reset and refuses to queue one whose final fingerprint is not the ledger's.

## What the RNG does in this battle, and what it does not

`Random()` is an LCG over `gRngValue`, returning the top 16 bits
(`decompiled/src/random.c`). Two seedings bracket the intro: the title screen seeds from
timer 1 (`SeedRngAndSetTrainerId`, `decompiled/src/title_screen.c:735`,
`decompiled/src/main.c:264`), and leaving the *player* naming screen seeds again from the
same timer (`MainState_Exit`, `decompiled/src/naming_screen.c:722` — the player template
only, not the rival's). Everything from the bedroom to the battle therefore runs on a stream
whose seed is fixed at naming-screen exit and advanced once per frame
(`decompiled/src/main.c:412`); manipulation upstream of that exit cannot reach the battle
except by moving the exit itself. Three things in a normal battle consume the stream:

- **Criticals**: `!(Random() % sCriticalHitChance[critChance])`, base chance 1 in 16
  (`decompiled/src/battle_script_commands.c:1199`, table at `:588`).
- **Damage variance**: 85-100%, as `100 - (Random() % 16)`
  (`decompiled/src/battle_script_commands.c:1558`).
- **Accuracy**: `(Random() % 100 + 1) > calc` (`:1093`).

The first of those is switched off here **only for the opening turns, not for the battle**. This
file used to say "criticals are off in this battle" flatly; watching the 2026-08-11 tier-2 replay
disproved it — the rival's Bulbasaur lands a critical hit on us — and the decomp agrees with the
screen. `gBattleTypeFlags` reads `0x1C` at the start of the rival battle
(`BATTLE_TYPE_IS_MASTER | BATTLE_TYPE_TRAINER | BATTLE_TYPE_FIRST_BATTLE`,
`decompiled/include/constants/battle.h:45`) and the crit condition carries
`&& (!(gBattleTypeFlags & BATTLE_TYPE_FIRST_BATTLE) || BtlCtrl_OakOldMan_TestState2Flag(1))`
(`decompiled/src/battle_script_commands.c:1200`). The second half of that `||` is the part that
was missed: `BtlCtrl_OakOldMan_TestState2Flag(1)` reads
`gBattleStruct->simulatedInputState[2] & FIRST_BATTLE_MSG_FLAG_INFLICT_DMG`
(`decompiled/src/battle_controller_oak_old_man.c:2228`, constant `0x1` at
`decompiled/include/battle_controllers.h:287`), and that flag is **set the first time an
opponent's hit finishes draining the health bar** — `CompleteOnHealthbarDone` sets it and hands
over to `PrintOakText_InflictingDamageIsKey`
(`decompiled/src/battle_controller_opponent.c:304-306`). So the tutorial suppresses criticals
until Oak has given his "inflicting damage is key" line, and from that moment on both sides can
crit at the normal 1-in-16.

Two consequences for manipulation:

- **The crit roll always burns RNG**, suppressed or not. `&&` short-circuits left to right, and
  `!(Random() % sCriticalHitChance[critChance])` sits *before* the `FIRST_BATTLE` clause in the
  condition, so the call happens on every damaging hit even while the result is being thrown
  away. Any model of this battle's stream has to count it.
- **The rival's crit is a legitimate search target.** It is one `Random()` outcome in a stream
  the route already re-searches; see "What is not optimised".

**The battle is not luck-independent, and the route no longer pretends otherwise.** Delaying the
same A mash by a single frame flipped it from a win to a loss and back, over twelve consecutive
delays -- six wins, six losses, strictly alternating. `09-battle-win` therefore searches: it tries
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

`07-starter` held UP for 8 frames to turn towards the ball. One frame is enough, and trimming the
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

That sweep is now two route generations stale (it predates both the core re-pin and the 10946
rebuild); `turn_hold = 8` is carried forward unre-derived, and re-running `frlg route tune` on the
current route is cheap insurance nobody has bought yet.

## What is not optimised

The 2026-08-12 rebuild routed out the three cheapest items this section used to carry: the
seven-character names, MID text speed, and battle animations (197 frames of `04-options` bought
all three; the section header above has the per-segment arithmetic). What remains, largest
first:

- **`09-battle-win`, 3322 frames.** The search only varies *when* the mash starts. It never varies
  what the mash does -- move choice, or waiting a frame between turns to move the damage roll. A
  turn-by-turn search over small delays is the obvious next machine, and it has two levers:
  the 85-100% damage roll, and the crit roll. Criticals are live from Oak's "inflicting damage
  is key" line onwards (see the RNG section above), for both sides: a rival crit costs damage
  and a message box, and an own crit saves a turn -- both are single `Random()` outcomes in a
  stream the search already moves. Whether the current 3322-frame battle contains either has
  not been checked; the previous battle demonstrably ate a rival crit on the tier-2 recording.
- **The intro's text, 3699 frames of `01`-`03`, still prints at MID.** The option menu hangs off
  the field start menu (`StartMenuOptionCallback`, `decompiled/src/start_menu.c:531`), and there
  is no field until the bedroom, so every box before `04-options` pays 4 frames a character no
  matter what. Shrinking that means fewer characters, not faster ones -- and the boxes are Oak's
  speech, which no menu shortens. Structural, until someone finds a skip.
- **Starter choice.** Measured once each, mashed, not manipulated, on mGBA 0.10.5 -- two route
  generations ago. Redo it against manipulated battles before treating Squirtle as settled.
- **`turn_hold` is carried forward unre-derived** -- see the tuning table above.
- **LeafGreen is built and has never been raced.** The sandbox builds `pokeleafgreen.gba`
  byte-exact (`docs/sandbox.md`) and the harness only needs a ROM path and symbols, but every
  number in this file is FireRed. Version differences up to the rival fight are believed small
  (label: guess -- nobody has cited or measured them); the RNG stream will differ regardless, so
  the honest comparison is a full build-and-tune per version, same as the starter question.

## Tier 2

**Status (2026-08-12, sandbox): `route-10946f-b1a0875a77e9` is queued and has not been
replayed.** The current 10946-frame movie exists only as tier-1 evidence plus a queue entry
(with its `gRngValue` trace); a host run of `tools/verify-runner.sh` is what turns it into a
result.

**The previous build passed (2026-08-11, host): `route-12713f-a4ad4280bbdc`.** BizHawk 2.11.1
replayed all 12713 frames of the pre-optimisation route, ended on the same EWRAM+IWRAM
fingerprint tier 1 computed (`73b329af5d561a864cc4b0d46e8d4c409ce1b6df`), and matched the
per-frame `gRngValue` probe on **every single frame** — not one divergence anywhere in the run,
which is a far stronger statement than the final hashes agreeing. The result is
`$FRLG_ARTIFACTS/verify/results/route-12713f-a4ad4280bbdc.json`. That closes the bedroom
desync, and with it the only open question about whether this route *family* exists on real
hardware timing — the 10946 rebuild changes inputs, not boot, core, or format, so the plausible
desync causes are all ones the 12713 pass already ruled out. Plausible is not proven: the new
movie still has to replay. Everything below about the desync is kept because the root cause is
worth not rediscovering.

The desync was the boot: BizHawk *movie playback* never skips the BIOS intro. `MGBAHawk.cs:41` (2.11.1 sources,
`$FRLG_ARTIFACTS/reference/bizhawk-2.11.1/`) passes
`skipBios: _syncSettings.SkipBios && !lp.DeterministicEmulationRequested`, and loading a movie
requests deterministic emulation — that is precisely why line 30's `MissingFirmwareException`
fired on the host until the BIOS existed. So the template's `SkipBios: true` is overridden to
false for every replay, `bizinterface.c:171`'s `GBASkipBIOS` call never happens, and the
~272-frame boot animation plays with movie input already being consumed. Tier 1 booted with
`opts.skipBios = true`, so its whole log ran ~272 frames early on BizHawk: mash segments
absorbed the shift, and the first frame-exact walking (the bedroom) died — exactly what was
watched. Tier 1 now boots BIOS-with-intro (`Emu::load_bios(_, false)`, ledger marker
`bios+intro:<sha1>`), the route was rebuilt (12713 frames, 16/16 battle delays win), and that
rebuild is what passed. One cited root cause, one fix, one clean replay: no second cause ever
existed, which is what the 2026-08-12 audit predicted.

Two runner bugs were found and fixed on the way to the first replay: `--lua` was passed
relative (EmuHawkMono.sh cd's to its own directory first), and `--userdata` is not a data
directory at all but movie metadata whose parser exits 1 on a bare path (`--config` is the
right flag).

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

**Settled by the pass (2026-08-11): the whole pipeline.** Every piece of tier 2 has now done its
job once, which is the difference between "written" and "working": the `.bk2` writer's output
loads and plays, the `.ilog` -> movie join is frame-exact on a second emulator, the Lua reports
(status file, 288K RAM dump, per-frame probe compare), and the shell turns all of that into a
verdict. `tools/verify-runner.lua` was the least-exercised code in this repository for three
sessions; it is not any more.

**Still open, in order:**

1. **Only the whole route has a tier-2 result, never a segment.** The queue and the runner are
   per-request, and one request has ever been made. A segment-level replay would localise a
   future desync without a bisect, and costs nothing but export plumbing.
2. **Nothing about the runner, for once.** A pass costs 31s headless against the movie's own
   213s, and is CPU-bound rather than waiting on anything — see "Making a replay cheap" below.
   Further speed would have to come out of BizHawk's frame loop or the Lua's per-frame probe
   read, which is not worth doing at 31s.

### Requesting a run, and reading the answer

`tools/verify-runner.sh` drains the queue on the host. The Lua it hands EmuHawk is
`$FRLG_ARTIFACTS/verify/verify-runner.lua` **when that file exists**, else the checked-in
`tools/verify-runner.lua` — the override exists because the runner executes from the host
checkout while Lua fixes are authored in the sandbox's clone, and iterating on the Lua must not
need a repo round trip per attempt. When the override version stabilises, fold it back into the
repo and delete the override. Both sides depend on this contract, so it lives here rather than
in either script:

    in   $FRLG_ARTIFACTS/verify/queue/<id>.bk2     the movie to replay
         $FRLG_ARTIFACTS/verify/queue/<id>.json    optional, what the sandbox expects:
                                                   {"ilog_sha1", "ram_hash", "frames",
                                                    "trace": {"file", "domain", "offset",
                                                              "size", "symbol", "frames"}}
         $FRLG_ARTIFACTS/verify/queue/<id>.trace   optional, the per-frame probe the request's
                                                   "trace" describes: one little-endian u32 per
                                                   frame, sampled by tier 1 at each frame's end.
                                                   `frlg route export` writes gRngValue, which
                                                   the game advances once per VBlank
                                                   (decompiled/src/main.c:412), so the first
                                                   mismatching frame *is* the divergence frame.
    out  $FRLG_ARTIFACTS/verify/results/<id>.json

    {
      "id":                "08-battle-win",
      "bk2_sha1":          "…",   "ilog_sha1":  "…",   "rom_sha1": "…",
      "bizhawk_version":   "2.11.1",
      "verdict":           "pass" | "desync" | "error",
      "desync_frame":      null,      // first frame where the probe trace differed, when known
      "ram_hash":          "…",   "expected_ram_hash": "…",
      "replay_mode":       "fast",    // fast | realtime, each optionally "+headless"
      "duration_s":        134,       // wall clock, so a slow run is visible as one
      "finished_at":       "2026-08-11T18:04:00+02:00",
      "notes":             "replayed 12209 frames; fingerprint matches"
    }

`bk2_sha1` identifies the bytes that were replayed, but do not expect it to be stable across
exports: the `.bk2` is a zip and its entry timestamps move, so two exports of the same route
have the same `ilog_sha1` and different `bk2_sha1`. The `.ilog` digest is the identity.

`ram_hash` is the same fingerprint tier 1 computes — sha1 over EWRAM then IWRAM
(`docs/harness.md`) — which is what makes the two tiers comparable rather than merely both
green. The runner always writes a result, including for its own failures, and always removes the
queue entry: a request that died in a dialog must not look like one nobody has picked up yet.
`bin/frlg-doctor` prints the newest verdicts at startup.

### Making a replay cheap

A verification replay has no audience, so paying for one is waste. Out of the box EmuHawk
replays at 100%: a 12713-frame movie costs the 3m33s the TAS itself costs, which is enough to
make a person not bother. `tools/verify-runner.sh` therefore seeds `config.ini` (it is plain
JSON) before every launch, rather than expecting anyone to click through a GUI it never opens:

| Setting | Value | Why |
| --- | --- | --- |
| `Unthrottled` | `true` | run frames as fast as the host manages |
| `ClockThrottle`, `VSyncThrottle`, `SoundThrottle` | `false` | the three things that would otherwise pace it |
| `DispSpeedupFeatures` | `0` | `MainForm::Render` returns immediately without touching the video provider (EmuHawk.exe IL) |
| `SoundEnabled` | `false` | nothing to listen to |
| `SoundOutputMethod` | `3` = `ESoundOutputMethod.Dummy` | opens no audio device at all — verified with `monop` on `BizHawk.Client.Common.dll`. **BizHawk writes this back as `2` (OpenAL)**, so it is being overridden somewhere on Linux; `SoundEnabled: false` is what actually silences it |
| `PauseWhenMenuActivated`, `SuppressAskSave`, `UpdateAutoCheckEnabled`, … | — | every modal dialog is a hang in an unattended runner |

**None of this touches emulation**, and that is checked rather than assumed: every replay below
produced the same fingerprint *and* the same 12713-frame probe trace as the original
100%-with-sound run. `--realtime` puts the desk settings back for when a person does want to
watch, which is not a luxury — watching the 2026-08-11 replay is what produced three of the
route findings above.

`--headless` runs EmuHawk under `xvfb-run` rather than on the desktop. EmuHawk is WinForms under
mono and will not start without an X display, so a throwaway one is the only headless available;
nothing is drawn on it. It needs `xvfb` on the host (`sudo apt install xvfb`) and the preflight
says so when it is missing.

The four measurements that set the defaults, same movie every time:

| Run | Wall clock | Effective |
| --- | ---: | ---: |
| Stock EmuHawk (100%, sound, rendering) | ~213s | 59.7 fps — the movie's own length |
| Seeded config, on the desktop | 134s | ~95 fps |
| Seeded config, `--headless` | **31s** | ~410 fps |
| `--headless`, `DispSpeedupFeatures` 1 or 2 | 47s | ~270 fps |

**6.9x, and the win is mostly the X server, not the throttle.** Unthrottling alone bought 1.6x
because something still waited ~8ms per frame; the process was idle for three quarters of that
134s. Headless removed the wait entirely — 32s of CPU across 31s of wall clock is a replay that
is finally CPU-bound. That fingers the desktop X connection rather than any emulator setting,
and the most likely single culprit is the per-frame window title: with `DispSpeedupFeatures == 0`,
`CalcFramerateAndUpdateDisplay` takes the branch that calls `FormBase::UpdateWindowTitle()` on
*every* frame (EmuHawk.exe IL), which is an X11 round trip per frame under mono. Cheap on a
local Xvfb, expensive against a real desktop.

The last row is the same experiment run the other way, and it says the rendering setting is
worth keeping: letting EmuHawk render (1 or 2) costs 16s even with nothing to display it on,
while 0 makes `MainForm::Render` return immediately. Rerun any of these with
`FRLG_VERIFY_CONFIG_EXTRA='{"DispSpeedupFeatures": 2}'`, which is applied on top of the seeded
config for exactly this purpose.

Further speed now has to come out of per-frame work, not configuration: tier 1 replays the same
12713 frames in ~12s (~1070 fps), so BizHawk's frame loop plus the Lua's per-frame probe read
costs about 2.6x what mGBA alone does. Nobody has tried to shrink that, and at 31s nobody needs
to yet.

The sandbox still cannot run tier 2 itself under any of this: BizHawk needs mono, and the
sandbox has none and may not install one. What headless buys is an *unattended* runner —
`tools/verify-runner.sh --watch --headless` on the host drains the queue without taking over a
screen, which is as close to "the sandbox runs tier 2" as the closed network allows.
