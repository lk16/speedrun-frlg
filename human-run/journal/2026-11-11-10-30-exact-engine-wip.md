# 2026-11-11: mechanics-exact battle engine (WIP — crate does not compile yet)

Goal of this session (user request): replace the model's estimations with hard numbers —
emulate the game's RNG and battles in code, advance the RNG for walking/NPC time, then
re-test the WR run's techniques (Squirtle-only, FireRed, SpA gate, Torrent, no fishing,
Abra teleport, escape ropes, Mewtwo).

## Done and committed previously (65373b3)

gen_dex.py + data.rs now carry: trainer `aiFlags` (1 bad-move, 2 viability, 4 try-to-faint;
`include/constants/battle_ai.h:37-39`) and `.items` (E4 Full Restores), move `pp`/`chance`/
`effect`, species abilities, TM/HM + tutor learnsets, `items.json` prices. Verified:
Squirtle line learns TM28 Dig / TM13 Ice Beam / TM14 Blizzard / TM31 Brick Break (Blastoise
+ TM26 Earthquake); Mega Punch/Kick tutors are free on Route 4
(`data/maps/Route4/map.json:96,110`).

## Written this session (THIS COMMIT — engine done, integration half-done, DOES NOT BUILD)

* `crates/frlg-human/src/rng.rs`: `GameRng` — the real LCG via `frlg-rng` (emulator-verified
  crate), `frames(n)` = per-frame VBlank advance (`src/main.c:412`), rolls with the game's
  16-bit arithmetic. Re-keyed per segment for CRN pairing (justified in the doc comment:
  random 16-bit timer seed + human jitter).
* `crates/frlg-human/src/battle.rs`: full rewrite. Exact damage pipeline (badge boosts incl.
  the previously missing Thunder-Badge ×1.1 SPEED boost in `GetWhoStrikesFirst`,
  `battle_main.c:3444`), stat stages (`gStatStageRatios`, `pokemon.c:1442`), accuracy stages
  (`battle_script_commands.c:570`), crits {16,8,4,3,2}, status (sleep 2-5/freeze 20% thaw/
  para 1-in-4/confusion self-hit/toxic), ~60 move effects, X items (+1 stage,
  `item_effects.h:241`), enemy Full Restores under 1/4 HP (`ShouldUseItem`), and the AI
  ported from `data/battle_ai_scripts.s`: CheckBadMove vetoes, TryToFaint (+4/+6 kill, -1
  not-most-powerful with the AI's own rolled damage variance), CheckViability handlers
  (paralyze/confuse/toxic/evasion/acc-down/heal/rest/high-crit/explosion/recharge/counter/
  protect/trap/always-hit; sleep deliberately no-op — FRLG checks the TARGET's moves).
  Score ties break by `Random() % n` — this is why real AI mixes status with attacks.
  Has tests incl. "Dig + X Defend beats Surge at Wartortle 26" (unverified — never ran).
* `params.rs`: new plan knobs `tms` (Teach), `xbuys` (XBuy), `fight_tactics` (per-segment X
  items + heal threshold), `heal_below`, `keep_moves`, `cure_status`.
* `route.rs`: `Kind::Teach`/`Kind::XBuy` + insertion; `Kind::Trainer` gains `ai`,`heals`;
  `Kind::Parties` gains `ais`.
* `sim.rs` (HALF-DONE): State gains `game: GameRng`, `slots` (4 move slots), `learned_to`,
  `carry_status`, `x_stock`, `mewtwo`; `bump` advances the game stream; moveset management
  (`slot_utility`/`offer_move`/`learn_moves`, keep-list aware) hooked into `add_exp`;
  `shakes_catch`/`throw_balls` moved to GameRng with the exact status bonus
  (`battle_script_commands.c:9553-9556`).

## To finish (in order) — next session starts here

1. `sim.rs`: fix `slot_utility` dead condition (`m.power == 0` twice); init the new State
   fields in `attempt()` (slots = starter L5 moves, learned_to = 5, game re-keyed per
   segment from `rng.next_u64() as u32`); resolve `keep_ids`/`tactics` in `eval.rs::ctx()`.
2. Rewrite `sim_battle()`: build the carry via `battle::make_mon(..., badges)` (add
   bit 2 on `surge-badge`!), moves = `st.slots` (Mewtwo: its rolled IVs + default L70 moves
   `Swift/Recover/Safeguard/Psychic` + optional `tms` with `mewtwo: true`), tactics from
   `ctx.tactics` by segment name limited by `st.x_stock`, `fight(...)` with `st.game`
   (copy out/in, GameRng is Copy), charge heal items by tier price under money_model,
   persist `out.end_status` into `st.carry_status`, cure after battle (Antidote 100/
   Paralyze Heal 200/Awakening 250/Burn Heal 250/Ice Heal 250 + ~400f, `items.json`).
3. Wild battle_sim foes: real IVs (two 16-bit rolls, 5-bit fields; `pokemon.c:1836-1853`)
   and nature = pid % 25 (`Random32()`, `pokemon.c` CreateBoxMon); wild Abra Teleport-flees
   (engine handles it — pass `wild: true`).
4. `attempt()`: match arms for `Kind::Teach` (teach carry or Mewtwo, pay cost, frames) and
   `Kind::XBuy` (stock + money + frames); fix `Kind::Trainer`/`Parties` patterns for the
   new fields and pass `ai`/`heals` through; `throw_balls` call sites gain
   `Status1::None`; static catches with status via future tactic.
5. `main.rs`: update the `battle` command to the new `fight` signature (Tactics, ai flags,
   GameRng); then `cargo test` — the four battle.rs tests gate the engine (lab rival ~5
   turns, Dig+X-Defend vs Surge >60%).
6. Re-run `battle-analysis.md` tables with TMs/X items; set `battle_sim: true` as baseline;
   re-tune plan (expect: no more `team_bonus`/`type_floor`); descend.
7. Then the WR-technique variants (user's list): Squirtle-only (default), FR vs LG wild
   tables, starter SpA gates (NB: L5 SpA ≥ 30 is impossible — base 50 gives ≤ 12 at L5;
   interpret as a check at a later level, e.g. Wartortle L16 SpA ~27-31, and test gates),
   Torrent-range tactics (heal_below < 1/3 keeps Torrent up — engine models the ×1.5),
   no fishing (drop fish fills + rod), Abra teleport (already on), escape ropes (new
   `rope_or_walk` segment kind or a Teach-like event; Escape Rope 550, `items.json`),
   Mewtwo for the final fights (already on; now with real stats incl. Recover).

Everything cited above was read in `decompiled/` this session; the AI viability port
trims the handlers to the statuses the model tracks and is the main labelled
approximation, alongside: no PP, no enemy switching, gender-less Attract, no-op
Future Sight/Roar/Teleport-in-trainer-battles, weather only as damage/acc modifiers.
