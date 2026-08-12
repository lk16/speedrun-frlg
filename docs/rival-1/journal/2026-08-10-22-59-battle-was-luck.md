# 2026-08-10 -- the battle was luck, and the obvious trim made things worse

**The win was a coin flip.** Delaying the same A mash into the rival battle by one frame flipped it
from a win to a loss, alternating over twelve consecutive delays. So `08-battle-win` now searches:
16 start delays, keep the shortest that wins, print how many did. Same 11873 frames as before -- the
route did not get faster, it got *chosen* instead of lucky, which is what stops it losing the next
time something upstream moves.

**Criticals are off in this battle.** `gBattleTypeFlags` is `0x1C`, which includes
`BATTLE_TYPE_FIRST_BATTLE`, and the crit check is gated on that flag being clear (or on the tutorial
having spoken): `decompiled/src/battle_script_commands.c:1199`. So the spread is damage variance
(85-100%, `:1558`) and accuracy (`:1093`) only. An earlier guess in this journal that a critical was
what made Squirtle's battle short was wrong; it cannot have been.

**Local trims are not free.** `06-starter` held UP for 8 frames to face the ball; 1 is enough.
Trimming it saved 6 frames and cost 391 in the battle, because the shifted `gRngValue` produced a
battle needing two more attacks. Net 385 slower. That is now the reason `Tuning` exists: knobs like
this are route-level, recorded in the ledger, and swept end-to-end by `frlg route tune`, which scores
each variant on total frames to the win rather than on the segment it lives in.

Swept all eight values end-to-end afterwards: 8 (untrimmed) is the best at 11873, and every trim is
163 to 958 frames worse, with no monotonicity at all. So the route is unchanged in frames and much
better understood.

**Carry this forward.** Every future optimisation upstream of a fight has to be measured through the
fight. Anything that reports "saved N frames" without re-running the battle is not evidence.
