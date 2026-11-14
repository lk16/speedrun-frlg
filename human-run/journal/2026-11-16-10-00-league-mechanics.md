# 2026-11-16: League mechanics instead of fitted multipliers (plan 804)

Task: replace `league_power` 2.2 / `carry_power` 1.9 / `mewtwo_power` 1.5 by explicit mechanics from `decompiled/` where possible, refit
the remainder, evaluate the Mewtwo moveset. Branch base: 803. All new parameters default off; **plan 804 turns them on. Rows marked MODEL
CHANGE are model statements, not plan evaluations.**

## 1. What was modelled (all in `battle.rs` / `sim.rs`, flags in `params.rs`)

| flag | mechanic | cite | effect on the fit |
| --- | --- | --- | --- |
| `ev_cap` | `MonGainEVs` stops at 510 total / 255 per stat | `pokemon.c:5512-5595` | none (the carry never reaches 510 in the route) |
| `ev_lag` | stats are cached, EVs change them only at the next level-up/candy; EV is given before exp in `Cmd_getexp`, so a level-up in the same KO sees it | `pokemon.c:5591-5593` (`SetMonData` of the EV only) | carry slightly weaker mid-level; +0.4/+1.5 h at WR+2m |
| `mewtwo_evs` | Mewtwo gains the EV yield of every R2 foe (flushed at its 3 level-ups) | same | -1.1/-1.0 h (n.s.) |
| `black_glasses` | Dark +10% on the Bite user's SpA from `black-glasses-swap` | `pokemon.c:1468`, `RocketHideout_B3F` ball, `items.json` param 10 | Lorelei 9.5 -> 10.1 actions |
| X Accuracy / Guard Spec | new `XItem::Accuracy` (+1 acc stage) and `GuardSpec` (Mist 5 turns); `xbuys[].accuracy/guard` (950 / 700) | Celadon 5F list, `items.json` | harness at Lance L59: X Accuracy +2.4 pts win, -1.4 actions; Guard Spec at Champion 1: -1.4 pts (no gain in the engine) |
| `mystic_after` | Mystic Water bought with 1,000 coins = 20,000 yen (prize counter), held instead of the glasses | `strings.c:425,537` | **+24 to +28 h worse** (20k yen starves the X-item/heal purchases): refuted |
| `mewtwo_keep` | which Mewtwo slots a TM teach may not replace (WR set keeps Safeguard, loses Recover) | `v4-e4r2.md` | identical results: the engine's Mewtwo never picks Recover or Safeguard |
| `league_torrent` | no League pre-fight full heal while the carry sits in the Torrent window | `CalculateBaseDamage` | **zero effect**: after the engine's fights the carry is never inside the window that also survives the next foes' best opening hit (window empty at Bruno/Agatha) |

Mystic Water and Twisted Spoon are the only other type boosters on the route's reach (Mystic Water: coins; Twisted Spoon: 5% wild Abra hold): neither is
the WR's (the video shows Black Glasses only).

## 2. What the mechanics do NOT explain

Refit with and without the mechanics (`league`, R = 200 000 x 2 seeds): minimum of mean |log(model/WR)| actions+seconds stays at league_power 2.0-2.1
(old 2.0-2.2), carry_power 2.0, mewtwo_power 1.4-1.45. **The explicit mechanics account for under ~5% of the multipliers.** Findings that narrow the cause:

* Isolated harness fights at the *video's* levels (Lorelei L50 ... Champion L55, WR bag) need power 2.0-2.5 to reach the WR's actions; the model's carry is
  also **6 levels too high at Start E4R1** (56.5 vs the WR's 50; `splits` lvM/lvW: the dex-fill hunts give the carry exp that the WR spends on throwers),
  so the stand-in is larger, not smaller, at honest levels.
* Damage the player takes matches the formula (Dewgong 21-33, Onix Earthquake 28-30); damage dealt is 1.5-2x short. Compatible with Torrent x1.5 (the
  WR sits at 28/153, 20/156, 51/159 for Lorelei 2-5, Bruno, Agatha: `v3-e4r1.md`) times one X Special x1.5 on the *same* mon: Hitmonchan needs 131 and
  Surf does 63 plain, 94 in Torrent, 141 with one X Special. Lorelei's Jynx/Cloyster and Lance-2's Kingdra/Dragonite/Aerodactyl (all neutral for Mewtwo,
  72/63/89% of HP per Psychic) one-shot in the video but cannot in the formula: **unexplained, or a 4-s sampling error**. Bruno-2/Agatha-2 (all super
  effective) fit at power 1.0.
* More X Specials do not help the model: every one costs an action (grid `ns 2-4` x `setup_min` 1-2 x power 1.0-1.6 only made fights longer).
* Brock (Geodude Rock/Ground = 4x Water) fits at power 1.0 in the harness (2.4 actions vs 2).
So the multipliers remain stand-ins for the human's play (staying in Torrent, risk taking) and for 4-s sampling error; they are not EVs or items.

## 3. Refit (R = 200 000 x seeds 3,4, mechanics on)

League 5 fights: lp 1.9/2.0/2.1/2.2/2.3 -> sum |log| (actions+seconds) .179/.153/.164/.172/.178 -> **2.0**. Early 8 fights: cp 1.7..2.2 -> .418/.395/.343/.314/.333/.356 -> **2.0**
(actions want lower, seconds higher: the model's turn/overhead constants, not power, separate them). R2 4 fights: mp 1.3/1.4/1.45/1.5/1.6 -> .229/.209/.212/.224/.234 -> **1.4**.

## 4. Mewtwo moveset (honestly)

Paired WR+2m, seeds 1,2, R = 400 000 (diff vs 803+mechanics, h): Recover<->Safeguard 0.0/-0.1; Shock Wave replacing Recover/Safeguard/Swift (final Psy/SW/FB/EQ) **-0.6/-1.9**
(one seed significant); Shock Wave replacing Earthquake **+24** (Tyranitar is Psychic-immune, EQ is essential); dropping Fire Blast +1.9/+5.0; Brick Break (3000) +10/+7
(worse: costs and displaces); Water Pulse/Blizzard/Ice Beam(80k)/Thunderbolt(80k) are no-ops (single-use TMs already spent / unaffordable).
**The stand-in hides moveset value.** Probe at Mewtwo power 1.0 (`battle`, 2 500 runs): Lance 2 win 90.6% (SW 96.4%, Thunderbolt 97.2%, Ice Beam 92.4%), Champion 2 win 68.9% (Brick Break 85.0%,
Thunderbolt 77.9%); at the fitted 1.5 they are 99.6/96.0% and the differences vanish. So with the real damage unknown, the moveset descent cannot be settled: in the model
Shock Wave (free) is worth about 1% EHT, Brick Break and Ice Beam are not worth their price.

## 5. Evaluation of 803 -> 804 (WR+2m, R = 800 000)

| step (vs 803) | seed 1 | seed 2 |
| --- | --- | --- |
| mechanics only | -1.35 [-3.5,+0.5] | +0.64 [-1.2,+2.6] |
| mechanics + refit | +9.1 [+5.4,+12.7] | +6.1 [+2.9,+9.2] |
| + X Accuracy at Lance | +7.8 [+4.1,+11.3] | +4.7 [+1.7,+7.8] |
| + Mystic Water | +28.4 | +23.9 (refuted) |
| 804 on seed 3 | 102.9 h (1915 fin) vs 803 97.9 h: +5.0 [+1.9,+8.5] | |

The refit (lp 2.2 -> 2.0, mp 1.5 -> 1.4) costs ~+5-7% EHT: the absolute number is that sensitive to the stand-ins. Policy parameters were not retuned.

## Next
* Find the 1.5-2x: get the carry's real level (exp spent on throwers in the dex-fill hunts) and a Torrent-window model in the League, then drop `league_power`.
* Frame-level read of 5-10 WR fights (Cloyster, Lance-2 Kingdra/Dragonite) to settle whether the one-shots are real.
* Safeguard/Calm Mind use by the engine's Mewtwo.
