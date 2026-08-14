# Full glitchless run: high-level plan (first pass)

> **Status: STRUCTURAL PLANNING DRAFT**, derived by re-deriving and extending the dependency
> graph from `dbaf8e9`/`1ff1b2a` (deleted in `44d099a` before `rival-1` existed; recovered from
> git history and re-verified against the current decomp rather than trusted as-is — every
> citation below was checked fresh against `decompiled/` in this session, not copied). This
> covers target 3 from `README.md`: power-on to Hall of Fame. Nothing here is a route yet — no
> segment has been built, measured, or tier-1 verified. This is the scaffolding `rival-1` and
> `defeat-brock` had before their first `frlg route build`: what gates what, which choices are
> real degrees of freedom, and which numbers (version, starter, exact HM order) have to be
> *measured* rather than decided from this doc alone. Where a claim can't be backed by a path in
> `decompiled/`, it's labelled a guess.
>
> **No code changes were made producing this document** — pure decomp research.
>
> **2026-08-14 correction pass**: gym-order wording fixed; Flash evidence corrected
> (`requires_flash` exists, is render-only); Rock Smash resolved non-mandatory by collision BFS;
> starter×HM learnset audit added (solo run impossible); §3 rebuilt against the real collision
> map and engine sight rules — Rock Tunnel is not dodgeable, and the Sammy / Cerulean rival /
> Dig grunt / Tower / Hideout / Silph-7F / Snorlax forced battles plus the S.S. Ticket, Poké
> Flute and Tea gate chains were added. §8 tracks what stayed open.

---

## 1. The critical-path spine

The mandatory story backbone, unchanged in shape from the original dependency-graph draft and
reconfirmed this session:

```
start → starter → rival #1 (Oak's Lab) → Oak's Parcel (get + deliver) → Pokédex
      → Brock/Boulder → [Misty, Surge, Erika, Koga, Sabrina, Blaine — order-free, see §2]
      → Giovanni/Earth (Viridian Gym) → League gate (all 8 badges) → Victory Road
      → Elite Four (Lorelei, Bruno, Agatha, Lance) → Champion (rival) → Hall of Fame
```

Two hard facts bound the "order-free" middle six gyms, both reconfirmed fresh this session
(the original citations still hold on the current decomp revision):

- **The Viridian Gym door requires badges 02–07** (Cascade through Volcano) —
  `ViridianCity_EventScript_TryUnlockGym`, `decompiled/data/maps/ViridianCity/scripts.inc:29-35`
  (six `goto_if_unset FLAG_BADGE0{2..7}_GET` checks). So Giovanni/Earth is unconditionally last
  among the eight gyms.
- **The League approach checks all 8 badges**, split across two gates: Boulder at
  `Route22_NorthEntrance_EventScript_BoulderBadgeGuard`
  (`decompiled/data/maps/Route22_NorthEntrance/scripts.inc:4`), and Cascade through Earth
  sequentially at `decompiled/data/scripts/route23.inc:65-95` (`goto_if_set FLAG_BADGE0{2..8}_GET`
  chain). So **Brock/Boulder is mandatory even though the Viridian door doesn't check it** — it's
  checked at the opposite end of the map, on the way to Victory Road.

Everything between Brock and Giovanni (which of the middle six gyms is fought second, third, ...
seventh — Boulder is always first and Earth always eighth) is a genuine router degree of freedom,
constrained only by the item/HM gates in §2 — e.g. Surge's gym needs Cut (behind Cascade), Koga's
city hosts the only Surf/Strength source, etc. This is where most of the frame-count optimization
work will live, the same way `defeat-brock` found its gains in path shape and RNG streams rather
than in "what to do."

Three mandatory sub-chains hang off the spine, verified this session (each link is a script
gate, not a guess):

- **S.S. Ticket → Cut**: the Vermilion dock guard checks `FLAG_GOT_SS_TICKET`
  (`decompiled/data/maps/VermilionCity/scripts.inc:197`); the ticket is given only by Bill at his
  Route 25 cottage (`decompiled/data/maps/Route25_SeaCottage/scripts.inc:99-101`). So Route
  24/25 — Nugget Bridge *and* the Bill detour — is on the critical path, not optional.
- **Poké Flute → Fuchsia**: Snorlax bodies block both land approaches to Fuchsia — Route 12
  (`decompiled/data/maps/Route12/scripts.inc:16-48`) and Route 16
  (`decompiled/data/maps/Route16/scripts.inc:34`) — each behind a `FLAG_GOT_POKE_FLUTE` check
  (waking one starts a wild `SPECIES_SNORLAX` battle; only one of the two needs waking). The
  flute comes from Mr. Fuji (`decompiled/data/maps/LavenderTown_VolunteerPokemonHouse/scripts.inc:7-11`),
  rescued at the top of Pokémon Tower; the tower's Marowak ghost is unbeatable without the Silph
  Scope (`BATTLE_TYPE_GHOST` without the item — `decompiled/src/battle_setup.c:320-337`, gate at
  `:221-233`), and the Scope only drops from Giovanni in the Rocket Hideout
  (`decompiled/data/maps/RocketHideout_B4F/scripts.inc:18-43`), whose entrance stairs only
  appear after the Game Corner poster grunt
  (`decompiled/data/maps/CeladonCity_GameCorner/scripts.inc:400-415`). Net: Celadon → Hideout →
  Lavender → Fuchsia is a forced ordering layer on top of the badge graph.
- **Tea → Saffron**: every Saffron gate guard blocks entry until `FLAG_GOT_TEA`
  (e.g. `decompiled/data/maps/Route5_SouthEntrance/scripts.inc:25-40`); the Tea is given in the
  Celadon Condominiums (`decompiled/data/maps/CeladonCity_Condominiums_1F/scripts.inc:12`). Since
  Sabrina's badge is mandatory (§1) and her gym is in Saffron, Celadon-before-Saffron is forced.
  (The Underground Paths Route 5↔6 and 7↔8 bypass Saffron for *transit*, so the Tea gates only
  Saffron itself, not Vermilion or Lavender.)

---

## 2. HM gates: which are load-bearing, freshly re-checked

Field-move authorization is a single flag check independent of the badge's own gym:
`FlagGet(FLAG_BADGE01_GET + fieldMove)` over the `FIELD_MOVE_*` enum,
`decompiled/src/party_menu.c:3925-3927`, enum in
`decompiled/include/constants/party_menu.h:34-42` — Flash(0)→Boulder, Cut(1)→Cascade,
Fly(2)→Thunder, Strength(3)→Rainbow, Surf(4)→Soul, RockSmash(5)→Marsh, Waterfall(6)→Volcano.
**But the badge only authorizes use — the move must also be taught to a living party member**,
confirmed at three independent code paths (auto-Surf: `PartyHasMonWithSurf`,
`decompiled/src/field_player_avatar.c:1184`; field-move menu:
`decompiled/src/party_menu.c:2963-2980`; obstacle scripts: `checkpartymove`,
`decompiled/src/scrcmd.c:1777-1793`). There is no bag-only HM use anywhere in `item_use.c`.

Of the seven HMs, this session confirmed which are actually required to reach the Elite Four:

| HM | Required? | Evidence |
| --- | --- | --- |
| **Cut** | **Yes** | Blocks the approach to Surge's gym: `EventScript_CutTree` object in `decompiled/data/maps/VermilionCity/map.json`; giver `decompiled/data/maps/SSAnne_CaptainsOffice/scripts.inc:18` |
| **Surf** | **Yes** | Cinnabar Island's only connections are water routes — `decompiled/data/maps/CinnabarIsland/map.json:16-23` (`MAP_ROUTE21_SOUTH`, `MAP_ROUTE20`), no land connection exists; giver `decompiled/data/maps/SafariZone_SecretHouse/scripts.inc:9-11` |
| **Strength** | **Yes** | Boulder puzzles gate Victory Road's floors: `EventScript_StrengthBoulder` objects in `decompiled/data/maps/VictoryRoad_{1F,2F,3F}/map.json`; item is a Gold Teeth trade at `decompiled/data/maps/FuchsiaCity_WardensHouse/scripts.inc:26`, Gold Teeth itself found in the Safari Zone |
| **Flash** | **No** — confirmed, not just unproven | The field exists and is set: `"requires_flash": true` in `decompiled/data/maps/RockTunnel_{1F,B1F}/map.json` (nowhere else — Victory Road doesn't set it). But its only consumers are `SetDefaultFlashLevel` (`decompiled/src/overworld.c:956-964`, sets the darkness radius on map load) and the Flash field-move availability check (`decompiled/src/fldeff_flash.c:164-172`). Neither touches collision — the darkness is a scanline-window rendering effect (`decompiled/src/field_screen_effect.c:90-100`), not a passability gate. (An earlier revision of this row claimed the field didn't exist at all; the conclusion stands, the evidence was wrong.) |
| **Waterfall** | **No** — confirmed unused in the base game | `EventScript_Waterfall` exists as a template in `decompiled/data/scripts/field_moves.inc:178-195`, but **zero** maps place an object using it (`grep -rln "EventScript_Waterfall\|checkpartymove MOVE_WATERFALL" decompiled/data/maps/` → no hits). HM07 itself is only given at `decompiled/data/maps/FourIsland_IcefallCave_1F/map.json`, in the postgame Sevii Islands — off the glitchless-completion path entirely. |
| **Fly** | **No (but valuable)** | Never gates a mandatory transition — Cinnabar is Surf-reachable (above), and no other required area is fly-only. It's purely a backtrack-time optimization, see §5. |
| **Rock Smash** | **No — resolved** | Every confirmed use is in optional/postgame areas (Cerulean Cave, Sevii Islands, Sevault Canyon; `grep -rln "EventScript_RockSmash" decompiled/data/maps/`) except **`RockTunnel_B1F`'s 15 boulders**. A BFS over the collision bits of `decompiled/data/layouts/RockTunnel_{1F,B1F}/map.bin` (bits 10-11 of each u16 block; format per `decompiled/src/fieldmap.c:61-72` and `decompiled/include/fieldmap.h`), with every boulder and NPC object left in place as a blocker, walks the full mandatory chain — Route 10 north → 1F(17,2)→(45,2) → B1F(38,28)→(33,3) → 1F(4,2)→(20,13) → B1F(27,12)→(2,3) → 1F(45,21)→(18,37) → Route 10 south — at 46+88+32+42+44 tiles per leg. No boulder is load-bearing; the rocks only gate shortcuts/items. |

**Net: exactly 3 HMs are hard-required (Cut, Surf, Strength).** This is the number that drives
the move-slot analysis in §4 — but *which mon carries which HM* is no longer free, see below.

### Starters cannot solo the HM set

From `decompiled/src/data/pokemon/tmhm_learnsets.h` (Bulbasaur line `:11-70`, Charmander line
`:74-150`, Squirtle line `:151-230`):

| Line | Cut | Surf | Strength | Rock Smash | Flash | Fly |
| --- | --- | --- | --- | --- | --- | --- |
| Bulbasaur/Ivysaur/Venusaur | yes | **no** | yes | yes | yes | no |
| Charmander/Charmeleon/Charizard | yes | **no** | yes | yes | no | Charizard only |
| Squirtle/Wartortle/Blastoise | **no** | yes | yes | yes | no | no |

**No starter line learns all three mandatory HMs — a strict solo-starter run is mechanically
impossible**, independent of any frame count: the grass/fire starters can never Surf, and the
water starter can never Cut. The minimum party is 2, and the second mon's learnset (plus its
catch/level-up cost) is now a routing variable, not an optimization afterthought.

Also worth pricing in: the HMs are not dead slots. From `decompiled/src/data/battle_moves.h` —
Cut 50 power/95 acc Normal (`:198`), Surf 95/100 Water (`:744`), Strength 80/100 Normal
(`:913`), Rock Smash 20/100 Fighting with a defense-drop chance (`:3240`); only Flash (`:1927`)
is a pure status move. Surf on the Squirtle line is a primary attack, and Strength is a
respectable one on anything — "HM slave" undersells what these slots do in battle.

---

## 3. Trainers: mandatory gauntlets vs. dodgeable

The engine has exactly two `trainer_type` values in use across the whole game —
`TRAINER_TYPE_NONE` (1198 uses) and `TRAINER_TYPE_NORMAL` (432 uses); the omnidirectional-sight
type `TRAINER_TYPE_SEE_ALL_DIRECTIONS` is declared (`decompiled/include/constants/trainer_types.h:6-7`)
but **never placed on any map**. So nothing is "mandatory" by an unavoidable-sight flag — every
forced fight below is forced by *geometry* (a trainer sitting in the only walkable column of a
corridor/bridge) or by a *scripted trigger* (a `coord_events` warp-in-front-of-camera cutscene),
not by an aggressive sight radius.

**Mandatory** (chokepoint or scripted trigger, no way to route around):

| Where | Why unavoidable | Citation |
| --- | --- | --- |
| Nugget Bridge, Route 24 (5 trainers) | Trainers alternate the bridge's only two walkable columns (x=10/x=12) the full length of a 1-lane bridge | `decompiled/data/maps/Route24/map.json` object_events |
| S.S. Anne rival, `SSAnne_2F_Corridor` | Forced `coord_events` trigger sits directly in front of the only warp to the Captain's Office (needed for Cut) | `decompiled/data/maps/SSAnne_2F_Corridor/map.json`, `.../scripts.inc:4-79` |
| Pokémon Tower 7F, 3 Rocket grunts guarding Mr. Fuji | Span the sole column between the entry warp and Fuji | `decompiled/data/maps/PokemonTower_7F/map.json`, `.../scripts.inc:4-33` |
| Giovanni, `SilphCo_11F` | Forced trigger line blocks the only path in the room; defeat sets `FLAG_HIDE_SAFFRON_ROCKETS`, clearing 8 non-battle roadblock NPCs in Saffron | `decompiled/data/maps/SilphCo_11F/scripts.inc:46-77`; consumed at `decompiled/data/maps/SaffronCity/map.json:52,66,80,94,108,123,137,151` |
| "Late Rival," Route 22 (post-badges) | Same forced-trigger pattern on the only Viridian↔Route 23 path — BFS: gate door unreachable from the Viridian side avoiding trigger tiles (33,4-6) | `decompiled/data/maps/Route22/map.json`, `.../scripts.inc:155-220` |
| Bug Catcher Sammy, Viridian Forest | The only S↔N corridor is 5 tiles wide (x=3-7 at y=22); Sammy's body covers x=7 and his 4-tile leftward sight cone covers x=3-6 — BFS finds no path dodging him | object in `decompiled/data/maps/ViridianForest/map.json` (`FACE_LEFT`, sight 4); corridor from `decompiled/data/layouts/ViridianForest/map.bin` |
| Rival, Cerulean City (pre-Nugget Bridge) | Trigger tiles (22-24, y=6) span the only walkable columns to the Route 24 edge | `decompiled/data/maps/CeruleanCity/map.json` coord_events, `.../scripts.inc:16-56` |
| Dig grunt, Cerulean City | The only path to Route 9 runs through the burgled house's back door; grunt body (33,6) + triggers (33,5/7) close the backyard passage. Defeat hands over **TM28 Dig** — the §7 tool arrives free, on the critical path | `decompiled/data/maps/CeruleanCity/map.json`, `.../scripts.inc:160-190` |
| ≥7 Rock Tunnel trainers | See the dodgeable-section correction below — Rock Tunnel is **not** cleanly dodgeable | `decompiled/data/maps/RockTunnel_{1F,B1F}/map.json` + layout BFS |
| Rival, Pokémon Tower 2F | Triggers (17,5)/(16,6) block the only 1F-stairs → 3F-stairs path | `decompiled/data/maps/PokemonTower_2F/map.json`, `.../scripts.inc:8-16` |
| Marowak ghost, Pokémon Tower 6F | Triggers (11,15)/(12,16) are the only approach tiles to the 7F stairs at (11,16); the fight is unwinnable without the Silph Scope (`BATTLE_TYPE_GHOST`), making the Rocket Hideout mandatory (§1) | `decompiled/data/maps/PokemonTower_6F/scripts.inc:4-30`; `decompiled/src/battle_setup.c:221-233,320-337` |
| Game Corner poster grunt, Celadon | The Hideout stairs are drawn by `OpenRocketHideout` (setmetatile) only after the poster he guards is checked | `decompiled/data/maps/CeladonCity_GameCorner/scripts.inc:400-415` |
| Rocket Hideout: Lift-Key grunt + Giovanni | B4F is two disconnected regions: the stairs side only reaches Grunt1, whose defeat spawns the Lift Key; Giovanni's side is lift-only. Giovanni's defeat spawns the Silph Scope. (His two flanking grunts have sight 0 and a 2-tile gap between them — dodgeable.) | `decompiled/data/maps/RocketHideout_B4F/scripts.inc:18-70`, `map.json` + layout BFS |
| Rival, Silph Co. 7F | Multi-floor BFS over all Silph stairs/warp-pads/elevator, with the door-barrier tiles from `silphco_doors.inc` closed: 11F Giovanni is unreachable without the Card Key (5F item ball, `decompiled/data/scripts/item_ball_scripts.inc:249-250`), and with it every path still crosses the rival triggers at 7F (2,4)/(2,5). The 7F→11F warp pad is also the *only* entry to Giovanni's side of 11F — the 10F stairs and elevator land in a walled-off region | `decompiled/data/maps/SilphCo_7F/scripts.inc:20-80`, `decompiled/data/scripts/silphco_doors.inc` |
| Wild Snorlax (Route 12 *or* 16) | Waking either roadblock Snorlax starts a wild battle (fled or won — either ends it); one of the two is unavoidable to reach Fuchsia | `decompiled/data/maps/Route12/scripts.inc:16-48`, `Route16/scripts.inc:34` |
| 8 gym leaders | Mandatory by the badge gates in §1, not by geometry | §1 citations |
| Elite Four rooms ×4 + Champion | Each room's entry door stays locked until `FLAG_DEFEATED_<NAME>` is set for that room — no walk-past is possible | `decompiled/data/maps/PokemonLeague_LoreleisRoom/scripts.inc:14-52` (pattern repeats), Champion at `decompiled/data/maps/PokemonLeague_ChampionsRoom/scripts.inc:53-93,124-144` |
| Hall of Fame trigger | `special EnterHallOfFame` fires on room entry immediately after the Champion is beaten | `decompiled/data/maps/PokemonLeague_HallOfFame/scripts.inc:1-40` |

No Pokémon Center or Mart exists in any of the six `PokemonLeague_*` maps — whatever the run
carries into the League gate is what it fights the Champion with; no restock is possible between
Lorelei and the Champion.

**The collision check has now been run** for every entry above: BFS over the layout `map.bin`
collision bits with NPC bodies as blockers and static-facing trainers' sight cones modeled the
way the engine actually checks them — the sight line is walked tile-by-tile and stops at the
first impassable tile (`CheckPathBetweenTrainerAndPlayer`,
`decompiled/src/trainer_see.c:198-231`), so walls block sight. Rotating/wandering trainers are
treated as body-blockers only (a TAS controls their facing RNG). Two model caveats, flagged
honestly: script-driven `setmetatile` barriers (gym puzzles) and NPC wander ranges are not
modeled, so **gym interiors and Victory Road remain unaudited** — see §8.

**Correction — Rock Tunnel is NOT dodgeable.** The previous draft called its ~15 trainers
avoidable from object placement alone. With sight cones on the real collision map, the mandatory
five-leg traversal forces at least 7 fights: Ashton (1F leg 1), Martha + Winston (B1F leg 1),
at least one of a mutually-covering pair on 1F leg 2, Dudley + Sofia (B1F leg 2), and Dana (1F
leg 3) — each "individually forced" meaning BFS finds no path when only that trainer's cone is
blocked. Nugget Bridge's 5 all re-confirm as forced by the same method.

**Still believed dodgeable, now with the caveat that only Rock Tunnel got the full-cone
treatment**: Pokémon Tower 3F–5F Channelers, Silph Co. floors 2–10 (the multi-floor BFS above
reached 11F while treating them as bodies only), S.S. Anne's side rooms (zero trainer objects),
and the open routes (9, 10, 16–21, plus the early Route 22 rival, a separate pre-badges trigger
from the mandatory late one). These should be re-swept with the same tool before any segment is
declared final.

**Unresolved**: whether `FLAG_SYS_GAME_CLEAR` is set exactly where expected — it's set inside
the compiled `EnterHallOfFame` special, not visible from map scripts alone.

---

## 4. Team composition: the party is at least 2, by learnset arithmetic

**A solo-starter run is impossible** — not by an engine rule, but because no starter line covers
Cut+Surf+Strength (§2): the Squirtle line can't learn Cut, the other two can't learn Surf. The
open question is no longer "1 or 2 mons" but **which second mon, caught where, carrying what**.
The candidate pool is constrained: for a Squirtle-line run the second mon needs Cut before
Surge's gym (Cut learners on or near the mandatory path, cross-checking `tmhm_learnsets.h`
against `decompiled/src/data/wild_encounters.json`: Oddish (FR) / Bellsprout (LG) on Routes
24/25 and 5/6, Paras in Mt. Moon, Sandshrew on LG Route 4, Meowth on Routes 5/6, Krabby/Tentacool
surf-fishing spots);
for a Bulbasaur/Charmander-line run it needs Surf (Psyduck/Poliwag/Slowpoke/Tentacool/Krabby/
Lapras — the free level-25 gift Lapras on Silph 7F, `decompiled/data/maps/SilphCo_7F/scripts.inc:118-121`,
sits steps from the mandatory rival fight and learns Surf+Strength with zero catch cost, though
it arrives after Cut is already needed). Which pairing is fastest is a measured
sweep (§8), but the sweep space is "starter × second-mon × catch location", not "solo vs slave".

Engine facts that still matter for the team plan, checked directly:

- The Viridian Old Man catching tutorial (`decompiled/data/maps/ViridianCity/scripts.inc:196-232`,
  `special StartOldManTutorialBattle`) scripts a fake catch of a scripted `SPECIES_WEEDLE` via
  `gBattleTypeFlags = BATTLE_TYPE_OLD_MAN_TUTORIAL` (`decompiled/src/battle_setup.c:301-307`) and
  never touches `gPlayerParty` — no forced catch, no forced second party member.
- `GetMonsStateToDoubles()` returns "can't double battle" whenever `gPlayerPartyCount == 1`
  (`decompiled/src/pokemon.c:3769-3787`), and both the sight-triggered and talk-triggered double
  battle paths bail out gracefully for a 1-mon party (`decompiled/src/trainer_see.c:114-116`;
  `decompiled/data/scripts/trainer_battle.inc:22-38`). The only map using `trainerbattle_double`
  in the whole game is `decompiled/data/maps/VictoryRoad_3F/scripts.inc:52-59` — with one
  Pokémon, that NPC is simply inert.

**HM slots are sticky, whoever carries them.** Teaching an HM fills the first empty of 4
move slots exactly like a TM (`GiveMoveToBoxMon`, `decompiled/src/pokemon.c:2208-2224`); if all
4 are full it routes into the normal "choose a move to forget" flow — except that flow explicitly
**refuses to let you select an HM move to forget** unless it's specifically in
`PSS_MODE_FORGET_MOVE` (`decompiled/src/pokemon_summary_screen.c:3766-3775`, `sHMMoves` list at
`decompiled/src/pokemon.c:1633-1637`). `PSS_MODE_FORGET_MOVE` is reachable from exactly one place
in the entire game: the **Move Deleter, in `FuchsiaCity_House3`**
(`decompiled/data/maps/FuchsiaCity_House3/scripts.inc:4-30`, `special SelectMoveDeleterMove`).

With the party floor now fixed at 2 (learnsets, §2) and exactly 3 mandatory HMs, the slot
arithmetic is milder than the old solo analysis assumed but still real:

- Cut is needed before Surge (one of the earliest of the middle six) and its slot is
  **permanently stuck** on whichever mon learned it until the run reaches the Fuchsia Move
  Deleter — no other NPC or menu can clear an HM move.
- Surf and Strength's items are both picked up *inside Fuchsia itself* (Safari Zone / Warden's
  House), next to the one place that can free a slot — and since the Cut-carrier and the
  Surf-carrier are necessarily *different mons* (§2), the three HMs spread over 8 slots, not 4.
  Rock Smash is confirmed non-mandatory, so the old zero-free-slot scenario is off the table.
- The real remaining question is whether the *attacker* (usually the starter) should carry any
  HM at all — Surf on a Blastoise-line attacker is a primary attack, not slot churn (§2) —
  or whether both non-Cut HMs live on the second mon.

**Which split is faster is a frame count, not a design call — same standard as `rival-1`'s
starter/version pick — and isn't decided by this doc.**

---

## 5. Which game: FireRed or LeafGreen?

A real search (`grep -rn "VERSION_FIRE_RED\|VERSION_LEAF_GREEN" decompiled/src decompiled/data`,
plus a full diff of `decompiled/src/data/wild_encounters.json` and every `FIRERED`/`LEAFGREEN`
conditional in map scripts) found:

- **No difference anywhere in mandatory trainer data.** Rival, gym leaders, Team Rocket, and
  Elite Four/Champion rosters are byte-identical between versions — checked directly in
  `decompiled/src/data/trainer_parties.h`, `trainers.h`, and every `data/maps/*/scripts.inc`
  (only the optional Celadon Game Corner prize list differs by version).
  Any team difference across a run comes from the *starter* choice, not the cartridge version.
- **Wild encounters differ only on Route 4 and Route 22** (land + fishing species swap, e.g.
  Ekans↔Sandshrew) — `decompiled/src/data/wild_encounters.json`. Both routes are on the mandatory
  path but neither species is required for anything; irrelevant unless a future route decides to
  catch there.
- The only previously-known difference, the **rival name preset list**
  (`decompiled/src/oak_speech.c:649-658`), still stands and was the deciding factor in `rival-1`'s
  9658-frame pick (LeafGreen's 3-letter RED needs one wrap-free DOWN vs FireRed's KAZ needing two
  wrapping UPs).
- The ~4-frame LeafGreen boot-speed edge measured in `rival-1` is **not** explained by the title
  screen's cry species (Charizard vs Venusaur) — the post-cry wait is a hardcoded 90-frame counter
  independent of the cry, `decompiled/src/title_screen.c:708-731`. The real cause is still uncited.

**Conclusion: no decomp-derivable reason exists to prefer one version over the other for the
full run beyond what `rival-1` already measured for its own segment** (naming-screen frames).
Since the naming and starter choice both carry through to the whole run, and defeat-brock already
found the optimum can differ once more of the game's objective is in view (it picked FireRed +
Squirtle, contra rival-1's LeafGreen + Bulbasaur), **the version × starter pick for the full run
has to be re-swept against the full run's objective, not assumed from either prior target.**

---

## 6. Pitfalls

**Money.** New-game money is a flat **3000**, `SetMoney(&gSaveBlock1Ptr->money, 3000)` in
`decompiled/src/new_game.c:128`. Trainer prize money is credited automatically with no menu
interaction — `Cmd_getmoneyreward`, `decompiled/src/battle_script_commands.c:5320`, formula
`4 × enemy's last mon's level × gTrainerMoneyTable[class].value` (rival/gym-leader/E4 classes
worth more than common trainers, table at `decompiled/src/battle_main.c:451-460`) — so income
along the mandatory gauntlet is free in frame terms and should keep pace with spending in most
cases. Two confirmed mandatory costs found this session:

- **The Safari Zone entrance fee is a real, mandatory 500-money purchase**, not optional: both
  Surf's item and the Gold Teeth (→ Strength) are inside the paid area. Checked directly —
  `checkmoney 500` / `removemoney 500`,
  `decompiled/data/maps/FuchsiaCity_SafariZone_Entrance/scripts.inc:114-116`. This is the one
  clear "don't run out of money" risk on the mandatory path; a route should budget for it rather
  than discover it short.
- The catching-tutorial Poké Balls are free (`giveitem_msg ... ITEM_POKE_BALL, 5`,
  `decompiled/data/maps/PalletTown_ProfessorOaksLab/scripts.inc:660`), and the Bicycle cannot
  actually be purchased at all — the "buy" branch is dead code that always says "can't afford it";
  the only way to get it is a free voucher trade
  (`decompiled/data/maps/CeruleanCity_BikeShop/scripts.inc:19-40`, with the decomp's own comment
  confirming the price check is unreachable). Neither is a money risk.

**PP.** Pokémon Center healing restores **PP for free, for the whole party**, in the same routine
that restores HP and clears status — `HealPlayerParty`, `decompiled/src/script_pokemon_util.c:17`,
looping `CalculatePPWithBonus` into every move slot with no cost check. Since the route will
already need to heal repeatedly for HP, **PP is not an independent risk except within a single
no-heal stretch** — the one thing worth tracking once real segments exist is whether any specific
no-heal stretch (e.g. a long trainer gauntlet before the next Center) drains a low-PP move the
route depends on. Not exhaustively checked per-gym this session; flag for whichever segment ends
up with the longest no-heal chain.

**Other**: the Safari Zone entry script also hard-blocks entry if the party is full (6/6) with no
box space (`getpartysize` / `IsThereRoomInAnyBoxForMorePokemon`,
`decompiled/data/maps/FuchsiaCity_SafariZone_Entrance/scripts.inc:151-158`) — irrelevant for a
1-2 mon run but worth remembering if the team plan grows. White-out money loss exists
(`ComputeWhiteOutMoneyLoss`, `decompiled/src/overworld.c:249`) but only matters if the plan ever
intentionally or accidentally whites out, which a TAS route shouldn't. The HM move-slot congestion
from §4 is itself a pitfall in the softer sense: teaching an HM too early, or in the wrong order,
can strand the run without a free slot for an attacking move until a Fuchsia round-trip.

---

## 7. Tricks

| Trick | Mechanic | Requirement | Citation |
| --- | --- | --- | --- |
| **Teleport** | Field-usable outdoors (routes/towns, not caves/buildings); warps to `gSaveBlock1Ptr->lastHealLocation` — the last Pokémon Center visited | No badge (`FIELD_MOVE_TELEPORT = 7`, above the badge-gated range in `party_menu.h:41`) | Dispatch: `decompiled/src/party_menu.c:3939-3945`; outdoor-only gate: `Overworld_MapTypeAllowsTeleportAndFly`. Learned at **level 1** by Abra/Kadabra/Alakazam, Claydol, and Ralts — `decompiled/src/data/pokemon/level_up_learnsets.h:843,848,864` (Abra line) |
| **Fly** | Warps to any *visited* town, or the Route 4 / Route 10 Pokémon Centers specifically — never to a dungeon/route interior; opens a full map-select screen, not a 1-button warp | Thunder Badge (`FIELD_MOVE_FLY = 2` → `FLAG_BADGE03_GET`) | `decompiled/src/region_map.c:2952-3002` (fly-target eligibility), `:4023-4036` (landing logic) |
| **Dig** | Same warp routine as Escape Rope — destination is `gSaveBlock1Ptr->escapeWarp`, i.e. **the entrance the player walked into the current dungeon from**, not a Pokémon Center. Reusable (no consumption), any map with `allowEscaping == TRUE` | **TM28, not an HM** — no badge required (`FIELD_MOVE_DIG = 8`, above the badge-gated range) | `decompiled/src/fldeff_dig.c:12-21`; TM id at `decompiled/include/constants/items.h:388` |
| **Escape Rope** | Identical destination/mechanic to Dig, but single-use (consumed, `RemoveUsedItem`) and bought (550, Cerulean/Lavender Marts) or found free on Route 11 | Item, no HM/badge | `decompiled/src/item_use.c:622-647`; price `decompiled/src/data/items.json:1365-1377`; Marts: `decompiled/data/maps/CeruleanCity_Mart/scripts.inc:32`, `LavenderTown_Mart/scripts.inc:36`; free copy `decompiled/data/maps/Route11/map.json:250` |
| **Repel / Super Repel / Max Repel** | Suppresses a wild encounter only if the wild level rolled is *below* the lead party member's level — not a hard block | Item (100/200/250 steps) | Encounter check: `IsWildLevelAllowedByRepel`, `decompiled/src/wild_encounter.c:601-620`; step counter `decompiled/src/item_use.c:567` |

**Priority read for route value**: Teleport is available earliest (an Abra caught and boxed gives
free instant returns to the last Center from anywhere outdoors, well before Thunder Badge), Fly
is the biggest single-trip saver once Surge is beaten (e.g. a Safari Zone/Cinnabar loop back to
the League side of the map) but costs real menu frames and can't reach dungeon interiors, and
Dig/Escape Rope only save a walk *back out* of an already-cleared dungeon to its own front door —
useful, but narrower than Fly/Teleport. Running is not available from the start: `FLAG_SYS_B_DASH`
is only set by the Route 1/Viridian/Pewter Oak's Aide cutscene
(`decompiled/data/maps/PewterCity/scripts.inc:695-724`), so the entire opening stretch through
Route 22's early rival is walk-speed regardless of routing.

**Unverified and explicitly not used in this doc**: whether Route 2 or any other early map has a
Cut-tree shortcut past a dungeon (checked `Route2/scripts.inc`, found none — this rules out the
commonly-remembered Gen-1-style Route-2-to-Diglett's-Cave Cut shortcut *for this decomp*, but
should be re-checked against the actual FRLG map set before being relied on either way).

---

## 8. Open questions — status after the 2026-08-14 session

1. ~~**Rock Smash in `RockTunnel_B1F`**~~ — **RESOLVED, not mandatory.** Collision-map BFS with
   all 15 boulders in place traverses the whole Rock Tunnel chain (§2 table has the per-leg
   distances). Mandatory HM set is exactly Cut/Surf/Strength.
2. ~~**Solo-mon vs. HM-slave**~~ — **RESOLVED in structure, open in detail.** Solo is impossible
   (no starter line learns all three mandatory HMs, §2), so the party floor is 2. Still open and
   frame-count-shaped: *which* second mon, caught where, carrying which HM split (§4).
2b. **Version × starter for the full run** — unchanged, still open; needs its own sweep once
    enough of the route exists to score against. The second-mon choice (2) is now part of the
    same sweep space, and version matters to it (e.g. Route 4 Sandshrew is LeafGreen-only, §5).
3. ~~**Collision check of the §3 gauntlets**~~ — **DONE for every §3 table entry** (method and
   caveats in §3: engine-accurate sight cones per `trainer_see.c:198-231`, BFS on layout
   collision). **Still open**: gym interiors (script-`setmetatile` puzzles defeat the static
   model — Vermilion's gates, Cinnabar's quiz doors, Sabrina's teleporters need per-gym
   modeling), Victory Road, and a full-cone re-sweep of the "believed dodgeable" list.
4. **Exact PP risk windows** — unchanged; needs the segment-by-segment battle plan. Now
   quantifiably worse than the abstract version suggested: Rock Tunnel alone forces ≥7 fights
   (§3) and sits mid-way through the longest plausible no-heal stretches.
5. This document's HM/trainer/version claims should be treated the way `defeat-brock` treats its
   own research phase: correctness first, then segment-by-segment measurement — nothing here is
   a route until it's built and tier-1 verified.
