# 2026-11-14: League engine audit (E4 / Champion, both rounds)

Task: make the model's E4 and Champion fights match what the real runs show (turn counts, seconds, a
prepared runner losing a few percent, not 30-55%). Everything below is `crates/frlg-human/src/battle.rs`
unless noted; decomp citations are in the code comments.

## What the audit found (real engine bugs, fixed)

| # | Bug | Effect on the model |
| --- | --- | --- |
| 1 | An enemy's recharge flag never cleared (a Hyper Beam silenced the mon for the rest of its life) | every Lance mon carries Hyper Beam: Lance was far too easy |
| 2 | Outrage / Thrash did not lock (2-3 turns, then confusion; `battle_script_commands.c:2719`, `battle_util.c:952`) | wrong enemy damage pattern |
| 3 | No weather at all: Hail chip, Rain Dance, Sunny Day (halves Water, Solar Beam instant), Sandstorm from Tyranitar's Sand Stream | Venusaur / Tyranitar / Lorelei fights |
| 4 | Roar / Whirlwind were no-ops; the AI scored them 100 (real AI_CV_Roar: -3 unless the player is at +3) | Pidgeot/Onix wasted turns; the real effect (drag-out, stages lost) now exists |
| 5 | Trainer mons were neutral-nature; the game fixes `personality = 0x88/0x78/0x80 + (nameHash << 8)` and nature = `% 25` (`CreateNPCTrainerParty`); `tools/add_natures.py` writes it to dex.json | every trainer fight's stats |
| 6 | AI viability scripts were "trimmed" generic ports (Reflect, Light Screen, Charge-up, Foresight, Scary Face/Rock Tomb, every stat-up/down script had its own logic; two used the wrong neutral stage) | enemy move choice; all scripts now ported one by one (`ai_cv`) |
| 7 | Overheat did not drop SpA; Future Sight did nothing; Water/Volt Absorb and Flash Fire ignored (Lapras absorbs Surf); Intimidate ignored | |
| 8 | Trainer Full Restores ignored the `validMons` gate (`battle_ai_switch_items.c:580`) and used `hp < max/4` in reals | enemy healed too often |
| 9 | Sitrus Berry on each E4 member's last mon (+30 at half HP) | |
| 10 | The human's "sure kill" ignored his own accuracy stage (Sand Attack x3 = 50%) | deaths at Alakazam |
| 11 | Heal loop: 11 Full Restores in a row against Outrage; heals when a sure kill was available; X Specials spent on foes that fall in one or two hits | wasted turns |

## What was resource modelling, not mechanics

* **PP.** Blizzard has 5 PP; the plan's carry used it on Agatha/gyms and was empty at Lance. Scarce-PP slots (base
  PP <= 5) are now rationed (value x0.6 unless they kill) and the last 2 Max Elixirs are kept for the League
  (`league_max_reserve`). With PP restored alone, round 1 went 100/93/98/90/85% -> 100/93/98/96/92%.
* **Heals.** The League heals were free and unlimited (Lance used 7 Full Restores per attempt). Heal items are now
  a bag: `xbuys[].heals` (Hyper Potion 1200, money model) and a top-up at the Indigo mart (Max Potion 2500) to
  `league_heals`; the human spends one between League fights below `league_preheal` HP.
* **X Specials / Revives.** Plan 376 spent all of fly-celadon-2's money on 8 Revives (for 4 Champion sacrifices) and
  arrived at Indigo with 0-4 X Specials. The WR bag has 20 X Specials and no Revive purchases.

## Calibration (fit, not mechanism): `league_power`

After the fixes a prepared runner (full PP, X Specials, heals) still loses 25-50% at Champion 1 with
`carry_power` 1.9, and the loss is *not* sensitive to power alone (Alakazam outspeeds, Sand Attack, crits,
Venusaur). Action counts against the WR video (Lorelei 11, Bruno 7, Agatha 6, Lance 8, Champion 10) as a function
of the League-only multiplier on Attack/SpA (`league_power`): mean |log ratio| 0.20 at 1.9, **0.11 at 2.2**, 0.15 at
2.6. EVs alone move Champion 1 from 21% to 58% win (60 EVs in every stat), which is the kind of thing the stand-in
absorbs: the WR carry's EVs, Black Glasses/Mystic Water (not modelled), vitamins. It is a fit, labelled as one.
`mewtwo_power` 1.5 already matches round 2's counts (Bruno-2 5.4/5, Agatha-2 5.0/6.5, Lance-2 7.9/7.5, Champion-2 10.6/11.5).

## Result (plan `377-league-calib`, engine as of this commit, R=20 000 reaching the League)

| fight | win | model actions | WR actions | model s | WR s |
| --- | --- | --- | --- | --- | --- |
| Lorelei | 98.5% | 10.9 | 11 | 149 | 148 |
| Bruno | 97.5% | 6.0 | 7 | 108 | 104 |
| Agatha | 97.9% | 6.1 | 6 | 112 | 88 |
| Lance | 94.5% | 8.7-10.9 | 8 | 145 | 120 |
| Champion | 94% | 10.3 | 10 | 185 | 154 |
| Lorelei-2 | 99.5% | | | 118 | |
| Bruno-2 | 99.7% | 5.4 | 5 | 102 | 92 |
| Agatha-2 | 100% | 5.0 | 6.5 | 99 | 98 |
| Lance-2 | 94-99% | 7.9 | 7.5 | 116 | 110 |
| Champion-2 | **~55%** | 10.6 | 11.5 | 158 | 196 |

Round 1 chain: ~75-80% (was ~45% for plan 376 on the fixed engine, ~70% before the audit by accident of the bugs).
**Champion-2 is the open problem**: Mewtwo (Psychic) is weak to Dark/Bug/Ghost: Tyranitar's Crunch does ~145 of its
~240 HP, Megahorn OHKOs, and Tyranitar's Sand Stream chips 1/16 per turn. Mewtwo wins that duel only when it
enters with ~180 HP and kills in two Earthquakes; the greedy human heals at the wrong time. A lookahead human
(`league_rollouts`, opt-in) lifts it to ~58% but costs ~35x runtime, so it is off.

## Not done / next

* Mewtwo's moveset: Recover occupies a slot; the WR has Shock Wave slot one (TM34, Surge's gift, now a descend
  candidate) and Ice Beam is worth 25 points of Champion-2 win in the probe (`frlg-human battle ... --species
  SPECIES_MEWTWO`). Champion-2 sacrifices (teammates absorb the Crunch) are untried.
* Black Glasses / Mystic Water (+10%), Guard Spec, X Accuracy and the human's real heal choices are not modelled.
* The pre-League pace is the bigger gap to the WR (Start E4R1 +13.7%), not the League.

## Tools

`frlg-human battle` gained `--power --nature --ev --hp-iv --spa-iv --heals --others --rollouts --setup-min`;
`FRLG_SIM_TRACE=1` prints every League fight with stats/kos/heals; `descend.py` got `--fights`, heal-bag and
Mewtwo-TM knobs and no longer needs `starter_min_total`.
