# Full glitchless run: initial route

> **Status: INITIAL ROUTE, UNMEASURED — tier 0.** First end-to-end route for target 3
> (power-on → Hall of Fame), 2026-08-14. Built on `plan.md` (dependency graph),
> `route-sketch.md` / `distances.json` (tile costs), plus this session's own measurements.
> **No segment has been built, emulated, or verified at any tier.** Every decision below is
> "sane starting point", not "measured optimum" — §7 lists what has to be re-swept once
> segments exist. Tile counts are static-BFS floors (`bin/frlg-mapgraph`; battles, dialogue,
> menus, and script barriers excluded — see the tool header for the model caveats).
>
> **One correction to `plan.md` found while routing** (now §1.5 here): the Cinnabar Gym is
> locked behind the **Secret Key**, so **Pokémon Mansion is a mandatory dungeon** —
> `plan.md` §1/§3 missed it.

## 1. Decisions

### 1.1 Version and starter: FireRed, Squirtle

`plan.md` §5 found no decomp-derivable difference in mandatory content between versions
(trainer parties byte-identical; wilds differ only on Routes 4/22). The only *measured*
numbers we own are the two prior targets, and `defeat-brock` — the larger objective —
measured **FireRed + Squirtle** fastest to Brock (38862 frames, tier-2 accepted,
`route/defeat-brock/ledger.json`). Reusing that verified prefix is worth more for an
initial route than re-guessing. **Marked for re-sweep** (§7): the full run's objective is
different again (Misty is water-vs-water, the E4 shape matters), so version × starter must
eventually be scored on the whole run, same as `defeat-brock` re-decided it against
`rival-1`'s pick.

### 1.2 Party and HM split

No starter learns Cut+Surf+Strength (`plan.md` §2, from
`decompiled/src/data/pokemon/tmhm_learnsets.h`), so the party floor is 2. This route runs 4:

| Slot | Mon | Caught | Carries (field) | Why |
| --- | --- | --- | --- | --- |
| 1 | **Squirtle** | starter | **Surf, Strength** | Both are real attacks (Surf 95 power, Strength 80 — `decompiled/src/data/battle_moves.h:744,913`); the Squirtle line learns both plus Dig but never Cut (`tmhm_learnsets.h:151-230`) |
| 2 | **Paras** | Mt Moon B1F — the **only species on that floor** (`decompiled/src/data/wild_encounters.json`, `sMtMoonB1F_FireRed` land slots are all `SPECIES_PARAS`) | **Cut, Dig (TM28)** | The only early-route Cut learner that also learns TM28 Dig (`tmhm_learnsets.h`; Oddish/Bellsprout/Abra do not). TM28 is handed over by the *mandatory* Cerulean dig-grunt fight (`plan.md` §3), and Dig-the-field-move warps to the dungeon entrance (`decompiled/src/fldeff_dig.c:12-21`) — it exits the Rocket Hideout, Pokémon Tower, and Silph Co., all of which set `"allow_escaping": true` (`decompiled/data/maps/{RocketHideout_B4F,PokemonTower_7F,SilphCo_11F}/map.json`) |
| 3 | **Abra** | Route 24/25 grass, on the mandatory Bill leg (`wild_encounters.json`, `sRoute24_FireRed`/`sRoute25_FireRed`) | **Teleport** (innate, L1 — `decompiled/src/data/pokemon/level_up_learnsets.h:843`) | Free warp to the last-visited Center from anywhere outdoors, no badge (`decompiled/src/party_menu.c:3939-3945`); pays twice before Fly exists (§2 legs 6, 9) |
| 4 | **Pidgey** | same grass as Abra (`sRoute24/25` slots) | **Fly (HM02)** | Only cheap Fly learner on the early path (`tmhm_learnsets.h`; Abra/Paras can't). HM02 from the Route 16 house (`decompiled/data/maps/Route16_House/scripts.inc:9-11`), badge-gated on Thunder (`decompiled/src/party_menu.c:3925-3927`). Fly powers the endgame star (§2 legs 13-16) |

3 catches ≤ 5 free tutorial Poké Balls
(`decompiled/data/maps/PalletTown_ProfessorOaksLab/scripts.inc:660`); a TAS manipulates
first-ball catches, so no ball purchases. HM slots: Squirtle 2 (both usable attacks), Paras
2 (pure slave), no HM ever needs deleting — the Move Deleter stays unvisited.

Fallback noted: Oddish (FR Routes 24/25 + 5/6) also learns Cut and shares the Abra/Pidgey
grass, consolidating all catches into one patch — but then nothing cheap carries Dig.
Measure later if Dig turns out not to pay (§7).

### 1.3 Abra: yes, on the Bill leg

Caught in Route 24/25 grass during the mandatory S.S.-Ticket detour (`plan.md` §1), kept in
party. Teleport targets the last Pokémon Center visited, outdoors only (`plan.md` §7).
Planned uses, each ~156-260 saved run-tiles minus one catch (~1 wild battle) and per-use
menuing — to be measured:

- Bill's cottage → Cerulean PC: saves the 156-tile return walk (`distances.json`,
  `cerulean-pc_to_bills-door`).
- Vermilion (post-Surge) → Cerulean PC: saves the 260-tile Route 5 return
  (`cerulean-pc_to_vermilion-pc`) — **requires not entering the Vermilion PC**, so the
  whole Vermilion block (rival, Surge) runs on the Cerulean heal. PP/HP risk flagged in §6.

### 1.4 Gym order

`Brock → Misty → Surge → Erika → Koga → Sabrina → Blaine → Giovanni`

Forced edges (all from `plan.md` §1-§3): Boulder first and Earth eighth (badge-door
checks); Cut (S.S. Anne) before Surge's tree-blocked gym and before Route 9/Rock Tunnel;
Celadon before Lavender's tower (poster grunt → Hideout → Scope → Marowak); flute before
Fuchsia; Fuchsia's Safari before Surf/Strength exist; Tea + Silph clear before Sabrina's
door. The free choices this route makes:

- **Misty immediately** (13 tiles from Cerulean PC) rather than delayed — she's 2 tiles of
  detour and her badge gates Cut use, needed two segments later. No measurable alternative.
- **Erika on the first Celadon visit** — her gym is 86 tiles from the Celadon PC behind an
  already-available Cut tree; any later visit re-pays the trip.
- **Koga before Sabrina before Blaine**: Koga is where the run already is (Fuchsia, for the
  Safari HMs); Sabrina's unlock (Silph) is a Fly-hop away; Blaine needs Surf (Soul badge —
  Koga's) to reach at all, plus the Mansion detour (§1.5).

### 1.5 NEW: the Secret Key / Pokémon Mansion is mandatory

The Cinnabar Gym door only unlocks once the Secret Key item ball has been picked up:
`CinnabarIsland_EventScript_CheckUnlockGym` gates on
`FLAG_HIDE_POKEMON_MANSION_B1F_SECRET_KEY`
(`decompiled/data/maps/CinnabarIsland/scripts.inc:41`), which is the hide-flag of the item
ball at Pokémon Mansion B1F (5,7) (`decompiled/data/maps/PokemonMansion_B1F/map.json`,
`finditem ITEM_SECRET_KEY` at `decompiled/data/scripts/item_ball_scripts.inc:334`).
**`plan.md` §1/§3 missed this dungeon entirely.** The Mansion's interior is
statically unpathable for `bin/frlg-mapgraph` (its switch-statue barriers are
script-`setmetatile`, the same model gap as the gym interiors, `plan.md` §8.3), so its
cost is unknown — it needs either a metatile-script model or emulator work (§7).

### 1.6 Transport plan

Speed eras per `route-sketch.md` §1: walk 16 f/t → Running Shoes (post-Brock, Pewter east
exit) 8 f/t → Bicycle 6 f/t.

- **Bike: yes, early.** Voucher at the Vermilion Fan Club — 30 tiles off the PC
  (`giveitem_msg ... ITEM_BIKE_VOUCHER`,
  `decompiled/data/maps/VermilionCity_PokemonFanClub/scripts.inc:27`), redeemed at the
  Cerulean Bike Shop — 20 tiles off the PC (measured this session) — on the way to Route 9.
  Everything from Route 9 onward is predominantly bike-era; a ~110-tile detour buying 2 f/t
  over thousands of tiles is taken on faith for the initial route, measured later.
- **Fly: yes**, taught after Erika (HM02 house is 109 cut-tiles west of Celadon PC,
  measured). Fly targets used: Lavender (tower), Celadon (return), Pallet (Cinnabar
  approach — Route 20 is severed at Seafoam, `route-sketch.md` §3.9), Viridian (endgame).
- **Snorlax: Route 16 side** — feeds Cycling Road; `route-sketch.md` §2 already showed the
  Celadon/Cycling side beats Routes 12-15 (≈374×6 vs ≈547×8 frames).
- **Dig (field) instead of Escape Ropes** — free with TM28, reusable, same warp
  (`decompiled/src/fldeff_dig.c:12-21`); exits Hideout, Tower, Silph (§1.2).

## 2. The route

Segment numbering will become the ledger's segment list; battles named in **bold** are
mandatory per `plan.md` §3. Tile counts: `distances.json` — legs marked *(new)* were added
to `bin/frlg-mapgraph`'s dump table this session and regenerate with the rest.

| # | Leg | Era | Tiles (floor) | What happens |
| --- | --- | --- | --- | --- |
| 1 | Power-on → Brock beaten | walk | — (measured: 38862 frames as defeat-brock) | The `defeat-brock` accepted route wholesale: FR+Squirtle, naming, **lab rival**, parcel round trip, tutorial (5 free balls), **Sammy**, Brock. Re-decisions deferred (§7) |
| 2 | Pewter → Cerulean | run | 492 | Running Shoes at east exit; Route 3, Mt Moon: **catch Paras on B1F** (100% slot), **Miguel + mandatory fossil pickup** at (13,7), exit Route 4 |
| 3 | Cerulean arrival | run | ~30 | Heal/anchor at Cerulean PC (Teleport target for legs 4/6) |
| 4 | Nugget Bridge → Bill | run | 156 one-way | **Cerulean rival**, **5 bridge trainers**; **catch Abra + Pidgey** in Route 24/25 grass; S.S. Ticket from Bill (`decompiled/data/maps/Route25_SeaCottage/scripts.inc:99-101`); **Teleport → Cerulean PC** |
| 5 | Misty | run | 26 r/t | Cascade badge → Cut usable once taught |
| 6 | Cerulean → Vermilion block | run | 260 + 60 + 60 + 72 | Route 5 underground; Fan Club voucher (30×2 *(new)*); S.S. Anne: **dock rival**, captain → HM01 → **teach Cut to Paras**; Vermilion Gym (Cut tree): Surge → Thunder badge. No Vermilion PC visit; **Teleport → Cerulean PC** |
| 7 | Cerulean → Lavender | bike from shop | 20 *(new)* + 218 | Bike Shop (voucher → Bicycle); **dig-grunt house** (mandatory fight, TM28 → **teach Dig to Paras**); Route 9, Rock Tunnel (**≥7 forced fights**, no Flash needed — render-only, `plan.md` §2); Lavender PC anchor |
| 8 | Lavender → Celadon | bike | 227 | Route 8 underground path; Celadon PC anchor |
| 9 | Celadon block | bike | 40 + ~52 + 172 + 109 *(new)* | Tea (Condominiums); **poster grunt** → Hideout: **Lift-Key grunt**, **Giovanni** → Silph Scope, **Dig out**; Erika (Cut trees) → Rainbow badge → Strength usable later; Route 16 house → HM02 → **teach Fly to Pidgey** |
| 10 | Tower (Fly Lavender) | bike | 15 + interior | **Tower rival (2F)**, **Marowak** (Scope in bag), **3 grunts (7F)**, Fuji → Poké Flute; **Dig out**, Fly → Celadon |
| 11 | Celadon → Fuchsia | bike | 351 *(new)* | Route 16: **wake + fight/flee Snorlax** (`FLAG_GOT_POKE_FLUTE` gate, `decompiled/data/maps/Route16/scripts.inc:34`); Cycling Road; Fuchsia PC anchor |
| 12 | Fuchsia block | run (in Safari) | 70 + ~230 in + ~230 out + 9 + 38 | Safari: 500 fee, 600 steps / 30 balls (`decompiled/src/safari_zone.c:27-33`, ample: ~460 used); Gold Teeth (`SafariZone_West/map.json` (28,14)) + Secret House → HM03 (`decompiled/data/maps/SafariZone_SecretHouse/scripts.inc:11`); Warden → HM04 (`decompiled/data/maps/FuchsiaCity_WardensHouse/scripts.inc:26`); **teach Surf + Strength to Squirtle**; Koga → Soul badge → Surf usable |
| 13 | Silph + Sabrina (Fly Celadon) | bike | 100 *(new)* + interior + 43 | Route 7 gate (Tea); Silph: Card Key (5F ball), **7F rival** (skip the gift Lapras), **11F Giovanni** → Saffron rockets clear; **Dig out**; Sabrina → Marsh badge |
| 14 | Blaine (Fly Pallet) | surf | 146 + interior + 34 | Route 21 south (`pallet_to_cinnabar-pc`); **Pokémon Mansion → Secret Key** (§1.5, cost unknown); Blaine → Volcano badge |
| 15 | Giovanni (Fly Viridian) | bike | ~110 | Viridian Gym (door checks badges 02-07, `decompiled/data/maps/ViridianCity/scripts.inc:29-35`) → Earth badge |
| 16 | League run | bike/surf | 113 + 167 + 155 (floor) | Route 22 (**late rival**), Route 23 badge gates, Victory Road (**Strength puzzles — unmodeled**, `plan.md` §8.3); heal + anchor at `IndigoPlateau_PokemonCenter_1F` (exists — last restock; the six `PokemonLeague_*` maps have none, `plan.md` §3) |
| 17 | E4 + Champion | — | — | **Lorelei, Bruno, Agatha, Lance, Champion**, each door-locked until the previous falls (`decompiled/data/maps/PokemonLeague_LoreleisRoom/scripts.inc:14-52` pattern); Hall of Fame trigger ends the run |

Measured overworld floor: ≈4,900 tiles across eras (heavily bike-weighted), **plus** the
unmeasured interiors (Hideout, Tower, Silph, Mansion, Victory Road, gyms) and every battle,
menu, and text box. This is a shape, not a time estimate.

## 3. Badge → HM-use ordering check

Every HM is authorized before its first field use: Cut taught after Cascade (leg 6 > 5);
Fly taught after Thunder (9 > 6); Strength used in Victory Road after Rainbow (16 > 9);
Surf first used on Route 21 after Soul (14 > 12). Authorization is
`FlagGet(FLAG_BADGE01_GET + fieldMove)` (`decompiled/src/party_menu.c:3925-3927`).

## 4. Key-item chain check

Parcel → Pokédex → (balls) · Ticket → dock · Voucher → bike · Cut → Route 9/gyms ·
TM28 → escapes · poster → Scope → Marowak → Flute → Snorlax · Bicycle → Cycling Road ·
Tea → Saffron · Card Key → 11F · Silph → Sabrina's door · Teeth → HM04, house → HM03 ·
**Secret Key → Blaine** · badges 02-07 → Viridian door · 8 badges → Route 22/23 gates.
Every arrow lands in an earlier leg than its consumer. Citations: §1-2 above + `plan.md`
§1-3.

## 5. Money

Start 3000 (`decompiled/src/new_game.c:128`). Mandatory spend: Safari 500
(`decompiled/data/maps/FuchsiaCity_SafariZone_Entrance/scripts.inc:114-116`). Everything
else is free: 5 tutorial balls cover 3 catches, bike via voucher (purchase path is dead
code, `plan.md` §6), Dig replaces Escape Ropes. Income from ~40+ mandatory fights is
automatic (`plan.md` §6). Balance cannot go negative.

## 6. Known risks in this shape

- **Leg 6 no-heal stretch** (Cerulean heal → rival + Surge + travel): PP/HP viability
  unproven; if it fails, the fallback is healing in Vermilion and eating the 260-tile walk
  back (Teleport anchor moves).
- **Battle viability is entirely unmodeled**: levels, exp routing, and whether
  Squirtle-line + 3 slaves clears Rock Tunnel's ≥7 fights, Silph, and the E4 without grind
  detours. `frlg-battle` exists for exactly this; nothing here is validated against it.
- **Interior costs unknown**: Hideout, Tower, Silph, Mansion (§1.5), Victory Road, all gym
  interiors — the static model can't path them (`plan.md` §8.3).
- Wanderer NPCs, spinners (Rocket bases!), and forced-movement tiles are unmodeled.

## 7. Re-sweep list (what "initial" means here)

1. Version × starter × second-mon on the full-run objective (`plan.md` §8.2b) — this
   route's FR+Squirtle+Paras is inherited evidence, not full-run evidence.
2. Bike voucher detour, Fly pickup, each Teleport use, Dig-vs-walk-out: each is a
   claimed-positive trade taken on floor arithmetic; measure each against its skip.
3. Paras vs Oddish as the Cut slave (consolidated catches vs losing Dig).
4. Gym-order permutations within the forced edges (Erika timing is the loosest).
5. Mansion and Victory Road interior modeling; gym interiors; spinner manipulation.
6. Exp/level plan and per-battle RNG, once segments exist to score against.
