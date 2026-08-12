# 2026-08-12 (sandbox, late night) -- frlg-battle: the battle's roll semantics in pure Rust, and the accuracy check the player never rolls

The seed dial came up dry, so per plan the next distillation step got built:
`crates/frlg-battle` models which `Random()` calls the rival battle consumes, in what
order, and what the damage arithmetic does with them -- every formula cited
(`CalculateBaseDamage`'s physical branch `decompiled/src/pokemon.c:2509-2558,2648`,
`gStatStageRatios` `:1442-1457`, the 85-100% variance `battle_script_commands.c:1557-1568`,
the AI's rolls `battle_ai_script_commands.c:310,408`, `data/battle_ai_scripts.s:1129-1150`).
**Validated the only way that counts**: `tests/committed_battle.rs` replays the committed
battle on libmgba, extracts its logic-roll stream (38 rolls beyond the 2-per-frame VBlank
pair), and drives the model over exactly that list -- it must consume every roll, predict
every HP change (5, 4, crit 9, 4, 4), the AI's move choices, and the win. It does.

Three things the validation forced into the open, all now measured facts:

- **Within a busy frame, the VBlank pair leads and the game's rolls trail.** The first
  reconstruction assumed the opposite and the damage arithmetic refused to match; with
  the pair leading, every crit and damage value validates (e.g. turn 2's crit roll is
  61552, ≡ 0 mod 16, and its variance roll gives exactly the observed 9).
- **The player never rolls accuracy in this battle -- Tackle's 95% cannot miss.**
  `Cmd_accuracycheck` evaluates its FIRST_BATTLE skip on the raw script argument *before*
  the `ACC_CURR_MOVE -> gCurrentMove` substitution
  (`battle_script_commands.c:1005-1018` vs `:1035`), and `ACC_CURR_MOVE` is 0
  (`include/constants/battle_script_commands.h:67`) = `MOVE_NONE`, whose power is 0
  (`src/data/battle_moves.h:3-8`). So the `power != 0` disjunct is dead code and the
  `power == 0` disjunct -- gated on `FIRST_BATTLE_MSG_FLAG_STAT_CHG`, which only the
  *player's own Growl* sets (`battle_controller_oak_old_man.c:1769-1771`) -- covers every
  player move for the whole battle on this route. Verified in the emulator: turn 2's
  Tackle crits (proving INFLICT_DMG set) while consuming no accuracy roll. The rival
  rolls accuracy on every move. This corrects `docs/rival-1/route.md`, which described the
  accuracy skip as ending with the INFLICT_DMG flag.
- **The INFLICT_DMG flag sets at battle frame 1185 on the committed battle** -- when our
  first hit's HP bar finishes draining, *before* `gBattleMons.hp` updates at 1313,
  because Oak's interjection sits between (`simulatedInputState[2]`, offset 0x94 of
  `BattleStruct`, computed with the decomp's own headers under
  `gcc -m32 -ffreestanding` and watched flipping in RAM by the `flag-probe` example).
  Crits are live for both sides from frame 1185 of this battle, not "from turn 2".

Also in this entry's window: the per-turn roll budget for this matchup is now exact --
turn start 1 (`gRandomTurnNumber`, `battle_main.c:2999`), AI 6 (4 simulatedRNG + the
Growl-viability roll + the unconditional tie-break), our Tackle 3 (crit, damage,
secondary -- no accuracy, above), rival Scratch 4 (accuracy, crit, damage, secondary),
rival Growl 1 (accuracy only), +1 per speed-tie site when speeds tie (ours don't: 11 vs
9, we act first -- measured, correcting the session's earlier guess that the rival is
faster).

**What v1 is not:** a battle *predictor*. Frame pacing (text, HP bars) still decides
where in the stream each logic roll lands, and the emulator still owns that. v2 -- a
pacing model per action type, validated the same way -- is what would let a search
enumerate candidate battles in microseconds and only emulate the survivors.

**Unverified:** nothing in this entry; every claim above is either cited, measured, or
both.
