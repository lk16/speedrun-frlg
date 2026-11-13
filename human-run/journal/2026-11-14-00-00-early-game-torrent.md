# 2026-11-14: the first 30 minutes — Torrent, deaths without salvage, the encounter-seed manip (plan 377)

Task: the first half hour decides most real attempts (Main97: Brock-split pass 25-75% by day, 2/12 and 3/14 on bad days,
deaths are 1-6% crit/min-roll events; he keeps Squirtle at <= 1/3 HP, skips the PC potion/Centre/Mom heal, stalls against Rock
Tomb, takes Sammy's poison). The model (376) had Brock +8.7% (3 min of forest encounters against a real sample with none),
Misty 105 s against 54 s, nobody died before Misty, and a quiet-seed knob that scored worse. Plan 377 is the result;
`plans/CURRENT` now points at it. It is a **model change**: EHT(WR+10m) 132.8 -> 53.1 h is not a plan improvement of the same model.

## What was built (all knobs default off; `crates/frlg-human`)

* **Torrent tactic** (`battle.rs` `torrent_action`, `Tactics::torrent`): the engine already applies x1.5 at HP <= max/3
  (`pokemon.c`, `CalculateBaseDamage`); what was missing was the carry *staying* low. Withdraw when the foe's max roll KOs, the carry
  is faster and one Defense stage saves it (Rock Tomb 8-10 -> 5-7, the runner's number); a stall turn (best non-Water attack) while
  above 1/3 HP when one hit is survivable and lands the carry in the boost. Brock at the WR's state (Lv11, 10/32, Tackle/Bubble/
  Withdraw) wins 97% in about 2 actions per foe (test `brock_at_one_third_hp_with_torrent_is_a_short_survivable_fight`);
  at full HP the same fight is 3.9 actions: Torrent is what makes the WR's 2.
* **Torrent phase** (`torrent_until`, `torrent_hits`, `torrent_heal_below`, `poison_frames_per_hp`): skip `center-heal@*`, no pre-fight
  potions, a pre-fight window heal only to `torrent_hits` max-roll hits (`entry_floor`), poison kept and ticking in the field
  (`decompiled/src/field_control_avatar.c:710-733`, `field_poison.c:92-119`: 1 HP per 5 steps, faints at 0; the human drinks an
  Antidote at `poison_floor`).
* **No salvage** (`no_salvage_until`): a whiteout ends the attempt (STRATEGY 7: "it's so much better to just get another run").
* **Quiet seed keeps wanted encounters** (`manip.quiet_keeps_wanted`), **extra optional fights** (`extra_fights`), per-fight
  `power`, `carry_power_early`.
* **Instrumentation** (`frlg-human splits`): per fight the *core* seconds (intro to the last faint, comparable with the video's
  intro-to-defeated numbers; the old "model s" included outro and level-ups, which made Brock look 54 s against 31), the fit
  statistic over the 17 fights, and `reach`: of the attempts that pass the starter, the share that survive to each split.
* Test `wr_starter_stats_match_the_video`: the manip wiring is right (the WR starter's IVs/Rash give 32 HP at Lv11 and 56 at Lv20,
  exactly the video's "10/32" and "56/56"; IV order HP, Atk, Def, Spe, SpA, SpD end to end through `manip::sample_good`,
  `make_mon`, the nature table).

## Findings

1. **The old "refuted" Torrent was a global policy.** With the window heal, Withdraw, the stall and a 30% emergency heal, a Torrent
   phase to the League is worth +19 h at WR+10m over the heal-up default (to Mt. Moon alone it is +-0: the early fights are short
   either way; the mid-game fights are where HP/Torrent decides). The stall turn alone is worth 2.4 h; the optimum is flat at
   2 max-roll hits; 3 hits and 0.5 hits are both worse; heal below 0.3 beats 0 (+7.6 h) and 0.5 (-3 h).
2. **Early deaths in EHT terms.** At WR+10m the first half hour is most of an attempt's life (pace resets kill the rest), so every
   10% of early deaths is ~10% of the finishes. Salvaging a whiteout (the old retry from the Centre) was *worse* than dying (99.5 vs
   132.8 h): the retried runs spent their time and rarely finished.
3. **The quiet seed.** Old knob: every encounter in the first 60 rolls is a miss, so the Route 1 exp kills and the forest bug
   disappear (carry Lv7.4 at Brock, 3 species instead of 6, +30 min of fill). It scored worse for that reason, not because the manip
   is bad. The real seed also chooses which encounters happen ("a perfect two encounter forest"), so unwanted ones are suppressed
   and wanted ones kept: Brock 14:27 (the saving against no manip is 1:29, the runner says about a minute; his Brock-split with and
   without manip, 14:14 vs 14:20, hints at much less, so this is an upper estimate). 99.5 -> 67.3 h. The number of allowed hits
   (2/4/6) does not matter; the window and the wait for a title frame are cheap (an off-timer ~5 s).
4. **Why carry_power was knife-edge.** `carry_power` multiplies Atk/SpA; early fights are hypersensitive to it (Brock reach 67% at 1.9,
   41% at 1.8, 33% at 1.5; EHT 74 h vs 200 h at 1.65). Torrent explains part of what it was fitted to (Brock 2 actions) but the
   mid-game still needs 1.9 (1.6: zero finishes at WR+10m). It is a stand-in that has not been refit.
5. **Fight lengths**: Brock core 43 s (video 31: its intro is 4 s, the model's 8 s; every other piece fits), Misty core 60 s with 5.1
   actions (video 54 s, 6-8). Misty was Lv18 in the model against Lv20 in the video; two optional Mt. Moon trainers (Josh, Miriam)
   fix the level and are worth 4.9 h in the final plan; a third (Iris) costs 12 h.

## Death rates (model, no resets, attempts that passed the starter; `splits` reach line)

Brock 83%, Misty 77% (step 93%), Bill 73%, Bike 71%, Rock Tunnel 70%, Giovanni 1 66%, Koga 64%, Sabrina 53%, E4 entry 44%.
The video's good day is 75% at Brock; bad days 17-40%. Hits 0.5..3 give 75-86%, so the model has one regime (good day).
Deaths after Brock are 5-8% a split (video: Koga/Sabrina deaths, Rock Tomb, Quick Attack crits). The Misty step is too safe (the
video's Misty-split deaths exceed Brock deaths on a good day); the Cerulean rival at power 0.75 would take 7% more (not applied,
no action count in the video to pin it).

## Not done / next

* Day regimes (manip drift, bad-day pass rates 17-40%): a mixture over `carry_power`/hits per attempt, or per-day sigma.
* Persim Berry vs Misty's confusion, Sammy's poison rule (poisoned: leave; else camp for both bugs), the 6 potions and their
  count, X Special at Pewter, Mega Kick PP budget (5 PP: the sim spends it on trainers), Route 3 Jigglypuff, rival Growl branch,
  "two of three" gate at Pewter (a reset rule on the HP state at the gym door).
* Refit `carry_power` jointly with the Torrent phase on the 16 fights (the splits fit statistic is the objective); the E4 fights
  are other workers' (the fit is 0.28 over 17 fights, 0.31 without the League).
* The pre-League fill (+12% at Start E4R1) is untouched.
* Reset thresholds were not re-tuned (early ones are inert; surge/giovanni 0.9 is worse by 0.7 h).
