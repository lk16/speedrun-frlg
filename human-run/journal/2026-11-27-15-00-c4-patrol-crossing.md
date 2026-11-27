# 2026-11-27 C4: crossing Mt. Moon's patrolling Lass and Grunt 4 without being seen

Code: `crates/frlg-human/src/play/walk.rs` (`wait_away`, `predict_sequence`, `crossing_clear`, `in_turn_box`),
`ram.rs` (`LiveObject` gains `ix`, `iy`, `seq_idx`). Scratch outputs `c4-*` in the session scratchpad.
`FRLG_C4_TRACE=1` prints the player and every trainer object per frame of a wait or step (debug).

## Two different causes

**Miriam (Mt. Moon 1F, `WALK_SEQUENCE_LEFT_DOWN_RIGHT_UP`, spawn (33, 4), range 5x3, sight 2).** Her loop is
deterministic: `MoveNextDirectionInSequence` walks LEFT until the next tile is outside the range box, then DOWN,
RIGHT, and the per-variant `_Step1` skips to UP when she is back on her initial column
(`decompiled/src/event_object_movement.c:3909-3935`, `:4183-4192`). So the loop is 5 left, 3 down, 5 right, 3 up =
16 steps x 16 frames = 256 frames (trace: her coordinates change every 16 frames). She faces down on (28, 7) for one
step only (the next down step is outside her range, so she turns right on the spot), and that is the only moment
she sees row 9: (28, 8) and (28, 9) (`GetTrainerApproachDistance`, `decompiled/src/trainer_see.c:122-143`, sight 2).
The old `wait_away` waited for one visible turn *away from the player's tiles* and then crossed. Her turn to face
down at (28, 4) -> (28, 5) looked "away" (that line reaches (28, 7) only), the human reacted for 22 frames and ran
onto (28, 9) just as she stepped onto (28, 7) (trace r2e-s11, frame 92202: player (28, 9), Miriam (28, 7) facing
south). So the failure is timing against a *future* pose, not the sight geometry.

**Grunt 4 (B2F (37, 21), `LOOK_AROUND`, sight 3).** Different: the fight was not luck at all. `LOOK_AROUND` and the
`FACE_*_AND_*` types test `ObjectEventIsTrainerAndCloseToPlayer` while they wait out their delay: with the dash flag
set (B held) and the player's *destination* within `trainerRange` tiles on both axes, the delay is skipped and
the trainer turns toward the player along the longer axis (`event_object_movement.c:2772-2799`, `:2992-3015`,
`:3044-3064`). The walker already had `Blk::norun` for this (walk, do not run, in the box), but it is drawn when the
leg is *planned*, and Grunt 4 is not on screen then (first live at frame 100050; the plan was made at 100007). The
leg ran into the box and she turned north at 100435, 32 frames (her minimum dwell, `gMovementDelaysMedium`
{32, 64, 96, 128}) after the turn it had waited for. Not random.

## Method

1. `in_turn_box` (live): before every step the walker checks the live objects; when the destination tile is inside the
   `trainerRange` box of an unbeaten LOOK_AROUND / FACE_*_AND_* trainer, the step is a walk (no B). The planned
   `norun` stays as well.
2. `wait_away` for a walk-sequence trainer: the human watches until it has seen her coordinates change (this gives
   the step phase: a step starts on the frame the coordinates change), then each frame the walker predicts her
   poses from RAM (`predict_sequence`: position, facing, `directionSequenceIndex`, `initialCoords`) and asks
   `crossing_clear`: for every start `d` in `react_lo ..= react_hi + jitter_n + 1` (the reaction and the input
   lateness the human will have), the player stands on tile `j` from its step start to the next
   (`PlayerGetDestCoords`, `trainer_see.c:127`; 9 frames a tile running, 17 walking) and no predicted pose
   in that interval sees the tile. When true the human reacts (`U{15..30}`) and goes. Waiting frames are logged by the
   existing "waited N frames" decision event.
   Other turners keep the old rule (wait for a visible turn away, react, go) plus the live no-run box.

A unit test (`patrol_tests::miriam_loop_and_crossing_window`) checks the loop positions (28,4) (28,7) (33,7) (33,4)
and that (28, 9) is seen exactly once per 16 steps.

## Results (replay `--from mt-moon-pc --until miguel`, plan 1964-r3e-honest)

Baseline r2e-s11 seed 11: Miriam fought (92012 ..), Grunt 4 fought, checkpoint miguel 112230. Now:

| case | Miriam | wait (f) | Grunt 4 | wait (f) | bumps | miguel frame |
| --- | --- | --- | --- | --- | --- | --- |
| r2e-s11, seed 11 | no | 70 | no | 121 | 0 | 104275 (-7955 f, -133 s) |
| r2d-s11, seed 11 | no | 70 | no | 121 | 0 | 104275 |
| r2g-s31, seed 31 | no | 59 | no | 23 | 0 | 101576 |
| r2j-s31, seed 31 | no | 59 | no | 87 | 0 | 103531 |
| r2e-s11, seed 5 | no | 63 | no | 126 | 0 | 104476 |
| r2e-s11, seed 7 | no | 58 | no | 27 | 0 | 103316 |
| r2e-s11, seed 13 | no | 61 | no | 183 | 0 | 101269 |
| r2g-s31, seed 2 | no | 62 / 64 | no | 23 / 153 | 0 | 100525 (1 rollback, Miguel blackout) |
| r2j-s31, seed 9 | no | 61 / 65 | no | 20 / 30 | 0 | 102053 (1 rollback, Grunt 1 blackout) |

(`/ ` = first branch / after the rollback.) Miriam's wait is the ~60 frames of watching one step plus the
reaction; the loop never has to be waited out because the window (16 frames of danger per 256) is short.

Regression: `frlg-human play-walk --plan 1964-r3e-honest.md human-run/walk-regress.json`: 0 failed, 0 bumps.
`cargo fmt`, `cargo build`, `cargo test --release -p frlg-human` clean (57 passed).

## Open

- Grunt 1 on B2F ((12, 20), FACE_DOWN, sight 2) is still fought in every run ("no way around the sight line"):
  reason not investigated (a guess: its cone covers the only way). Not a patroller, not touched here.
- A random turner that turns *toward* the crossing tile during the crossing is still possible: after a visible
  turn the facing is guaranteed 32 frames, the react (15-30 f) plus a 17-frame walked tile can run past it
  (about 1/16 of crossings by the delay table). The 9 replays above had none.
- A walker that is blocked (walks in place) never changes coordinates, so the phase is unknown and
  `wait_away` runs out its 1800-frame cap ("crossing anyway" warn). Not seen.
