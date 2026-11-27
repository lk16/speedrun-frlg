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
