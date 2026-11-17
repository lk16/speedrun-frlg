# 2026-11-28: audit of five strategy dimensions; Calm Mind (plan 1043) and a dex.json data bug

Paired numbers: R 3.2M, seeds 71/72, against CURRENT `1040-optstop` (WR+2m 431.4 / 427.1 h, WR 592.9 / 592.6 h).

## 1. Dex route (60 species)
* **Data bug fixed.** `tools/gen_dex.py`'s evolution regex ended a species' table at the first `},\n`, so every species with several
  evolutions lost all of them: **Eevee** (Vaporeon/Jolteon/Flareon), **Gloom** (Vileplume), **Poliwhirl** (Poliwrath), Slowpoke (Slowbro).
  `evolve()` and `dexknobs.py` read `dex.json` `evo`, so none of those events could ever fire or be proposed. Fixed in the generator and patched
  into `dex.json` (only those 8 rows; 1040's score is unchanged to the digit).
* Eevee (gift, in the bag 100%) -> one stone evolution at `stones` (stone bought 2100): Vaporeon +16.4 h [-2.2, +33.1], Jolteon +13.4 h
  [-4.2, +30.2] (WR+2m, R 1.6M, seed 71). Not better: the fill (16 s per species on average, 11 species in 3:01) is cheaper than 2100 yen + a shop trip. Refuted.
* Game Corner prizes (decomp `CeladonCity_GameCorner_PrizeRoom/scripts.inc:30-100`): LG Pinsir 2500 coins, Dratini 2800, Porygon 6500 (x20 yen); the purse is 17k at the Sevii trip, so none is affordable before the fill. Oak needs
  Kanto-caught >= 60 (`PalletTown_ProfessorOaksLab/scripts.inc:905-907`, `Num Kanto Caught`).
## 2. Bag manipulation / trainer skipping
* Decomp: a trainer fights when `CheckForTrainersWantingBattle` (`trainer_see.c:88`) runs from `ProcessPlayerFieldInput` (`field_control_avatar.c:209`) with the player in a
  facing cone. `LOOK_AROUND` / `WANDER_*` / walk-sequence trainers (47 + 34 + ~40 objects in the maps) change facing by `Random()` and turn toward a
  *dashing* player inside their range (`ObjectEventIsTrainerAndCloseToPlayer`, `event_object_movement.c:2772`, needs `PLAYER_AVATAR_FLAG_DASH`). I found no item
  or bag state that suppresses sight; "bag manip" in the stream is not derivable from the decomp (open question, STRATEGY section 802).
* Model: `forced-audit.md` treats static cones exactly and dynamic trainers as free bodies (optimistic, no knob possible without a movement model); `skip_groups`, `group_n`, `cond_skips` cover the rest.
## 3. Items and money
* Money is not binding: 17-30k mid-run, 40-56k after the League. Vitamins cost 9800 for +10 EV = under +1 stat point each at Lv 45: not worth it. Leftovers (Route 12 / 16) are underfoot items and need the
  Itemfinder (`field_control_avatar.c:497-500`, `itemfinder.c:246`), which the Route 11 aide gives only at 30 species: out of reach. Repel: `repel_on` already searched (1784 rows).
## 4. Moves
* **New: Calm Mind.** Mewtwo's TM list has TM04 (`tmhm_learnsets.h`), Sabrina gives it (`SaffronCity_Gym/scripts.inc:22-24`), and the plan visits her. The engine only queued X items as setup;
  `XItem::CalmMind` (`"calm"` in an `x` list) plays the move as a normal turn. Teaching it alone changes nothing (+4.5 h, n.s.); replacing the Champion-2 `special, special` queue with `calm, calm`
  (Mewtwo = Psychic / Fire Blast / Earthquake / Calm Mind): **WR+2m -179.6 / -173.4 h, WR -246.4 / -244.7 h**, all CIs exclude 0. The effect is entirely at Champion-2 (arrival survival .46 -> .81); Lorelei-2..Lance-2 unchanged.
  Queue variants (R 1.6M, seed 71): `calm,calm` 255 h; `special,calm,calm` 269; `calm,special` 281; `calm x4+special x2` 303; `calm` 363; Psychic/Recover/Fire Blast/Calm Mind (no Earthquake) 346. A third calm adds nothing (HP gate).
* Caveat: Champion-2 is the least-calibrated fight (one WR sample); the engine-only death .45 is a stand-in. The mechanism is the decomp's, the size is the engine's. Mewtwo's PP is not tracked (50 per move), harmless for 2-4 Calm Minds.
## 5. PP
* Already tracked per slot for the carry (`slot_pp`, Elixir / Max Elixir pickups, scarce-slot rule); Mewtwo's is not. Nothing found to change.

Adopted: `plans/1043-calm`, CURRENT. Not done: confirm on the 1042 refit model, tune other fights' queues with `calm`, Mewtwo PP model.
