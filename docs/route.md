# The route: power-on to a beaten rival

11873 frames (~3m18s at 59.7275 Hz) from reset to `gBattleOutcome == B_OUTCOME_WON`, with Squirtle.
Tier 1 only: mGBA agrees. BizHawk has not seen it, and cannot in this sandbox.

    frlg route build       # run the segments, write route/logs/*.ilog and route/ledger.json
    frlg route verify      # replay the committed logs from reset and check every claim
    frlg route status      # print the ledger

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

## What is not optimised

Nothing yet. This route exists to be beaten by the next one, and the frame counts above are the
baseline it has to beat. The obvious targets, largest first:

- **`08-battle-win`, 3461 frames.** Mashing A picks FIGHT and the first move and eats every message.
  Both mons are level 5 with no type-effective moves (Tackle/Scratch and a stat-drop), so the length
  of the battle is damage rolls and criticals -- i.e. RNG manipulation, and the first place where
  `gRngValue` matters.
- **`06-starter`, 2496 frames.** A mash with no thought about text speed. The OPTIONS menu's text
  speed is not set (the route never opens it), and whether setting it pays for itself is measurable.
- **`02`/`03`, 3282 frames together.** Mashing A on the naming screen types whatever letter the
  cursor starts on until the name fills up. Picking a preset name from the list is the obvious
  alternative and has not been measured.
- **Starter choice.** Squirtle is what the current ledger uses. Which of the three wins fastest is a
  measurement -- three builds, three frame counts -- not a matter of opinion, and it has not been
  made yet.

## Tier 2

Blocked, and the ledger says so on every entry. A `.bk2` needs the Input Log column order and the
`SyncSettings` block, neither of which is derivable from anything mounted here; both come from
`route/template.bk2`, a movie recorded on the host, which does not exist yet (`docs/harness.md`).
Until it does, the `.ilog` files are the artifact and no `.bk2` is written.
