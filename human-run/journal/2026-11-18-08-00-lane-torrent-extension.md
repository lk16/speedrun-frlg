# 2026-11-18: Torrent credit, Brock tactic, the lane past rival-ss-anne, descent 947-959

Rows marked MODEL CHANGE / SENSITIVITY / DIAGNOSTIC in `scores.jsonl` are model statements. Plans: 940-lane (model change), 946-brock0, 959-manip156 (CURRENT).

## 1. Why the runner is at ~1/3 HP at Brock, and what heal-before-Brock is worth

Decomp: Torrent is only `gBattleMovePower *= 1.5` for Water moves at HP <= max/3 (`src/pokemon.c:2500`). Brock = Geodude L12 (Tackle, Defense Curl) + Onix L14 (Tackle, Bind, Rock Tomb 50 bp 80% speed-down), IV 0
(`src/data/trainer_parties.h` sParty_LeaderBrock). Squirtle: Bubble L7, Withdraw L10, Water Gun L13 (`src/data/pokemon/level_up_learnsets.h:101`). Both foes are Rock/Ground (Water x4).
Engine table (`frlg-human battle`, Torrent tactic, Rash-ish IV 20, no potions): L11 Bubble: 10/32 HP 38%, 50% HP 16%, 70% 78%, full 100%; L12: 49 / 26 / 84 / 100; **L13+ Water Gun: 2 turns, 100% at any HP**
(Water Gun needs no Torrent); with one potion in the fight the 1/3 HP entries are 97-99%. Entry level in the model is L10 (34%) / L11 (66%), never 12+. So the runner's 1/3 HP entry is a gamble at L11-12
(38-49%) and free at L13+ ("entry level 14 preferred", STRATEGY s4): the Torrent benefit is one fewer Onix hit and a skipped potion, plus Torrent downstream. Withdraw: the engine used it only against a slower
foe; Onix is faster (speed 24 vs 16), so I added `torrent_early` (Withdraw first when the stage buys a hit and the foe needs <= 2). It lowers the 10/32 HP win .39 -> .23 and is n.s. in the plan: refuted, default off.
(The stream "Bide" is Bind in the decomp.)

Torrent credit (all WR+2m, R = 800k, seeds 2 and 3):
* `risk_adopt` (new): a lane heal is real for the timing lane too (it leaves the fight at the lane's HP). Pass windows unchanged; EHT +3..+7 h.
* Brock potions 0 -> 5 stays **-21..-26 h** in both the flat-fatal and the lane model: the edge is not an artefact of the lane copy.
* `risk_torrent_lost` (a heal above 1/3 HP pauses Center skips and the Torrent tactic until damage re-enters): the edge shrinks to -1.3..-2.4 h, but the switch moves the baseline +-15-28 h in opposite directions in the two models and
  its Brock window pass (.755) misses the runner's .40: not used.
* Diagnostic `ablate_torrent` (timing lane without Torrent): +9.2 h [2.7, 15.9]. **The model's whole downstream Torrent is worth ~9 h ~ 1 min of run time (EHT 152.6 / 136.1 / 112.5 h at WR+1m/+2m/+5m, i.e. 8-16 h per minute);
  the heal edge is ~2.7 min.** To justify never healing, Torrent would have to be worth ~2.5x what the engine gives it. Nothing in the decomp or transcripts supplies that; the descent also moved `torrent_stall`, `torrent_hits`.
* Decision (coordinator): Brock potions stay 0 (plans 942-944 walked it 0 -> 3, rebased to 946-brock0; `earlyknobs.py` no longer proposes Brock potions). Heal-before-Brock is an **upside note only**.
* The better lever the engine sees is **entry level >= 13**, which makes the fight 100% at any HP (no Torrent needed).

## 2. The lane past rival-ss-anne (plan 940-lane)

`risk_until victory-road`, `risk_abandon` (a lane loss ends the attempt, like the flat `fatal`), `risk_windows` (power per runner window, `tools/fit_lane.py`, grid + interpolation, R = 800k, WR+160m):
rock-tunnel 1.10, snorlax 2.30, koga 1.49, blaine/sevii 0.91, giovanni 1.45, door 0.81. Flat `fatal` removed for rock-tunnel-trainers, hideout-giovanni, rival-tower, surge, koga, fuchsia-gym-trainers, blaine,
three-island-paxton, rival-silph, sabrina, erika, giovanni, rival-route22, victory-road (post-door `fatal` kept: 4 door attempts). Window survival model / runner: .437/.40, .604/.64, .722/.71, .917/1.0, .923/.89, .693/.63,
.733/.60, .754/.67; to the door .062 vs .040. Honest power 1.0 is far too deadly after Misty (Rocket Hideout window .48): the powers stand in for the team and tactics the carry-only lane lacks, exactly like `risk_power` 2.0 at Misty.
Window 3 fits at power 1.1 once the Cerulean rival is put in window 2 as `fit_attempts.py` labels it. `early_helpers` is limited to <= `no_salvage_until`. `tools/fit_lane.py --check PLAN` prints the window table.
Test: `risk_windows_scale_the_lane_and_abandon_replaces_the_flat_hazard`.

## 3. Descent (`descend.py --set earlymid`, WR+2m, seed 1 R 400k, confirm seed 2 R 800k; new `tools/midknobs.py`)

940 -> 946 -5.2 h (seed 2), then 947-959: Brock sacs 0 -> 2, torrent_stall off, candy_early_n 6, rival-tower sacs 1, rival-route22 sacs 3 + X Special, rival-ss-anne heal_below .5, torrent_hits 2.0,
drop Josh, manip min_spe 24. **959 at WR+2m: 62.1 h (8605 finishes, R = 1.6M, seed 3) vs 946 132.5 h, paired -70.4 [-75.0, -65.6].**

**Caution (not a result to quote).** 959's windows are .706/.615/.730/.972/.971/.843/.869/.948 against the runner's .40/.64/.71/1.0/.89/.63/.60/.67: the search found sacrifice-heavy lines the runner never plays (Brock .706 from two
helper sacrifices; Route 22 sacs 3) and the fitted lane powers were not refitted to them. Treat 959 as an upper bound; the sacrifice counts at Brock / Route 22 and the six early candies need a human check. No WR-rung confirmation was run.

## Next
* Check whether Brock helper sacrifices are physically available (which mons are alive at Brock) and cap `sacs` accordingly; refit `risk_windows` after any tactic that shifts pass rates by more than the runner's CIs.
* Level lever: a candidate that reaches L13 before Brock (Water Gun) priced in time.
* WR-rung confirmation of 959 and a second seed at R >= 1.6M.
