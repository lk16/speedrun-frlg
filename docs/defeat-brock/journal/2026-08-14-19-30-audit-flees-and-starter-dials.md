# 2026-08-14 19:30 — the run audited frame by frame; the flees, the steps, the starter

Task (user): optimize both TASes further, with four specific suspicions --
unnecessary steps after encounters (a bug?), encounters at all (avoid them,
walk less grass), a starter without max Atk/SpA nature, and whether
L7-for-Bubble before Sammy is worth setting up.

## The audit tool, and what the committed 38862 actually does

`crates/frlg-route/examples/audit-run.rs` replays a committed ledger from
reset and reports battle episodes (with both sides' `gBattleMons` stats and
every HP write), reversal steps, free-idle runs, and wild rate tests per
segment. On `route/defeat-brock/ledger.json`:

- **8 battle episodes**: rival 2936, four wild flees (Pidgey/Rattata/Rattata/
  Caterpie, 501-506 frames in-battle each, ~2.0k total), the scripted
  catching-demo Weedle 1943, Sammy 2867, Brock 4249.
- **Zero reversal steps, zero free-idle runs ≥ 8 frames** anywhere in 38862
  frames. The "unnecessary steps directly after encounters" a viewer sees
  are not a bug: a battle resets the encounter cooldown
  (`decompiled/src/battle_setup.c:205`), so the planner deliberately routes
  the next 6-7 grass steps *through* grass where they are nearly free -- it
  looks like wandering and is optimal. rival-1 audits equally clean.
- Wild rate tests consumed: to-viridian 8, deliver 13, tutorial 10,
  forest 31, to-pewter 1 (plus the boot seeding). 4 of ~63 passed = the 4
  flees; all four were second-LCG fated passes the planner priced as
  cheaper to take than to dodge (`ENCOUNTER_COST` 1400 vs measured ~575 --
  the model *over*-prices flees, so these four were genuinely index-walled).

## "Avoid all encounters" is arithmetic, and the arithmetic says no

The wild rate test runs on its own LCG, step-indexed, seeded once at the
title exit (`research/wild-encounters.md`). The full 0-255 `seed_delay`
scan (`frlg route scan --to-seed 256`, new; 0-63 was the old coverage):
**no seed models below 3 flees.** Best: seed 184 (walk 29643, 3 flees),
seed 148 (29896, 3). The committed seed 27 models 4 (matches the 4
measured). P(a 60-test stretch of a 21%/14%-rate stream contains zero
passes) ~ 1e-6, and no repel is purchasable before the forest --
Viridian Mart sells Poke Ball/Potion/Antidote/Paralyze Heal only
(`decompiled/data/maps/ViridianCity_Mart/scripts.inc:65-70`), and while
the parcel is held the clerk does not shop at all (`:55-56`). So zero
encounters is out of reach; 3 is the new target. Scan artifact worth
knowing: the `route2-south` crossing models as unreachable (u32::MAX) on
every seed -- constant across seeds so the ranking stands, but the
crossing shape in `scan.rs` needs fixing before its absolute costs are
trusted.

## The starter question, measured instead of assumed

New tools: `starter-genome.rs` (dump the committed starter),
`ball-scan.rs` (run `07-starter` once per `Tuning::ball_delay` -- new
knob, `--ball-delay` -- and read the created genome, IVs decrypted from
the box substruct, `decompiled/src/pokemon.c:2863-2896`). The committed
starter is **Docile, IVs 31/10/5/22/3/1** (hp/atk/def/spe/spa/spd) --
neutral nature, low Atk/SpA IVs, lucky HP IV 31.

But the audit's HP-event trace says the *fights* barely care:

- **Brock: both mons die in one crit each.** Geodude 31 HP one-shot
  (crit Bubble 30-36), Onix 33 HP one-shot. The delay search (delay 164
  of 384) already fished the double-one-shot out of the stream. No SpA
  the ball can produce beats that -- Modest/Rash + SpA IV >= 29 would give
  spa 15 at L7 (Bubble 20-24 vs Geodude, a guaranteed 2-hit), strictly
  worse than the crit 1-hit the route already has. **Torrent (<=1/3 HP,
  x1.5 water power, `src/pokemon.c:2500`) is closed by the same fact.**
- **Sammy: 4 Tackles, two of them crits** (4+4+9+9 vs Weedle 26 HP,
  def 10). Reaching L7/Bubble before Sammy needs ~68 exp with no source
  cheaper than a ~700-frame wild fight, to upgrade normal hits by +1.
  Closed.
- **The rival fight is the one live breakpoint.** Rival Bulbasaur: 19 HP,
  def 9 (measured). Committed profile 4 + 10(crit) + 5 = three turns. At
  atk >= 11 (Atk IV >= 24 neutral, or Adamant IV >= 20) the ungrowled
  Tackle goes base 5 -> 7 post-STAB and the crit ceiling 14, so
  crit + growled-hit can reach 19-20: **a 2-turn kill becomes possible**
  (~700 frames if a stream cooperates), and growled hits go 3-4 -> 5-6.
  Gen-3 damage floors (`(2L/5+2)*P*A/D/50 + 2`, stage -1 = x2/3, STAB
  15/10, type x4 after) put every other reachable stat change between
  breakpoints: Sammy damage is atk-invariant at 11-13, Onix Bubble is
  spa-invariant at 10-15.

Ball-scan 0-64 on the committed prefix: delay 16 = Adamant atk-IV 20
(L5 atk 11), delay 44 = Adamant atk-IV 30 (L5 atk 12), delay 27 =
Modest spa-IV 30 (the spa-15 shape, kept as a control).

## Wave 1 (in flight)

Five parallel builds from the committed prefix: ball_delay {16, 27, 44}
on seed 27 (via `--from 07-starter`; the `--from` tuning guard now
compares field-wise, since turn_hold/ball_delay cannot affect segments
before `07-starter`), and full builds of seeds 184 and 148 (the 3-flee
seeds; their extra seed_delay costs ~130-160 boot frames against one
~575-frame flee). Results below when they land.

## Results — wave 1 (all tier-1 builds, scored on total frames; committed = 38862)

| build | knobs | total | delta |
| --- | --- | ---: | ---: |
| ball27 | seed 27, ball 27 (Modest, SpA IV 30) | 39944 | +1082 |
| ball16 | seed 27, ball 16 (Adamant, Atk IV 20) | 40157 | +1295 |
| ball44 | seed 27, ball 44 (Adamant, Atk IV 30) | (below) | |
| seed148 | seed 148, ball 0 | 41482 | +2620 |
| seed184 | seed 184, ball 0 | 41742 | +2880 |

Two independent lessons, both worth keeping:

1. **The scan is a screener, not a predictor.** The seed-184 build *realized
   7 flees* (audited: 1 to-viridian, 1 deliver, 2 tutorial, 3 forest)
   against the model's 3 — the per-leg executor replans from real
   `sWildEncounterData` and its consumed-test indices drift from the scan's
   idealized crossings, so the scan's flee count carries ±4 of error. The
   corrected-crossing rescan (route2-south fixed) still ranks 184/148/27
   at the top, so the *ranking* survives; the absolute counts do not.
2. **The committed 38862 is a dial-tuned local optimum that raw rebuilds
   cannot reach.** Its rival battle carries the solver seed (2445; plain
   search on the same stream cannot win at all in 256 delays), and its
   forest/gym arrival streams carry three adopted head-wait dials. Any
   new seed or ball delay resets all of that: the wave's rival fights
   came out 2549-2935, and every build paid +600..+1500 more across
   Sammy/Brock stream luck. The Adamant genomes did produce the faster
   raw rival fights the breakpoint math predicted (2549 and 2680 vs
   ~2613-class for plain search on the committed genome) — the mechanism
   is real, its ~150-frame value is just an order of magnitude below the
   ~1000-frame cost of abandoning the tuned streams.

## The pre-idle sweep: both committed fights survive a 5x wider search

`FRLG_PRE_SWEEP="P[,D]"` (new, committed) widens `win_battle`'s stage 1
with in-battle pre-idle families 1..=P — the rival solver's `plan[0]`
dial, unreachable by segment-head waits (the overworld press grid
quantizes those to one family) because every in-battle frame moves the
stream by 2 (`decompiled/src/battle_main.c:1650`). Off by default; a
no-env `--from brock` rebuild reproduces the committed segment digest
exactly.

- **Brock probe** (`--from brock`, pre 1..24 x 64 delays + the full 384):
  13/1920 candidates win; the best is *still* the committed plan [164]
  at 3305. The crit double-one-shot survives 24 fresh families.
- **Sammy probe** (`--from forest`, same env): 36/1728 win; the best is
  still the committed plan [13] at 2603. Its downstream rebuild lacked
  the committed `FRLG_WAIT_TO_GYM=1` head-wait, arrived at Brock one
  frame earlier, and the best that arrival offered — across its own 24
  pre families — was 3580 (+275): an independent re-demonstration of
  why that 1-frame dial was adopted. Total 39136, discarded.

**Adoption: none. The committed 38862 stands.** The starter-stat and
seed dials are now *measured shut*, not assumed shut: to beat 38862 via
either, a candidate would need its own solver pass plus wait-dial
defense (~a session of compute per candidate) just to claw back the
tuning it forfeits, before its structural advantage (one fewer flee ~ 575,
or a faster rival family ~ 150) could show. The remaining real headroom
is unchanged: the Sammy/Brock fight model (backlog #1, measured swing
3157..3813 across families).
