# 2026-11-30: fishing, Safari length/areas, lead-level Repel hunts

All paired (common seeds), honest-model keys of CURRENT (Rash/Mild, reset time counted, `no_salvage_until`, `ball_batch` 20). Screens R 0.8-1.6M, confirmations R 3.2M on two fresh seeds, WR+2m and WR.

## Result: plan `1051-nofish-west-postfit` (CURRENT)

Parent `1044-postfit-calm`. Three changes:

1. **No fishing at all.** `rod: "none"` and the Route 6 fish fill dropped. The plan paid a Super Rod house trip (`super-rod`, 2400 frames guess, `route.rs`) and used the rod only for one fill entry (Route 6, 23 casts). Dropping the fill alone keeps the trip and is +8 h (the fill is worth a little); dropping both is the win (the trip is paid on every attempt that reaches Snorlax, including those that later reset). The descent never found it because the two knobs are coupled. On 1040: -26.9 / -29.2 h WR+2m, -61 / -72 h WR (R 3.2M, seeds 301/302). Good/Old rod instead of none: -8.8 / -9.6 (screen). This matches the WR streamer, who never fishes.
2. **Second Safari hunt in the West area, not Center.** The route walks to the Secret House in West anyway (`warden_early`, Gold Teeth). On nofish: -18.7 / -15.4 (WR+2m), -26 / -28 (WR).
3. **Drop the Route 10 hunt** (after `route10-north`): a further -21 / -25 (WR+2m), -41 / -51 (WR) vs nofish alone; it was a coin flip on 1040 and is a loss once the trip is cheaper.

Paired against CURRENT 1044, R 3.2M, seeds 321 / 322:
WR+2m 161.7 vs 182.7 h (-21.0, CI [-24.6, -17.1]) and 159.5 vs 185.1 (-25.6, [-29.1, -22.1]);
WR 203.2 vs 249.9 h (-46.7, [-54.2, -40.2]) and 202.7 vs 257.8 (-55.0, [-61.9, -47.8]).
Same changes on 1043-calm (`1050-nofish-west`, seeds 311/312): WR+2m -30.3 / -33.8, WR -69.7 / -68.9. On 1040 (all three): WR+2m -48 / -54, WR -102 / -123.

## Refuted

* **Lead-level Repel on Safari (item 4).** New knob `Hunt.repel_level` (`sim.rs`, `hunt`): a Lv L lead plus a Repel blocks every land encounter below L (`decompiled/src/wild_encounter.c:286,601-622`, `IsWildLevelAllowedByRepel`: first healthy party mon's level, `wildLevel < ourLevel` blocks; fishing is not checked, `:301-307`; no Safari exception in `item_use.c:550-567`, `safari_zone.c:42-53`). Costs: 300 f lead swap, 350/500/700 money and 350 f per Repel (100/200/250 steps). Safari land levels are 22-33 and the wanted species sit at the same levels as the junk (Venonat/Paras 22-23, Chansey 23-26, Pinsir 23-28, Kangaskhan 25-28, Tauros 25-28, Doduo 26, Rhyhorn 25-26; `data/dex.json` `sites_lg`), so any L that removes junk removes wanted species too. R 800k seed 201 WR+2m vs 1040: both areas L22 +15.7, L23 +44.6, L24 +15.9, L25 +149, L26 +260, L28 +402 h; one area only L23 +19..+39, L25 +27..+60. Power Plant (Voltorb/Magnemite 22/25, Pikachu 22-26) is the same shape. The knob stays in the sim (default off).
* **Safari stop count** (`safari_stop` 3/4/6/7/8/0): 3 is +68 h; 4..8 and 0 are within noise (-3.4 .. +2.6, CIs include 0). 5 stays.
* **Safari areas:** East -> North +69, Center -> North +23, adding West +50 or North +94 (stop 8): worse. East -> West -7 (n.s.); Center -> West is the accepted change.
* Dropping any other single hunt or fill entry on nofish (R 1.6M seed 204): Safari East +92, Safari Center +41, Power Plant hunt +84, Bond Bridge ~+20, Diglett +50 are losses; Route 22 / Route 23 hunts and the Power Plant, Route 18, Seafoam B4F, Diglett, Tower 7F fill entries are +-5 (noise).
* Fly destinations / orders (item 1) and larger city reorderings (item 6): route variants were already measured (`scores.jsonl` trv1-6: surge_early +134, safari_after_koga +103, erika_cycling +321, surge+erika +114 h; erika_pre_silph an artifact of windows); no new reordering code was written here.

## Not done / next

* The Repel idea is dead for Safari; a Repel keyed to a *rising* L as species get caught (split the hunt into two visits) could still help marginally; not tested.
* Item 5 (things not in the stream) produced only the coupled-trip pattern above: look for other fixed trips whose only user is one marginal knob (the descent tests knobs one at a time).
