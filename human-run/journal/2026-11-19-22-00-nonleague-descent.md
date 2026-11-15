# 2026-11-19: non-League descent (`--set all`), plans 984-1001

`descend.py --pointer CURRENT-RISK --rung WR+2m --r1 1600000 --r2 1600000 --set all` from 983-epot43, evidence guards on (no hidden candies, candy cap 6, League sacs 0, torrent/manip caps), seed 1 screen and seed 2 confirm. 664 candidates in pass 0; stopped by the coordinator after ~640 (no promotion in the last 40+ min), not by convergence; pass 1 and the remaining route variants not run.

Promoted (each confirmed on seeds 1 and 2): 984 drop Nidorino candy evo, 985 drop Sandslash candy evo, 986-989 level evolutions (Nidorino at Koga, Pidgeotto, Fearow, Sandslash; pre-evolution in bag 31-91%), 990/991 aggr brock 0.8->1.0, surge-badge 0.7->0.9, 992 drop Clefairy purchase, 993 ball_batch 20->5, 994 no Repel on Route 22, 995 champion-2 +X Attack, 996-1000 xbuys (heals 4->2, attack/speed cuts, special 16->12).

## Fresh-seed confirmation (seed 9, paired against 983)
| plan | WR+2m (R 800k) | WR (R 1.6M) |
| --- | --- | --- |
| 983 | 199.6 h (977 fin) | 236.4 h (1640 fin) |
| 994 | 142.6 h, +57.0 [46.8, 67.3] | |
| 1000 | 118.7 h (1981 fin), +80.9 [70.2, 92.2] | 221.7 h, +14.7 [2.8, 28.7] |
| **1001** | **151.1 h (1301 fin), +48.5 [38.4, 60.3]** | **187.2 h (1979 fin), +49.2 [40.0, 58.8]** |

## Guards
* level_gap (guess, plans use 8): same comparison with both plans at level_gap 0: 255.1 -> 193.1 (+62.0 [44.0, 82.4]); at 12: 192.4 -> 112.8 (+79.6 [69.8, 90.7]). The gain does not depend on it (part of it is xbuys/aggr, not level evolutions).
* Early windows (`fit_lane.py --check`, seed 2, R 800k, WR+2m): 1000 puts forest-brock at .589, **outside the runner's [.29, .53]** (983: .513). Cause isolated to 993 `ball_batch` 20->5 (1000 with batch 20 gives .513 exactly): fewer early balls leave money for Brock-side potions, an economy the stream does not show. **CURRENT = 1001 = 1000 with ball_batch 20.** It costs 32.4 h [25.3, 39.2] at WR+2m versus 1000. 1001 windows: .513/.542/.755/.898/.911/.715/.742/.753 vs runner .40/.64/.71/1.0/.89/.63/.60/.67: all inside the runner CIs; survival to the door .069 vs .040.
* Evidence notes for the xbuy promotions (996-1000): they cut X Attack/Speed at the second Celadon stop to 0/2 and X Special to 12; the runner reports 11-20 X Specials and X Accuracy in the League; the plan keeps 12+16 specials, but the zero X Attack stock is a model preference without a stream source. Worth a re-check.

## Not done
Power refit (league_power / mewtwo_power) after the descent; pass 1; route variants; the ideas for lowering time-to-WR beyond the descent were only analysed, not implemented: the reset rule is already a free per-split pace gate (aggr per split, dex-conditioned before Oak), extra splits were refuted earlier, and a clock-conditional skip lever (Power Plant / Pewter when behind, STRATEGY 12 and 14) would need a new conditional-segment mechanism in `sim.rs` (segments are fixed at route build time; `skip_powerplant` is static). That is the best next engine candidate.
