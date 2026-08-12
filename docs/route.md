# The route: power-on to a beaten rival

9658 frames (~2m42s at 59.7275 Hz) from reset to `gBattleOutcome == B_OUTCOME_WON`, with
**Bulbasaur, on LeafGreen** — both picked by measurement, not preference; the tables below
are the evidence. Tier 1 verified from reset; **tier 2 passed this movie** on 2026-08-12
(`route-9658f-269d169cd6db`, BizHawk 2.11.1, every frame of the probe matching) — the first
LeafGreen replay, so the version-aware header rewrite is now proven format knowledge and not
just plumbing; see [Tier 2](#tier-2). Rebuilding resets the tier-2 stamp, and should.

The route is rebuilt whenever the boot or the core moves, and the total has moved with it:
11873 (mGBA 0.10.5, HLE BIOS) → 12209 (2026-08-11, tier 1 re-pinned to the exact mGBA commit
BizHawk bundles, `94b1578f`, `docs/harness.md`) → 12222 (real-BIOS boot, intro skipped) →
12713 (2026-08-12: real-BIOS boot with the ~272-frame boot animation *played*, because that
is the only boot BizHawk uses for a movie — the desync fix below) → 10946 (2026-08-12:
the first pure routing win — one-character player name, preset rival name, text speed FAST
and battle animations off; nothing about the emulator moved) → 10531 (2026-08-12: the
battle search grew a second stage, per-turn delays) → 10085 (2026-08-12: `turn_hold`
re-swept on the new route; the stale 8 fell to 2) → **9658** (2026-08-12: hold A through
dialogue instead of mashing it (`text_hold`, below), repeat the battle search's per-turn
stage to a fixpoint, then sweep all six version × starter cells — LeafGreen with Bulbasaur
wins). Segments survive core re-pins shifted but intact; the battle RNG stream never does,
so `09-battle-win` re-searches its delays each time. Each re-pin surfacing a real delta
*before* tier 2 had to find it is the pinning doing its job.

Both versions build byte-exact in the same tree, the route reads which one it is driving
from the ROM header (BPRE/BPGE at 0xAC, `decompiled/config.mk:29-57`), and the two are one
speedrun category — so every rebuild is free to re-pick the version, and this one did.

    frlg route build       # run the segments, write route/logs/*.ilog and route/ledger.json
    frlg route verify      # replay the committed logs from reset and check every claim
    frlg route status      # print the ledger
    frlg route tune        # sweep the route-level knobs serially, scored on total frames
    bin/frlg-sweep         # the same sweep as parallel builds -- hours become one wave

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
| `01-boot` | 615 | 615 | `CB2_NewGameScene` -- NEW GAME taken (includes the ~272-frame BIOS animation) |
| `02-intro-oak` | 1565 | 2180 | `CB2_NamingScreen` -- Oak's speech and the boy/girl choice done |
| `03-names` | 1043 | 3223 | `CB2_Overworld` in the bedroom, player name 1 char, rival name 3 (RED) |
| `04-options` | 197 | 3420 | `optionsTextSpeed == FAST`, `optionsBattleSceneOff` set, back on the field |
| `05-house` | 431 | 3851 | map is Pallet Town (3.0) |
| `06-to-lab` | 1211 | 5062 | map is Oak's lab (4.3) |
| `07-starter` | 1785 | 6847 | `gPlayerPartyCount == 1`, `VAR_STARTER_MON` set, lab scene var 3 |
| `08-battle-start` | 402 | 7249 | `gMain.inBattle` |
| `09-battle-win` | 2409 | 9658 | `gBattleOutcome == B_OUTCOME_WON` (delay plan `[4, 3, 3, 3]`, 3 turns) |

Against the 10085-frame predecessor, three things changed:

- **Dialogue is held, not mashed** (`Tuning::text_hold`, below): once one press lands during
  a box, `RenderText` prints a character on *every* frame A or B is held
  (`decompiled/src/text.c:639-650`), so the old `[A, 0]` mash printed at half the held rate
  everywhere the route talks -- including Oak's speech, where no menu can reach the text-speed
  option yet, and the battle's own message windows
  (`decompiled/src/battle_message.c:2778-2785`). Holding 4 frames per 1-frame release cut the
  pre-options intro from 3699 to 3223 frames on its own.
- **The battle search repeats its per-turn stage until a pass adopts nothing**, instead of
  walking the turns once and never revisiting an earlier turn on the stream a later adoption
  changed.
- **The version and the starter were both re-picked by a full sweep** (tables below):
  LeafGreen with Bulbasaur, `turn_hold` 4, `text_hold` 4. LeafGreen boots 4 frames faster,
  its 3-character rival preset (RED) is one DOWN instead of FireRed's two wrapping UPs
  (`sRivalNameChoices`, `decompiled/src/oak_speech.c:649-658`), and its Bulbasaur stream
  yields a 3-turn, 2409-frame battle.

Map ids are `(group, number)` indices into `decompiled/data/maps/map_groups.json`.

## The things the route has to get right

**The player never gets a preset menu; the rival always does.** The intro's naming flow is
asymmetric: `Task_OakSpeech_YourNameWhatIsIt` fades straight into the naming screen
(`decompiled/src/oak_speech.c:1352-1379`) — the player's preset menu only exists on the
say-NO re-ask path, which costs a round trip to reach. The rival's menu is the first thing
asked (`Task_OakSpeech_MoveRivalDisplayNameOptions` → `PrintNameChoiceOptions`,
`oak_speech.c:2117`), its rows are literal and version-dependent (`sRivalNameChoices`,
`oak_speech.c:649-658`: GREEN/GARY/KAZ/TORU on FireRed, RED/ASH/KENE/GEKI on LeafGreen), and
it wraps (`Menu_MoveCursor`, `decompiled/src/menu.c:306`). The shortest name is 3 characters
on both versions but on different rows: KAZ is row 3 (two UPs, wrapping), RED is row 1 (one
DOWN). On the naming screen itself, START jumps the cursor to OK (`HandleKeyboardEvent`,
`decompiled/src/naming_screen.c:1485`) and a one-character name is accepted (`SaveInputText`,
`:1851`). One letter, START, A: seven fewer characters than the old mash, paid back on every
message box that prints the name.

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
segmented run, `e65e93b6712b408ed915f55e46c9a79f874016cd`, ending on `gBattleOutcome = 1`,
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
except by moving the exit itself.

Since 2026-08-12 the stream has a Rust model (`crates/frlg-rng`: step, O(log n) jump, and
the discrete log `distance_to`, so "how many `Random()` calls happened between these two
observed states" is a 135 ns question instead of a replay). Its correctness is not argued
from the constants: `frlg-rng`'s `tests/emulator.rs` replays the whole committed route and
requires the model to reproduce `gRngValue` on every single frame, with only the two
`SeedRng` events breaking stride. The `rng-trace` example narrates the route in
"extra steps beyond the per-frame VBlank call", which is exactly the set of frames where
some other consumer rolled.

**Who consumes the stream in the field** — measured by that trace and by
`field-experiments`, each claim also cited:

- **The VBlank interrupt, once per displayed frame, unconditionally**
  (`decompiled/src/main.c:412`, handler installed at `:52`; it does not depend on
  `gMain.vblankCallback`). In battle this doubles: `VBlankCB_Battle` adds a second call
  per frame (`decompiled/src/battle_main.c:1650`, installed at `:698`) — measured, every
  frame of `09-battle-win` moves the stream by exactly 2.
- **Pressing A: never.** No `Random()` anywhere on the text/menu/interaction path
  (`src/text.c`, `src/menu.c`, `src/field_control_avatar.c`, `src/start_menu.c`,
  `src/script.c` all have zero calls; `src/scrcmd.c:459` is the script `random` command,
  unused by these maps). Measured: 600 frames idle, holding A, and mashing A from the same
  state consume identically. A presses are free as far as the stream is concerned.
- **Player walking and turning: never** (every `Random()` in `src/field_player_avatar.c`
  is in the fishing minigame). Measured: two same-length paths to the same tile left
  `gRngValue` identical at a common frame horizon.
- **NPCs with rolling movement types.** `MovementType_WanderAround` rolls a delay and a
  direction per cycle (`decompiled/src/event_object_movement.c:2716,2737`); the
  look-around and wander-up/down variants likewise (`:3037,3061,3090,3110`); plain
  `FACE_*` NPCs never roll. On this route the only field roller is Pallet Town's **fat
  man** (13,17) (`decompiled/data/maps/PalletTown/map.json`) — the sign lady reads as a
  wanderer in map.json but the map's on-load script parks her at (5,15) as
  `MOVEMENT_TYPE_FACE_UP` until her scene plays
  (`data/maps/PalletTown/scripts.inc:27-28`), confirmed against `gObjectEvents` by the
  `who-rolls` example — plus Oak's lab's three aides
  (`.../PalletTown_ProfessorOaksLab/map.json`). Two gates matter: an object event
  outside the spawn window around the player (live iff `px-9 <= tx <= px+10` and
  `py-7 <= ty <= py+9` in template coords, `event_object_movement.c:1798-1801`) is
  despawned and rolls nothing — the fat man is despawned at the house door and spawns as
  the player moves south, which makes his roll count the route's one free stream lever —
  and
  `lockall`/`lock` scripts freeze object events entirely (`:5117`,
  `src/scrcmd.c:1195-1221`), so the aides stop rolling for the whole scripted rival
  sequence. Measured idle rates per 600 frames: bedroom 2F (no object events) 0 extra
  steps, Pallet Town 14-16.
- **Map loads: 3 rolls each**, every warp and every battle entry — saveblock ASLR offset
  (`decompiled/src/load_save.c:75`) plus a 2-roll encryption key (`:126`), reached from
  `InitOverworldBgs` (`src/overworld.c:1337`) and from `CB2_InitBattle`
  (`src/battle_main.c:614`).
- **Ambient cries, outdoors only**: a species pick per map load and a delay roll every
  1200-3600 unlocked frames (`decompiled/src/overworld.c:1141-1172`); indoors has no
  wild-mon header and consumes nothing.

Three things in a normal battle consume the stream beyond the per-frame pair:

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
delays -- six wins, six losses, strictly alternating. `09-battle-win` therefore searches, in two
stages, both scored on the whole battle's frame count with wins the only candidates. Stage 1
tries 64 start delays -- wide, because winning battles on adjacent delays differ by hundreds of
frames while the widest delay costs 63. Stage 2 exploits the fact that the battle re-enters
`HandleTurnActionSelectionState` once per turn (`BattleTurnPassed`,
`decompiled/src/battle_main.c:2998`): walking the winning battle's turns in order, it tries
idling 1-15 frames at each turn's menu, replays the rest of the battle in full per trial, and
adopts only a shorter *battle* -- never a shorter turn, which is the `turn_hold` lesson again.
Single per-turn delays have been worth three-digit frame counts on every stream stage 2 has
seen (~1200 on the 10531 build's stream, ~400 on the current one): they move the whole
damage-roll lineage, not a margin. A route that merely happens to win goes on to happen to
lose the moment anything upstream moves by a frame.

## Which version, which starter — measured, 2026-08-12

Every cell below is the best of a full 24-variant tuning sweep (`turn_hold` 1–8 ×
`text_hold` {1, 2, 4}, `bin/frlg-sweep`), each variant a complete build from reset through
the whole two-stage battle search, on the current core and boot. Total frames to the win:

| | Squirtle | Charmander | Bulbasaur |
| --- | ---: | ---: | ---: |
| FireRed | 9789 | 9749 | 9666 |
| LeafGreen | 9747 | 9741 | **9658** |

The rival always takes the counter to your pick, so no starter has a type edge; what the
table ranks is which RNG stream families produce short battles. Both Bulbasaur cells win
with 3-turn, 2409-frame battles — and Bulbasaur was the *worst* starter in the old
one-mashed-sample table this section used to carry (12194 on mGBA 0.10.5, against Squirtle's
11873), which is the measurement lesson in one line: an unmanipulated sample ranks nothing.
The fragility is real, though: 10 of LeafGreen/Bulbasaur's 24 variants and 5 of
FireRed/Bulbasaur's could not win their battle at all, the highest lose rates of any cell.
The per-variant tables are in `docs/journal.md`.

## Frames saved locally can cost more than they save

`07-starter` held UP for 8 frames to turn towards the ball. One frame is enough, and trimming the
other seven saved 6 frames in that segment -- and cost **391** in the battle, because every frame
before a battle moves `gRngValue` and the battle that came out of the new stream needed two more
attacks. Net: 385 frames slower.

So knobs like that one are not a segment's decision. They live in `Tuning`, are recorded in
the ledger, and are swept end-to-end — `frlg route tune` serially, `bin/frlg-sweep` as
parallel builds — with every variant a complete build scored on total frames to the win.
There are two knobs now:

- **`turn_hold`** — frames of UP held to face the starter's ball. Mechanically 1 frame is
  enough; the value picks which RNG lineage the battle search fishes in, nothing more.
- **`text_hold`** — frames A/B is held per one-frame release in every dialogue mash. Longer
  holds print text faster (`decompiled/src/text.c:639-650`, one character per held frame once
  a press has landed) but register each menu-advancing press later, and the release phase has
  to line up with when boxes become ready, so the landscape is alignment, not a curve:
  measured on the intro alone (upstream of the naming-screen reseed, so no battle re-roll
  muddies it), 1 → 3699, 2 → 3361, 3 → 3584, 4 → 3229, 7 → 3591, 15 → 3719, 31 → 3988 frames.
  The ignored test `text_hold_on_the_intro_alone` reruns that measurement.

The winning combination differs per version × starter cell (see the table above), which is
the same lesson the first `turn_hold` sweep taught, now in two dimensions: in front of an
RNG-sensitive fight, local greed is uninformative — measure through the fight or do not
measure. A variant whose stream cannot win its battle at all is recorded as "loses" and is an
answer, not an outage (turn_hold 7 on the 10085 route was the first; the Bulbasaur cells have
several each). The older single-knob sweeps are in the git history and `docs/journal.md`.

## What is not optimised

The 2026-08-12 rebuilds routed out this section's cheapest items in two rounds: first the
seven-character names, MID text speed and battle animations (197 frames of `04-options`
bought all three), then the mash itself (`text_hold` — the intro was "structural, until
someone finds a skip" here for exactly one route generation; the skip was holding the button
down), plus the stale single-sample starter table and the never-raced LeafGreen. What
remains, largest first:

- **`09-battle-win`, 2409 frames.** The per-turn stage now repeats to a fixpoint, so
  adoptions do revisit earlier turns — measured across the day's 146 builds, a second pass
  found further cuts in 13 of them and one (a LeafGreen/Charmander re-run) kept adopting
  into a third, so the fixpoint loop is not paranoia. What still never moves:
  the start delay after stage 1 (a joint start × turn search remains untried), and *what* is
  pressed — move choice is untouched, and both crit rolls (live from Oak's "inflicting damage
  is key" line onwards, see the RNG section) and the 85-100% damage rolls are only reached
  through delays. The search keeps stumbling into good crit rolls because they make battles
  short; nothing yet aims for them, and nobody has re-run the crit census on the current
  battle.
- **`02-intro-oak`'s boxes still wait on scripted beats.** Text now prints at one character
  per held frame, but the intro is also fades, sprite slides and timer waits that no input
  reaches; 1565 frames is the floor for this drive shape, not for the scene. Nobody has
  audited which of those waits are input-gated versus timer-gated.
- **`text_hold` is one global knob.** The winning duty cycle is a compromise across every
  dialogue stretch in the route; per-segment (or per-box) hold values are strictly more
  general and completely unexplored.
- **The player name is one fixed letter.** Which letter (and which of the naming screen's
  cursor-start letters is cheapest to take) has never been compared; the name prints in a
  handful of boxes.

## Tier 2

**PASSED (2026-08-12, host): the 9658 LeafGreen movie, `route-9658f-269d169cd6db`.** BizHawk
2.11.1 replayed all 9658 frames to the ledger's fingerprint
(`08cf8de0a6a46f6df6bd322b4b51a80a3cbe93ba`) and the per-frame `gRngValue` probe matched on
every frame; result `$FRLG_ARTIFACTS/verify/results/route-9658f-269d169cd6db.json`
(`realtime`, 177s — watched, not headless). The committed route — LeafGreen, Bulbasaur,
`turn_hold` 4, `text_hold` 4, the fixpoint battle search — desynced nowhere against BizHawk,
and every segment's `tier2` field is stamped.

This was also the first tier-2 request that is not FireRed, which two changes made possible:
`frlg route export` writes the movie's own ROM identity into `Header.txt` (the `SHA1`
and `GameName` lines — everything else, `SyncSettings.json` above all, is still the
template's, byte-for-byte), and `tools/verify-runner.sh` picks the ROM whose sha1 the movie
header names out of `$FRLG_ARTIFACTS/rom` instead of playing everything on one configured
ROM. Both versions' ROMs are in that directory. The replay was the test of that rewrite, and
it held: BizHawk loaded the BPGE ROM the header named and stayed in sync for the whole movie,
so the header rewrite is proven format knowledge now, not tier-1-verified plumbing. Note the
`.bk2` container's sha1 is not reproducible across exports (zip metadata); the `ilog` digest
is the movie's identity, and the one that passed is `269d169cd6db…`.

**The FireRed predecessor passed (2026-08-12, host): `route-10085f-65ef20333a57`.** BizHawk
2.11.1 replayed all 10085 frames to the ledger's fingerprint
(`e65e93b6712b408ed915f55e46c9a79f874016cd`) and the per-frame `gRngValue` probe matched on
every frame — the then-current route (short names, FAST text, no battle animations, the
two-stage battle search, `turn_hold` 2) desynced nowhere against BizHawk. The result is
`$FRLG_ARTIFACTS/verify/results/route-10085f-65ef20333a57.json` (`realtime`, 172s — it was
watched, which is where the observations about the intro's text speed and the untried
starters came from). The same day's two earlier exports (`route-10946f-b1a0875a77e9`,
`route-10531f-e037421ddd87`, each superseded before any replay) were withdrawn from the queue
rather than left to burn a host run on a stale movie.

**The previous build passed (2026-08-11, host): `route-12713f-a4ad4280bbdc`.** BizHawk 2.11.1
replayed all 12713 frames of the pre-optimisation route, ended on the same EWRAM+IWRAM
fingerprint tier 1 computed (`73b329af5d561a864cc4b0d46e8d4c409ce1b6df`), and matched the
per-frame `gRngValue` probe on **every single frame** — not one divergence anywhere in the run,
which is a far stronger statement than the final hashes agreeing. The result is
`$FRLG_ARTIFACTS/verify/results/route-12713f-a4ad4280bbdc.json`. That closed the bedroom
desync; the 10085 pass above then confirmed the prediction that the rebuild — new inputs, same
boot, core and format — had nothing left to desync on. Everything below about the desync is
kept because the root cause is worth not rediscovering.

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
  copies every template entry verbatim except `Input Log.txt` and the two ROM-identity lines
  of `Header.txt` (since 2026-08-12, so one template serves both versions — see Status
  above), and **round-trips the result** —
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
green. The runner always writes a result, including for its own failures, and removes the queue
entry for every one of them: a request that died in a dialog must not look like one nobody has
picked up yet. The single exception is a replay stopped by a signal — ctrl-c, closing the
EmuHawk window, `kill` — which is not a verdict about the route: the result records
`"interrupted by signal N at frame …"`, the request stays in the queue, and the runner exits
rather than moving on, so running it again replays that same movie from the start. A crash is
not an interruption and is still scored and consumed. `bin/frlg-doctor` prints the newest
verdicts at startup.

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
