# 2026-08-14 — initial route written (route.md)

**Done**

- Wrote `route.md`: first end-to-end route, tier 0 / unmeasured. Decisions: **FireRed +
  Squirtle** (reuse of the tier-2-accepted defeat-brock prefix), party of 4 — Squirtle
  (Surf+Strength), **Paras** as the Cut+Dig slave (Mt Moon B1F is 100% Paras,
  `wild_encounters.json`; only early Cut learner that also takes TM28,
  `tmhm_learnsets.h`), **Abra** (Teleport) and **Pidgey** (Fly) both caught in Route 24/25
  grass on the Bill leg. Gym order Brock→Misty→Surge→Erika→Koga→Sabrina→Blaine→Giovanni;
  bike early (voucher on the Vermilion visit, shop 20 tiles off Cerulean PC); Snorlax on
  the Route 16 side; Cinnabar via Fly-Pallet + Route 21.
- **Found a mandatory gate `plan.md` missed**: the Cinnabar Gym door unlocks only when the
  Secret Key ball is taken — `CinnabarIsland_EventScript_CheckUnlockGym` gated on
  `FLAG_HIDE_POKEMON_MANSION_B1F_SECRET_KEY`
  (`decompiled/data/maps/CinnabarIsland/scripts.inc:41`; ball at `PokemonMansion_B1F`
  (5,7), `item_ball_scripts.inc:334`). **Pokémon Mansion is on the critical path.**
- Supporting facts pinned: Silph Co 1F has no story-flag entry gate (`SilphCo_1F/scripts.inc`
  OnTransition only sets the worldmap flag); `allow_escaping` true for Hideout B4F, Tower
  7F, Silph 11F (Dig exits all three), false for Safari Center; Safari = 600 steps / 30
  balls (`safari_zone.c:27-33`); Gold Teeth in `SafariZone_West` (28,14); givers for
  voucher / HM02 / HM03 / HM04 cited in route.md §1-2.
- New `frlg-mapgraph` legs, added to the dump table and regenerated into
  `distances.json`: Vermilion PC→Fan Club 30;
  Cerulean PC→Bike Shop 20; Celadon PC→Route 16 house 109 (`--cut --ignore-objects`);
  Celadon PC→Saffron Gym 100; Route 16 house→Fuchsia PC 351 (`--ignore-objects`); Safari
  entrance→Gold Teeth 195 via East→North→West (the direct Center→West door is blocked) +
  33 on to the Secret House.

**Failed / punted**

- Pokémon Mansion is statically unpathable (`setmetatile` switch barriers) — cost unknown,
  same modeling debt class as gym interiors. `frlg-mapgraph` returned unreachable even
  with `--ignore-objects`.
- No frame numbers anywhere: everything past Brock is tile floors.

**Next**

- Start building segment 2 (Pewter→Cerulean incl. the Paras catch) on tier 1; that
  exercises the catch machinery the route now depends on.
- Model Mansion/Victory Road interiors, or savestate-explore them in the emulator.
- Re-sweep list is route.md §7.
