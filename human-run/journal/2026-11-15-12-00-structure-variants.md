# 2026-11-15: structural route-order variants (plans 700+)

The coordinate descent over knobs converged on 608. This session adds `route_variants` (a list of named order variants of the WR order, `route_wr.rs`) and
tests the orderings a WR-level human could choose. Screen: WR+2m, seed 1, R = 400k; confirm seed 2, R = 800k. Baseline 608 = 117.8 h (seed 1).

## Accepted: 700-warden-early

`warden_early`: the Gold Teeth go to the Warden right after Koga, while still in Fuchsia, instead of a Fly back to Fuchsia after Erika (`fly-fuchsia`, 700 frames, removed).
Gold Teeth lie in Safari West beside the Secret House that gives HM03 (`decompiled/data/maps/SafariZone_West/map.json:30`, `SafariZone_SecretHouse/scripts.inc:9-11`), so the first
Safari trip already collects them; the Warden accepts them whenever he is visited (`FuchsiaCity_WardensHouse/scripts.inc:5-30`, flag `FLAG_HIDE_SAFARI_ZONE_WEST_GOLD_TEETH`).
The WR run hands them in after Erika (2:15, WR notes); no reason is given anywhere in the evidence, so this is a deviation from the runner, labelled as such. The Warden-house Rare Candy
(item ball, `FuchsiaCity_WardensHouse/map.json:58`) also moves earlier; isolating it (candy kept after Erika) gives the same -1.8 h, so the gain is the removed Fly.
608 -> 700: seed 1 -1.68 h CI [-2.83, -0.78] (R 400k), seed 2 -1.91 h CI [-2.72, -1.24] (R 800k), fresh seed 31 R 1.6M -1.89 h CI [-2.37, -1.45] (117.5 vs 119.4 h);
jitter N = 2: -1.05 [-1.60, -0.59]; N = 6: -3.41 [-4.62, -2.33]. Real saving ~12 s per run, 1.5% EHT.

## Refuted (seed 1, R 400k, all in scores.jsonl)

| variant | diff vs 608 | why |
| --- | --- | --- |
| `erika_cycling` (Erika from the Celadon Fly of the Snorlax leg) | +75.8 h | Erika at that level is a death sink; saves a Fly but loses far more |
| `erika_pre_silph` (Fly to Celadon, Erika, then walk on to Saffron) | +4.5 h CI [1.7, 7.5] | one Fly saved, Erika earlier |
| `surge_early` (Surge right after the S.S. Anne, no Fly to Vermilion) | +21.3 h | Surge fought at Misty-era strength; the WR "fold into the flute leg" is right |
| `safari_after_koga` (Koga first; Safari-dependent evolutions re-anchored) | +17.7 h (+43 h unanchored) | Koga before the Safari catches and candies |
| first X-item trip moved (fly-celadon, scope-and-dig, super-rod, hm02-fly) | -0.02 .. +0.1 h, n.s. (hm02-fly -0.13 CI [-0.38, -0.003], below the 0.3% bar) | money-limited, location-neutral |
| Clefairy buy moved to a Celadon visit / after Warden / after Blaine (2400 or 1700 frames) | +5 h at the Celadon visits (unaffordable, buy skipped), n.s. elsewhere | |

## Closed by the decomp, not simulated

* **Sevii trip 1 after the first Hall of Fame** (merge it into the post-game): the One Island scene `LeaveOneIslandScene` (`OneIsland_PokemonCenter_1F/scripts.inc:437-485`) forces a
  sail back to Cinnabar after the meteorite, and Celio's Ruby request needs the National Dex (`:199-212`) on a later visit. The trip costs the same wherever it sits and its catches
  (Moon Stone evolutions, Hypno, Bond Bridge) would move after the Oak check. No saving to find.
* **Safari before the Snorlax leg**, **Rock Tunnel / Mt. Moon skips**: Snorlax blocks both land approaches to Fuchsia (`docs/glitchless-run/plan.md:66`, `Route12/scripts.inc:16-48`, `Route16/scripts.inc:34`);
  Cerulean is reachable only through Mt. Moon pre-Tea; Lavender only through Rock Tunnel pre-Tea.
* **Sabrina earlier**: the gym has no Silph gate (`SaffronCity_Gym/scripts.inc`), so it is open after the Tea, but at that level it is a death sink (same shape as `erika_cycling`) and it
  needs a Fly back to Celadon for the Cycling Road. Not simulated (a guess, labelled).
* Order of Blaine / Sevii / Sabrina / Erika: Blaine needs Surf (Safari) and the Secret Key; Sevii needs Blaine's Cinnabar; Viridian needs seven badges; any other permutation adds a Fly.
  The WR order already minimises Flys; `warden_early` removes one more.

## Next

The remaining levers are not order: the post-door calibration (see 2026-11-14-22-00), and hazard shape. A descent from 700 over the dex and honest sets has not been rerun.
