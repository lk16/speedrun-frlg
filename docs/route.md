# The route: power-on to a beaten rival

11873 frames (~3m18s at 59.7275 Hz) from reset to `gBattleOutcome == B_OUTCOME_WON`, with Squirtle.
Tier 1 only: mGBA agrees. BizHawk has not seen it, and cannot in this sandbox.

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
| `08-battle-win` | 3461 | 11873 | `gBattleOutcome == B_OUTCOME_WON` |

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

All three win under the same mash. Built end-to-end, one build each:

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
scores it on total frames to the win. All eight values, each a complete build:

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
  cursor starts on until the name fills up. Picking a preset name is the obvious alternative and is
  still unmeasured -- and, per the finding above, it has to be measured through the battle.
- **`06-starter`, 2496 frames.** Text speed is never set; the route does not open OPTIONS. Whether
  the detour pays for itself over ~40 message boxes is arithmetic nobody has done here.
- **Starter choice.** Measured once each, mashed, not manipulated. Redo it against manipulated
  battles before treating Squirtle as settled.

## Tier 2

Blocked, and the ledger says so on every entry. A `.bk2` needs the Input Log column order and the
`SyncSettings` block, neither of which is derivable from anything mounted here; both come from
`route/template.bk2`, a movie recorded on the host, which does not exist yet (`docs/harness.md`).
Until it does, the `.ilog` files are the artifact and no `.bk2` is written.
