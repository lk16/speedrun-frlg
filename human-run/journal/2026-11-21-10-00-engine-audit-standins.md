# 2026-11-21: engine audit against the decomp, the trainer AI, and what the stand-ins are made of

Task: replace `carry_power` / `league_power` / `mewtwo_power` / `risk_power` by real mechanics where the decomp allows, check the WR's
unexplained one-shots, fix engine bugs with tests, refit what is left, re-evaluate CURRENT. Rows marked MODEL CHANGE in `scores.jsonl` are
model statements, not plan evaluations.

**Headline.** The *damage* engine is exact (every modifier of `CalculateBaseDamage` is pinned by a unit test): the 1.5-2x the stand-ins stand for is
not a missing damage mechanic. The bugs were in the **trainer AI**, and they all made the model's leaders, E4, champion and rivals too gentle:

1. TRY_TO_FAINT ignored recharge / two-turn / self-KO damage (Lance's whole team has Hyper Beam): CURRENT 155.6 -> 200.2 h at WR+2m.
2. The trainers do not send their mons out in party order: after every faint the game runs `GetMostSuitableMonToSwitchInto`. The engine fought the
   party in order. The port reproduces **all eight send-out orders the WR video shows** (six of them differ from the party order): 200.2 -> 541.3 h.
3. `ShouldSwitch` (absorb abilities, resisting bench mons with a super-effective move) was missing: 541.3 -> 536.0 h.
4. Small damage-engine gaps (type-table flooring, Guts / Hustle / Swarm / Marvel Scale / Huge Power never applied): no EHT effect.

After the refit of the risk windows (`fit_lane.py`: they are fitted to the runner's survival, so the engine's extra deadliness is partly absorbed there) and a three-stage descent,
**CURRENT = 1016-manip131: WR+2m 171.8 h (2312 fin), WR 205.7 h (1875 fin), R 1.6M seed 11**, against the pre-audit 1012's 155.8 / 185.0 on the old engine: the honest cost of the AI fixes is about +10%
(1013-ai-order, the refit before the descent, 178.7 / 215.8 h).
The stand-ins decompose: `carry_power` 1.7-2.0 is Torrent x1.5 times a level gap of 1.0-1.2 times ~1.1; `mewtwo_power` 1.4 is about one X Special stage (x1.33)
times ~1.1; `league_power` 2.2 is **not explained** by Torrent, X Special stacking, X Speed, Black Glasses, EVs or levels. The fits did not move with the AI changes.

## 1. The audit (`crates/frlg-human/src/battle.rs`, tests in the same file)

| piece | decomp | result |
| --- | --- | --- |
| stat formula, natures, EV/4, HP `+L+10` | `pokemon.c:2093-2170` | exact; the WR Mewtwo's HP (231/235/238/241 at Lv70-73, `v4-e4r2.md`) comes out at every level from an HP IV of 5 (`wr_mewtwo_hp_matches_the_video`) |
| badge boosts | `ShouldGetStatBadgeBoost` (`pokemon.c:2381`), speed `battle_main.c:3444` | Boulder Atk, Soul Def, Volcano SpA+SpD, Thunder Speed, **player side only** (a trainer mon has none), the defender side applies to the player's Def/SpD; `damage_modifiers_follow_the_decomp` |
| type booster (Black Glasses, Mystic Water) | `sHoldEffectToType` (`pokemon.c:1461`), `items.json` param 10 | +10% after the badge, own type only: exact |
| Torrent / Blaze / Overgrow | `hp <= maxHP / 3` | exact; **Swarm was missing (added)** |
| Guts, Hustle, Marvel Scale, Huge Power | `pokemon.c:2478-2487` | **never applied (fixed)**; Hustle's x0.8 accuracy (`Cmd_accuracycheck`) too |
| burn, screens, weather, crit (x2; the attacker's drops and the defender's gains ignored), STAB, stages, roll 85..100 | `pokemon.c:2540-2640`, `Cmd_damagecalc`, `ApplyRandomDmgMultiplier` | exact |
| **type multiplier** | `Cmd_typecalc` + `ModulateDmgByType` (`battle_script_commands.c:1220-1330`) | **fixed**: the damage is floored after every matching table entry in table order (Fire on a Water/Ice mon: 33 -> 16 -> 32, not 33); `Dex::modulate`, `type_modulation_floors_per_entry_in_table_order` |
| crit table {16,8,4,3,2}, Focus Energy +2, high-crit +1; accuracy stages | `Cmd_critcalc`, `sAccuracyStageRatios` | exact |
| **AI: TRY_TO_FAINT** | `Cmd_if_can_faint` (`battle_ai_script_commands.c:1475`) vs `Cmd_get_how_powerful_move_is` (`:966`) | **fixed**: `if_can_faint` excludes only `power < 2`; only the "most powerful" test skips recharge / two-turn / self-KO / Overheat moves. The engine returned 0 damage for them in both, so a finishing Hyper Beam never earned the +4. Explosion keeps its exemption. 85 trainers carry TRY_TO_FAINT (`trainers.h`; 657 have only CHECK_BAD_MOVE). `hyper_beam_that_can_faint_is_chosen` |
| **AI: next mon after a faint** | `OpponentHandleChoosePokemon` (`battle_controller_opponent.c:1411`), `GetMostSuitableMonToSwitchInto` (`battle_ai_switch_items.c:428`) | **new, `most_suitable`**: pass one takes the bench mon the player's types (the single type twice for a mono-type mon) hit hardest, if it has a super-effective move on the player, else the next; pass two, when none qualifies, takes the highest `u8` `bestDmg` over the bench mons' moves of `AI_CalcDmg(fainted mon, player)` with the last move used (`gCurrentMove`, not the candidate's) x the fainted mon's STAB on the candidate's move type x the move's effectiveness on the player, lowest slot on ties. Reproduces Lorelei, Bruno, Agatha, Lance, Champion (R1) and Bruno 2, Agatha 2, Lance 2, Champion 2 (R2) as the video shows them: `trainer_send_out_order_follows_the_decomp_and_the_video` |
| **AI: switching mid-fight** | `ShouldSwitch` (`battle_ai_switch_items.c:296`), run each turn by `AI_TrySwitchOrUseItem` for every trainer | **new, `ai_should_switch`**: absorbing ability after a landed Fire/Water/Electric move (Lapras vs Surf, Arcanine vs Fire Blast), immune / resisting bench mons with a super-effective move; `trainer_switches_to_an_absorber_after_a_landed_water_move` |
| AI viability / bad-move scripts | `battle_ai_scripts.s` | spot-checked (Evasion up, Heal, Recharge, Paralyze, bad-move list): the earlier script-by-script port holds |

New tooling: `frlg-human dmg TRAINER LEVEL --moves .. --hp cur,max --stage n` (per-foe damage range and hits to kill, WR IVs / Rash / all badges by default),
`battle --ivs --evs --boost --print-stats`, `splits --within SECONDS` (fight table of the attempts that finish in time), `tools/fit_lik.py` (section 4),
`FightTactic.free_torrent` (section 3).

## 2. The WR's one-shots (`probe_r1_implied_stage`, `probe_oneshots`: ignored tests that print the tables)

Engine Blastoise = the WR starter (IVs 15/26/9/29/30/20, Rash), 0 EVs, Black Glasses, all badges, the move and HP the video shows (Torrent when HP <= 1/3).
"stage" = the SpA stage that one-shots the foe at the maximum / minimum damage roll.

| fight | foe | move, HP | engine at stage 0 (% of foe HP) | stage needed (max / min roll) |
| --- | --- | --- | --- | --- |
| Lorelei | Cloyster L51 | Bite, 25/150 | 40% | +3 / +4 |
| Lorelei | Slowbro L52 | Bite (super effective), 28/153 | 42% | +3 / +4 |
| Lorelei | Jynx L54 | Surf, 28/153 (Torrent) | 60% | +2 / +2 |
| Bruno | Hitmonchan / Hitmonlee L53 | Surf, 48/156 (Torrent) | 69% | +1 / +2 |
| Bruno | Machamp L56 | Surf, 48/156 | 57% | +2 / +3 |
| Agatha | Gengar, Golbat, Arbok | Surf, 51/159 (Torrent) | 80-91% | +1 / +1 |
| Champion | Alakazam L57 | Surf, 131/167 | 51% | +2 / +3 |
| Champion | Pidgeot L59 | Surf, 165/165 | 49% | +3 / +3 |
| Lance | Dragonair, Aerodactyl, Dragonite | Blizzard at +1 (the X Special of the bag count) | 202-292 vs 146-197 HP | all one-shots at +1: **Lance 1 is fully explained** |

Mewtwo, Lance 2 (Psychic, SpA 230 = IV 14 neutral, the stat line quoted in `STRATEGY.md` for a Mewtwo whose 231 HP matches the video; badges): Kingdra 65%, Dragonite L66 62%,
Aerodactyl 68%, Dragonite L72 51%, Gyarados 63% (the video needs two Psychics for it: matches), Lorelei-2 Lapras 57% (two Psychics: matches), Misdreavus 89% (two turns: matches);
Hitmonchan / Hitmonlee 155%, Machamp 122%, Gengar 192%, Crobat 153%, Arbok 158%, Cloyster 185% are one-shots at stage 0. The unexplained one-shots are Lance-2's Kingdra / Dragonite /
Aerodactyl and R1's Lorelei Cloyster / Slowbro / Jynx: all neutral, all needing x1.5-3 more than the engine's mon at +0.

None needs a mechanic the engine lacks. The stage persists through a whole trainer battle (all five mons), so **two X Specials at the start of Lance 2 (Mewtwo +2 = x2.0) one-shot all
three**, one leaves Dragonite L66 at 93%; in R1 Lorelei needs +3/+4 by her third mon, Bruno +2, Agatha +1. The WR bag counts (X Special 20 at 2:11, 11 after Agatha, 7 at the
Champion; R2 6 -> 5) fit 3-4 X Specials per big fight, but the 4 s frames show about one item turn per fight (an X Special is a 3 s bag trip) and the count before Lorelei is unknown (the second Celadon
stop comes after the 2:11 snapshot), so the stacking cannot be confirmed from the stream. The foes' send-out order also differs from what the earlier fits assumed (section 1).

## 3. `carry_power` is Torrent times a level gap (mid-game, no X items)

The mid-game fights have no X Special, so they test the damage formula directly (`v2-mid-fights.md`). With the WR's own level, HP and badges, plain Surf / Water Pulse
(`mid_game_hits_to_kill_match_the_video_without_a_power_stand_in`): Koga L40 at 33/120 (inside Torrent): Koffing 1 Surf, Muk 2 ("Surf x2"), Weezing 1-2; Sabrina L45 at 43/135: Water Pulse kills Kadabra,
takes Mr. Mime two ("Water Pulse ... Surf KOs"), Surf one-shots Venomoth and Alakazam; Blaine L41 at 109/123: Surf one-shots Growlithe / Ponyta / Rapidash, Arcanine two. **The engine needs no multiplier there.**
The model's carry differs in two ways: (1) it arrives at **full HP** (`FRLG_SIM_TRACE`: hp 1.00 in 1247 of 1250 Koga fights, all Blaine, 395 of 450 Giovanni, Route 22), never inside the Torrent window the WR
walks in at (33/120, 43/135, 35/143: it does not heal between the gym trainers), so Surf is a third weaker (Muk 3 hits instead of 2); (2) it is **3 levels short at Koga** (L37 vs 40: Muk 62-73 vs
74-88, x1.19). 1.5 x 1.19 = 1.79: the fitted 1.7-2.0. Sabrina and Giovanni 3 are 1 and 2 levels short.

`FightTactic.free_torrent` (timing lane only, default off) puts the Torrent boost on for the whole fight. Koga / Sabrina / Giovanni with it and `power` .667 (cp_eff 1.33) give 155.3 h against 155.8 h for carry_power 2.0
plain on the old engine (2488 vs 2481 finishes): equivalent; cp_eff 2.0 148.3, 1.6 154.6, 1.2 158.8, 1.0 161.7: these three fights are not where EHT is steep. The action fit with free_torrent has its minimum at
cp_eff 1.3 (.213) with a second at 1.7 (.205): flat. **Not adopted**: the plan's other mid fights have no HP reading, and the level shortfall is an exp-economy question, not a constant.

## 4. League: what X stacking and Torrent do and do not buy; the likelihood refit

* `league_power` 2.2 stays the minimum with the WR-like X queues (Lorelei 4 specials, Bruno 3, Agatha 2, Lance accuracy+special, Champion 4), with an X Speed first or last in every R1 queue, and with `free_torrent` on Lorelei / Bruno /
  Agatha (grid 1.2-2.2; mean |log| actions .131 / .099-.124 / .143, the plan's queues .102). It stayed the minimum through every engine change of this session.
* At power 1.0 with the WR's queues the engine still needs 17 / 12 / 10 / 16 / 19 actions for Lorelei / Bruno / Agatha / Lance / Champion against the video's 11 / 7 / 6 / 8 / 10, because it heals 3-4 times per fight against the
  WR's 0-1: the WR's foes die before they move, the model's foes get two or three hits in. Lance 1 is the clean case: the engine's Blizzard one-shots everything at +1; what costs the model turns is Gyarados choosing Hyper Beam /
  Bite where the WR's chose Dragon Rage three times (a draw).
* The video is one draw per fight; conditioning the model's fight table on finishing within WR+2m changes nothing (`splits --within`: all 17 means within 0.1 action), so the WR being a lucky run does not explain it.
* **Likelihood refit** (`tools/fit_lik.py`: P(|model actions - WR| <= 1) per fight under the model's own distribution, summed as a log-likelihood; engine before the AI changes): league_power 1.2 -12.7, 1.4 -6.65, 1.6 -6.56,
  1.8 -4.72, 2.0 -3.45, **2.2 -1.93**, 2.4 -2.31, 2.6 -4.55, 3.0 -4.83: 2.0-2.5 within two log units. mewtwo_power 1.0 -6.68, **1.2 -3.73**, 1.4 -4.51, 1.6 -6.44: 1.1-1.5 (Lance 2 alone pins the low end: 2% at 1.0).
* `mewtwo_power` is about one more X Special: Lance 2 at power 1.0 needs 15.3 actions, 9.9 with a two-X queue (WR 7.5); mp 1.4 with the plan queue and mp 1.2 with +1 stage fit equally in actions (.113). In EHT they are not
  equivalent (mp 1.2 with the extra X 196.6 h vs 156.8 h at 1.4, old engine).
* Not tried because the stream gives no value: Mewtwo's held item (a wild Abra's Twisted Spoon would add x1.1), the WR Blastoise's vitamins (Calcium is sold on Celadon 5F: +10 SpA EV each, cap 100, about +11% at Lv52).

## 5. Early-game stand-ins

`risk_power` (Misty 2.0, Rival Cerulean 3.5, S.S. Anne rival 2.0): EHT is flat in them (Misty 1.4 / 1.7 / 2.0 / 2.3 / 2.6: 164 / 161 / 156 / 155 / 150 h; Cerulean 2.5 / 3.0 / 3.5 / 4.0 / 4.5: 164 / 159 / 156 / 156 / 156 h; old engine).
Brock's lane needs none (the Torrent / HP table of 2026-11-17).

## 6. EHT against the stand-ins (CURRENT 1012, WR+2m, R 1.6M seed 11; **old engine**, for the shape; section 7 has the rerun on the final engine)

carry_power 1.7 745 h (366 fin), 1.8 302, 1.9 199, **2.0 156**, 2.2 148, 2.4 134, 2.6 126 (<= 1.6: no finishes). league_power 1.6 463, 1.8 298, 2.0 212, **2.2 156**, 2.4 151, 2.6 143.
mewtwo_power 1.0 474, 1.2 194, 1.3 174, **1.4 156**, 1.5 140, 1.6 144, 1.8 301 (the queue 'special' is blind to Tyranitar above 1.6, journal 2026-11-19). The fitted ranges (cp 1.7-2.0, lp 2.0-2.5, mp 1.1-1.5) span
126 h to 745 h: the absolute EHT is not identified, the ranking is.

## 7. After the AI changes: refit, descent, sensitivity on the final engine

* 1012 on the new engine with its old windows: **536.0 h** (WR+2m, R 800k seed 11, 351 fin). `fit_lane.py` (R 800k seed 1) refit the six windows to **.985 / 3.104 / 1.451 / .846 / 1.917 / 2.390** (1012: 1.367 / 2.755 / 1.443 / .819 / 1.427 / 1.098):
  the late windows (Sabrina / Erika / Giovanni 3 / Route 22) take most of the new deadliness. Window survival vs the runner's: .724 / .936 / .909 / .765 / .799 / .718 against targets .714 / .925 / .853 / .700 / .709 / .756.
  **1013-ai-order = 1012 + those windows: WR+2m 179.4 h (1096 fin, R 800k) / 178.7 h (R 1.6M), WR 215.8 h (1771 fin).** The fight-table fit did not move (league_power 2.2 sum .136, mewtwo_power 1.4 .225).
* Descent on the new engine (`descend.py --pointer CURRENT-AI`, WR+2m, R 1.6M seeds 1/2, confirm on accept): League tactics / xbuys / heal knobs 96 candidates, **0 accepted**; mid-fight tactics (Surge to Route 22) 50 candidates,
  **1014 rival-tower -x special -4.9 h [-7.9, -2.2], seed 2 -5.1**; early + manip knobs 133 candidates: 1015 rival-cerulean potions -1 -> 1 (-0.004 h, noise), **1016 manip.min_spd 8 -> 14 -4.2 h [-6.7, -1.5], seed 2 -3.3**
  (a guess knob: no stream threshold; the WR's own SpD IV is 20). **CURRENT = 1016-manip131: WR+2m 171.8 h (2312 fin), WR 205.7 h (1875 fin)** (R 1.6M seed 11; 1014 176.3, 1013 178.7).
  The League stage moving nothing means the old League tactics (heals, X queues, `league_heals`, `league_preheal`) are still the optimum when the foes choose their mons the way the game does.
* `fit_lane.py` on 1016 itself does not converge (sevii-gio3 saturates at its .60 lower bound with model survival .393 against the .709 target, because the window survival is not monotone in the power once the order is real); those windows would give
  352 h. Not used: the 1013 windows stand for 1014-1016. It is the one step of this pipeline that needs a robust bisection.
* Stand-in sensitivity on the final plan (windows fixed at 1013's, R 1.6M seed 11, WR+2m): carry_power 1.7 545 h (648 fin), 1.8 384, 1.9 242, **2.0 172**, 2.2 163, 2.4 160; league_power 1.8 296, 2.0 250, **2.2 172**, 2.4 151, 2.6 148;
  mewtwo_power 1.0 586, 1.2 221, **1.4 172**, 1.6 156. With the windows refit at the changed carry_power (fit_lane at cp 1.7 / 2.4): 345 h / 163 h, i.e. the fixed-window grid exaggerates the low side by about 1.6x. League and Mewtwo powers act after the last window.
* Likelihood intervals (section 4, final engine; best, within two log units): league_power 2.2 [2.0, 2.4]; mewtwo_power 1.2 [1.2, 1.6]; carry_power (mid fights) 1.4 [1.2, 1.8] (the plan's 2.0 is 2.8 log units below the best, Sabrina's 9 actions pull it down).
  Put through the EHT table: inside the 95% fit intervals CURRENT is **between about 150 h and 550 h at WR+2m** (cp 1.8 alone 384 h; lp 2.0 250 h; mp 1.2 221 h); the point 172 h sits at the optimistic end of cp and lp.

## Next

* Read the WR frames at 1 s around every fight start for the HP bar, the bag (X Special / X Speed counts), the Mewtwo summary screen (stats, held item) and the Blastoise summary screen at E4R1: those pin cp / lp / mp directly.
* The model's mid-game carry is 2-3 levels short at Koga / Blaine and arrives at full HP: model the WR's no-heal chain through the gym trainers (HP carried into the leader) and the exp it spends, then drop `carry_power`.
* `league_power` is the one stand-in with no mechanism; the next candidates are the heal rule (the WR heals 0-1 times per fight, the model 2-4) and the unobserved X turns.
* The AI port still lacks: enemy PP (Hyper Beam has 5), Pressure on Mewtwo, Natural Cure / Perish Song / Wonder Guard switching, the AI's other item types.
