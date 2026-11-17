# 2026-11-17: is the Power Plant / Pewter detour worth it? (interim)

Base = CURRENT 1040-optstop. Paired (same seeds), R = 800k, seed 101, EHT hours, var - base (positive = the variant is worse).
Variants are patches of 1040's JSON (scratch `pv.py`/`run1.py`); the base's Power Plant hunt is on-route after `powerplant-out`
(fly Lavender 700 f + 260 tiles surf + 400 f), 200 steps. Encounter table: `decompiled/src/data/wild_encounters.json`
sPowerPlant_* (land rate 7; slots 20/20/10/10/10/10/5/5/4/4/1/1: Voltorb, Magnemite, Pikachu x4 slots 40%, Magneton 20% (LG; FR has Electabuzz 2% instead of 2 Magneton slots)).
Old Amber: `PewterCity_Museum_1F/scripts.inc:144-151`, scientist object has flag 0 (`map.json`), so it is available on the first Pewter visit (no story gate).

| variant | WR+2m | WR |
| --- | --- | --- |
| skip Power Plant (fill PP at 20 tiles kept) | +68.4 [+39, +97] | +106.8 [+52, +166] |
| skip PP and drop PP from fill | +187.1 [+146, +234] | +194.2 [+129, +266] |
| PP hunt steps 100 | +12.7 [-4, +30] | -8.8 [-42, +17] |
| PP hunt steps 400 | +25.3 [+11, +40] | +44.4 [+17, +70] |
| PP stop_after 1 / 2 / 3 | +49.2* / +10.9 / +3.4 | +73.8* / -16.6 / -14.7 |
| drop PP from the fill list (route hunt kept) | +2.6 [-1.8, +7.8] | -11.4 [-22, -2] |
| fill PP detour 150 | 0.0 | 0.0 (fill PP is never used while the route hunt runs) |
| no Aerodactyl / museum | +31.1 [+16, +46] | +10.9 [-20, +44] |
| Aerodactyl event 2400 f (3600 base) | -15.2 [-23, -8] | -37.4 [-58, -24] |
| Aerodactyl event 4800 f | +15.5 [+9, +23] | +48.9 [+32, +73] |
| no Diglett's Cave hunt | +55.9 [+38, +75] | +79.0 [+38, +117] |
| no Mr. Mime trade | +26.4 [+12, +41] | -2.1 [-35, +29] |
| no Pewter bundle (Aerodactyl+Diglett+Mime) | +133.3 [+103, +171] | +216.7 [+154, +285] |
| no Pewter bundle and no Power Plant | +278.5 [+213, +347] | +523.6 [+391, +667] |

Reading: both detours pay in the model; the PP hunt shape (200 steps, no stop) is on a plateau. The only candidate that
beats 1040 is a cheaper Pewter bundle (frames), which is a cost assumption, not a route change: the 3600 f event is a guess.
Break-even for the Aerodactyl event is about 6000 f. Caveat: 20-tile fill PP detour makes the skip variants optimistic.
