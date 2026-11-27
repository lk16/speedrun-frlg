# 2026-11-27 R2 continued: controller fixes on the way to Giovanni 1 (main session)

Started from `NEXT.md` (2026-11-24 20:15): the PP fix first, then 1841 from power-on to `giovanni1`.

## The "no PP" loop was a move-pick bug (commit 72063c9f)

`pp_left` in r2a-dev21's turn events stayed at 30 for BUBBLE across a whole forest, and the
`pewter-gym` state showed TACKLE 2 PP and BUBBLE 30/30, so the scene's PP was right and **the game never
used Bubble**. A new turn field `my_last_move` (`gLastMoves[0]`) confirmed it on a replay of
`tutorial -> pewter-gym`: every BUBBLE the policy chose was executed as TACKLE.

Cause: R2a round 2's Struggle fix let `use_move` return as soon as the action menu closed. After A on
FIGHT the controller first runs `PlayerBufferRunCommand`, then the move menu opens, so the function
returned early and the next mashed A chose slot 0. The Struggle shortcut now needs every move to be unusable
(0 PP or disabled; `AreAllMovesUnusable`, `decompiled/src/battle_util.c:405-424`), and a chosen move with
0 PP falls back to the best usable one. After the fix every executed move matches the choice.
R2b's runs used their own branch without the round-2 Struggle change, so they are not affected.

## What the fixed controller hit next (commit a36f3300)

- **Field poison faint.** A forest exp fight (`fight_until_level` 17) left the starter poisoned at 4/26.
  It fainted on the walk to Pewter: field poison faints in FRLG, `DoPoisonFieldEffect`
  (`hp == 0 || --hp == 0`, `decompiled/src/field_poison.c:93-115`). Nothing noticed, and Liam met Weedle.
  New walker guard (early stages only, `carry_faint_dead`): a poisoned carry gets the Antidote at the
  `early_torrent` window target (or at 3 HP, or a Potion at 2 HP when there is no Antidote). A field faint
  is a dead branch. Played: it cured at 9 HP in the forest (floor 17 = `entry_floor` + 1 at Lv7, above
  max/3).
- **The Antidote was spent in battle.** The policy's `Cure` used the forest Antidote on a Weedle's poison,
  so the plan's cure (`early_torrent.cure`) had nothing left. Battles before the `early_torrent` fight now
  hold back Antidotes.
- **Liam with Tackle only is a dead end.** With the moves now real, a Lv8 Squirtle's Tackle does 1-3 to
  Liam's Lv10 Geodude after Defense Curl, and the Geodude hits for 7-13. 12 of 12 branches blacked out
  from one checkpoint. Plan **1842-r2-notackle** = 1841 with `tackle_only` []. The sim never modelled the
  key, so the score is 1841's. **CURRENT -> 1842.** Seed 21, `pewter-gym -> brock`: Liam with Bubble in
  5 turns, Brock in 4, 0 rollbacks. The 20 r2a-dev21 Liam soft-locks and the "Bubble with 0 PP" note were
  both the move-pick bug.
- **Bedroom PC.** Seeds 11 and 31 soft-locked on every branch at the same frame. The Potion was withdrawn,
  but the loop waits until it has *seen* the top menu after the withdraw, and the B presses can pass that
  task between two samples. A 90-frame free run with the Potion in the bag now also ends it.
- **Dead branches keep their events**: `dead/<frame>-r<n>.events.jsonl` (they used to keep only a PNG
  and a state).

## Shadow Tag flee (commit 3294d266)

RUN against Shadow Tag (or Arena Trap on a grounded mon) fails at selection and returns to the action menu
without a turn passing (`decompiled/src/battle_main.c:3017-3037`), so the Six Island Wobbuffet flee looped
forever. `wild_turn` now attacks with the best move. **Not yet seen played.**

## Runs

`r2d-s11`, `r2d-s21`, `r2d-s31`: plan 1842, power-on to `giovanni1`, binary a36f3300 (no Shadow Tag fix;
not needed before Giovanni 1). Results below once they finish.

## More fixes on the way (commits c0d20b77, caa9a850, 2683b176, 097582b4)

- **No ball for the Cut hunt**: r2d-s31 died 12/12 at `nugget-bridge` ("hunt skipped, no ball in the bag").
  Cerulean (`persim-and-potions`) now tops up 3 Poké Balls when nobody in the party can learn Cut
  (`CeruleanCity_Mart/scripts.inc:24-25`).
- **Pewter Center fallback**: r2d-s31 met Liam at 4/28 HP, poisoned, with no item left (~25 dead branches).
  `preheal` heals at the Pewter Center when the bag cannot reach the floor. Not yet exercised.
- **Poison in every stage**: r2e-s31 entered Route 25 at 6/65 HP, poisoned (in-battle poison 1/8 a turn,
  `battle_util.c:805-806`). The carry died and Lass Haley beat the helpers, 34 dead branches. The walker
  guard now cures under max/4 everywhere, and Cerulean tops up 2 Antidotes.
- **Wrap**: RUN while wrapped is `BATTLE_RUN_FORBIDDEN` with no turn passing (`battle_main.c:3047-3051`), so
  the wrap never wears off. r2d-s21 pressed RUN against a Route 25 Bellsprout for 1.09M frames after its last
  ball missed. `wild_turn` now attacks when wrapped or escape-prevented, and a hunt stops with "no balls".

## Run of record to Giovanni 1: `runs/human-play/r2g-s31` (plan 1842, seed 31)

Power-on prefix from `r2d-s31` (binary a36f3300) to `mega-kick`, then binary 2683b176. **`play-verify`:
32 checkpoints match from boot. Bumps: 0.** Every trainer fought is a plan fight. Dan and Chad (Route 25)
were waited out until they faced away, and no hideout grunt outside the plan was fought. Grunt 4 / B1F are
covered by the same rule (only `hideout-grunts` plan fights appear).

| split | r2g-s31 (s) | model p50 (s) | r2b-full3b (s) |
| --- | --- | --- | --- |
| Brock | 1087 | 965 | (r2a-s11 978) |
| Mt. Moon PC | 1348 | 1155 | (r2a-s11 1366) |
| Misty | 1882 | 1691 | 1964 |
| Bill | 2699 | 2277 | 2791 |
| Bike | 3165 | 2752 | 3319 |
| Rock Tunnel | 4045 | 3307 | 4080 |
| Giovanni 1 | 4726 | 3845 | 4809 |

The gap to the model grows most in Bill (+230 s, Route 25 hunt and fights) and Rock Tunnel (+325 s from the
Bike split: ten trainers, walked without Flash). Those are the next targets for the controller and the model.

## Plan 1952 played (CURRENT -> 1952-r3d-cut-route5)

`runs/human-play/r2j-s31`: the r2g-s31 prefix to `brock`, then plan 1952 (R3d honest Cut). `play-verify`
**32 checkpoints match from boot, 0 bumps.** Pewter bought 2 Poké Balls before the 9 Potions. The Route 5
hunt after `tm28-dig` caught a **Meowth in 9 steps, 1 encounter, 2418 frames** (616 of them walking there),
and HM01 went to Meowth at `bike-redeem`. No Route 25 hunt. Giovanni 1 at **4720 s** (model p50 3847).
Also verified to Giovanni 1: `r2i-s11` (1842, 4900 s, 0 bumps, 4 rollbacks).

Other fixes: `talk_to` now matches the live object's map as well (a connected map's objects are loaded in
the current map's frame: Pallet's object 1 was taken for the Route 1 clerk at (3, 48), r2h-s41 fatal). The
Vermilion trash cans credit a switch opened by the walker's own settle-A (the cans were 15/10 from
VAR_TEMP_0/1, and the first switch was found on arrival and then "reset" by our press).

## Post-game on the current controller (`runs/human-play/post-v4-icefall`)

The v4 run (plan 1122) resumed from `icefall` (Six Island) with today's controller (binary 240cd9d7) played on to
`hall-of-fame-2` (39 rollbacks, mostly the rematch Champion). So the post-League stages still work with the R1
walker and today's battle fixes. The Shadow Tag / Wrap path was not triggered on this route.
