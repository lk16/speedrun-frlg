# FRLG glitchless — progression dependency graph (first-pass)

> **Status: FIRST-PASS STRUCTURAL DRAFT.** This is reasoned from general game
> *structure* (mandatory story gates, HM/badge/key-item dependencies), **not**
> from any web route. Every gate marked **⚠️verify** must still be confirmed
> against `decompiled/` (map scripts, warp tables, event flags, HM/badge field
> checks) before we trust it for routing. See the verification checklist at the
> end.
>
> **Scope:** high-level *what-gates-what*, not turn-by-turn. Nodes are
> milestones / required acquisitions; edges are hard dependencies. Where the
> game *allows* ordering freedom we say so — those are the degrees of freedom
> the router gets to optimize.
>
> **End condition** (from `docs/tas-rules.md`): beat the Elite Four **and** the
> Champion → Hall of Fame / credits.

---

## Legend

- **Solid box / arrow** = mandatory milestone and a hard prerequisite edge.
- **HM gate** = a field move whose use is gated by a **badge** (you must own
  *both* the HM and the enabling badge to traverse).
- **Key-item gate** = a specific item unlocks an area/event.
- "**(order-free)**" = the game does not force this relative to its siblings;
  it's a router degree of freedom, constrained only by the gates drawn.

---

## 1. Critical-path spine (the mandatory story line)

This is the linear backbone every glitchless completion must pass through.
Branches and HM/key-item gates are layered on in §2–§4.

```mermaid
flowchart LR
    start([Power-on / new game]) --> starter[Pick starter]
    starter --> rival1[Battle rival #1 - Oak's Lab]
    rival1 --> parcel_get[Get Oak's Parcel - Viridian Mart]
    parcel_get --> parcel_deliver[Deliver Oak's Parcel]
    parcel_deliver --> pokedex[Receive Pokedex + Poke Balls]

    pokedex --> brock[Beat Brock - Boulder Badge]
    brock --> misty[Beat Misty - Cascade Badge]
    misty --> surge[Beat Lt. Surge - Thunder Badge]
    surge --> erika[Beat Erika - Rainbow Badge]
    erika --> koga[Beat Koga - Soul Badge]
    koga --> sabrina[Beat Sabrina - Marsh Badge]
    sabrina --> blaine[Beat Blaine - Volcano Badge]
    blaine --> giovanni_gym[Beat Giovanni - Earth Badge - Viridian Gym]

    giovanni_gym --> badgecheck[Route 23 badge check - all 8 badges]
    badgecheck --> victoryroad[Clear Victory Road]
    victoryroad --> e4[Elite Four - Lorelei, Bruno, Agatha, Lance]
    e4 --> champ[Beat Champion - rival]
    champ --> hof([Hall of Fame / credits = END])
```

> **⚠️verify (gym order) — partly resolved.** The first seven gyms are *not* a
> forced linear sequence; their real constraints are the HM/area gates in §2–§5.
> **But** the decomp pins two hard ordering facts (§5/§6): the **Viridian Gym
> door requires badges 02–07**, so **Brock-or-not aside, Misty/Surge/Erika/Koga/
> Sabrina/Blaine all precede Giovanni (Earth)**; and the **league approach
> checks all 8 badges** (Boulder at the Route 22 gate). So Giovanni/Earth is
> genuinely last, and every other gym is mandatory. Ordering freedom lives among
> gyms 1–7, bounded by the §5 gate graph.

---

## 2. HM acquisition + badge → field-use gating

Each HM needs **(a)** the HM item itself and **(b)** the badge that authorizes
its field use. Traversal that depends on an HM therefore depends on *both*.

```mermaid
flowchart TD
    subgraph Badges [Badges - authorize field use]
        bB[Boulder Badge]
        bC[Cascade Badge]
        bT[Thunder Badge]
        bR[Rainbow Badge]
        bS[Soul Badge]
        bM[Marsh Badge]
        bV[Volcano Badge]
    end

    subgraph HMs [HM items]
        hCut[HM01 Cut]
        hFly[HM02 Fly]
        hSurf[HM03 Surf]
        hStr[HM04 Strength]
        hFlash[HM05 Flash]
        hRS[HM06 Rock Smash]
        hWfall[HM07 Waterfall]
    end

    bB -.authorizes.-> hFlash
    bC -.authorizes.-> hCut
    bT -.authorizes.-> hFly
    bR -.authorizes.-> hStr
    bS -.authorizes.-> hSurf
    bM -.authorizes.-> hRS
    bV -.authorizes.-> hWfall

    hCut -- "needed for" --> useCut[Cut trees - e.g. Vermilion Gym access]
    hSurf -- "needed for" --> useSurf[Water crossings - Routes 19-21, etc.]
    hStr -- "needed for" --> useStr[Boulders - Victory Road, etc.]
    hFlash -- "helps in" --> useFlash[Dark caves - Rock Tunnel / Victory Road]
```

> **✅ CONFIRMED (2026-06-22).** The badge→HM map is exact. `party_menu.c:3926`
> gates field-move use on `FlagGet(FLAG_BADGE01_GET + fieldMove)` over the
> `FIELD_MOVE_*` enum (`include/constants/party_menu.h`): Flash(0)→Boulder,
> **Cut(1)→Cascade**, Fly(2)→Thunder, **Strength(3)→Rainbow**, **Surf(4)→Soul**,
> RockSmash(5)→Marsh, Waterfall(6)→Volcano. (`field_control_avatar.c:605` also
> uses `FLAG_BADGE05_GET` for the auto-Surf prompt — consistent.)
>
> Still open: which HMs are *strictly required* to reach the E4. Confirmed
> required so far: **Cut** (Vermilion gym), **Strength** (Victory Road), **Surf**
> (Cinnabar/Blaine path). **Fly / Flash / Rock Smash / Waterfall** still look
> optional — confirm individually.

---

## 3. Key-item gates (area/event unlocks)

```mermaid
flowchart LR
    parcel[Oak's Parcel] --> dex[Pokedex unlock]

    bill[Help Bill - Route 25] --> ticket[S.S. Ticket]
    ticket --> ssanne[Board S.S. Anne]
    ssanne --> rival_ss[Rival battle - S.S. Anne]
    ssanne --> getcut[Get HM01 Cut - Captain]

    silphscope[Silph Scope - Rocket Hideout] --> tower[Clear Pokemon Tower ghost]
    tower --> fuji[Rescue Mr. Fuji]
    fuji --> flute[Poke Flute]
    flute --> snorlax[Wake Snorlax - Routes 12/16]

    tea[Tea - Celadon] --> guards[Pass Saffron gatehouse guards]
    guards --> saffron[Enter Saffron City]
    saffron --> silphco[Clear Silph Co.]
    silphco --> giovanni2[Beat Giovanni #2]

    goldteeth[Gold Teeth - Safari Zone] --> warden[Warden gives HM04 Strength]
    safari[Safari Zone] --> getsurf[Get HM03 Surf - Secret House]

    secretkey[Secret Key - Pokemon Mansion] --> cinnabargym[Open Cinnabar Gym]
```

> **✅ CONFIRMED (2026-06-22)** — all givers/gates verified (see §6 for files):
> - **Tea** → obtained in `CeladonCity_Condominiums_1F`; checked at all **four**
>   Saffron gatehouses (Route 5 South / 6 North / 7 East / 8 West entrances).
> - **Surf (HM03)** → `SafariZone_SecretHouse`. **Strength (HM04)** → Warden
>   trades it for **Gold Teeth** (`FuchsiaCity_WardensHouse`).
> - **Silph Scope** → `RocketHideout_B4F`; hard gate for the Pokémon Tower
>   Marowak ghost (`StartMarowakBattle` checks `ITEM_SILPH_SCOPE`).
> - **Poké Flute** → Mr. Fuji in `LavenderTown_VolunteerPokemonHouse`.
> - **Secret Key** → found in `PokemonMansion_B1F`; unlocks the Cinnabar Gym
>   door (`CinnabarIsland` checks `FLAG_HIDE_POKEMON_MANSION_B1F_SECRET_KEY`).
> - **S.S. Ticket** → Bill in `Route25_SeaCottage` (no badge gate).

---

## 4. Geographic / traversal gating (which gate unlocks which region)

```mermaid
flowchart TD
    pallet[Pallet Town] --> route1 --> viridian[Viridian City]
    viridian --> route2 --> forest[Viridian Forest] --> pewter[Pewter City]
    pewter --> route3 --> mtmoon[Mt. Moon] --> route4 --> cerulean[Cerulean City]

    cerulean --> nugget[Nugget Bridge / Route 25] --> bills[Bill's house]
    cerulean --> route5 --> path1[Underground Path] --> route6 --> vermilion[Vermilion City]

    vermilion -->|S.S. Ticket| ssanne[S.S. Anne -> Cut]
    vermilion -->|Cut + Cascade| vgym[Vermilion Gym - Surge]

    vermilion --> route11 --> diglett[Diglett's Cave]
    cerulean --> route9 --> route10 --> rocktunnel[Rock Tunnel] --> lavender[Lavender Town]

    lavender --> route8 --> saffrongate[Saffron gatehouse]
    celadon[Celadon City] -->|Tea| saffrongate -->|Tea| saffron[Saffron City]
    lavender --> route7 --> celadon

    celadon --> gamecorner[Game Corner -> Rocket Hideout] --> silphscope2[Silph Scope]
    silphscope2 --> tower[Pokemon Tower] --> flute2[Poke Flute]

    flute2 -->|wake Snorlax| route12[Route 12] --> route13 --> route14 --> route15 --> fuchsia[Fuchsia City]
    fuchsia --> safari[Safari Zone -> Surf, Gold Teeth -> Strength]

    fuchsia -->|Surf| route19[Routes 19-21] --> cinnabar[Cinnabar Island]
    cinnabar --> mansion[Pokemon Mansion -> Secret Key] --> cgym[Cinnabar Gym - Blaine]
    cinnabar -->|Surf| pallet

    saffron --> silphco[Silph Co.] --> giovanni_unlock[Viridian Gym opens]
    giovanni_unlock --> vgym8[Viridian Gym - Giovanni - Earth Badge]

    vgym8 --> route22 --> route23[Route 23 badge check] --> vroad[Victory Road] --> indigo[Indigo Plateau / E4]
```

> **⚠️verify (traversal):**
> - Whether **Flash** is required (vs. optional) for **Rock Tunnel** and
>   **Victory Road**.
> - **Victory Road** internal requirements: confirm it needs **Strength**
>   (boulders) and whether **Surf** and/or other HMs are required inside.
> - Confirm the **Snorlax** blocks (Routes 12 *and* 16) and that at least one
>   must be cleared for the southern loop.
> - Confirm **Cut** is the hard gate for **Vermilion Gym** entry.
> - Confirm **Surf** is the only legitimate (glitchless) way Fuchsia→Cinnabar
>   and Cinnabar→Pallet.

---

## 5. Combined hard-dependency graph (the routing-relevant DAG)

This collapses §1–§4 into just the **hard gates** that constrain ordering —
this is the graph the router actually reasons over. Anything not connected here
is order-free.

```mermaid
flowchart TD
    %% --- intro spine ---
    starter[Starter] --> rival1[Rival #1]
    rival1 --> parcel[Oak's Parcel -> Pokedex]
    parcel --> brock[Brock / Boulder]
    brock --> cerulean[Reach Cerulean]

    %% --- Cerulean: Misty and Bill both need ONLY Cerulean.
    %%     (Cascade is NOT required to get the S.S. Ticket from Bill - VERIFY) ---
    cerulean --> misty[Misty / Cascade]
    cerulean --> bill[Bill -> S.S. Ticket]

    %% --- Cut becomes usable: needs HM01 (S.S. Anne) AND Cascade to authorize ---
    bill --> ssanne[S.S. Anne -> HM01 Cut]
    misty --> cascade_auth{{Cascade authorizes Cut}}
    ssanne --> cutok[Cut usable]
    cascade_auth --> cutok

    %% --- After Cut: Surge gym + Celadon (Thunder NOT required for Celadon) ---
    cutok --> surge[Surge / Thunder - gym behind Cut tree]
    cutok --> celadon_access[Reach Celadon]

    %% --- Celadon branch ---
    celadon_access --> erika[Erika / Rainbow]
    celadon_access --> tea[Tea]
    celadon_access --> rockethide[Rocket Hideout -> Silph Scope]
    rockethide --> tower[Pokemon Tower -> Poke Flute]

    %% --- Saffron via Tea ---
    tea --> saffron[Saffron + Silph Co -> Giovanni #2]
    saffron --> sabrina[Sabrina / Marsh]

    %% --- South to Fuchsia (Snorlax woken by Poke Flute - may be reachable earlier, VERIFY) ---
    tower --> flute[Poke Flute -> wake Snorlax]
    flute --> fuchsia[Reach Fuchsia]
    fuchsia --> safari[Safari Zone -> HM03 Surf + Gold Teeth -> HM04 Strength]
    fuchsia --> koga[Koga / Soul]

    %% --- Surf + Strength become usable (badge auth + HM item, grouped near use) ---
    erika --> strength_auth{{Rainbow authorizes Strength}}
    koga --> surf_auth{{Soul authorizes Surf}}
    safari --> str_item[Strength item]
    safari --> surf_item[Surf item]
    strength_auth --> strok[Strength usable]
    str_item --> strok
    surf_item --> surfok[Surf usable]
    surf_auth --> surfok

    %% --- Cinnabar (needs Surf) ---
    surfok --> cinnabar[Reach Cinnabar -> Secret Key -> Blaine / Volcano]
    cinnabar --> blaine[Blaine / Volcano]

    %% --- Viridian Gym DOOR requires badges 2-7 (Cascade..Volcano) - CONFIRMED in decomp.
    %%     So all six middle gyms must precede Giovanni / Earth. ---
    misty --> viridian_door[Viridian Gym door - needs badges 2-7]
    surge --> viridian_door
    erika --> viridian_door
    koga --> viridian_door
    sabrina --> viridian_door
    blaine --> viridian_door
    viridian_door --> giovanni8[Giovanni / Earth]

    %% --- Endgame: league gate checks ALL 8 badges
    %%     (Boulder at Route 22 gate, Cascade..Earth at Route 23) ---
    giovanni8 --> badgecheck[League gate - all 8 badges]
    strok --> vroad[Victory Road - Strength]
    badgecheck --> vroad
    vroad --> e4[Elite Four]
    e4 --> champ[Champion]
    champ --> END([Hall of Fame])
```

> **✅ Decomp-verified (2026-06-22).** §5 gates checked against `decompiled/`.
> Citations in §6. Results:
> 1. **Cut → Surge's gym** — ✅ HM01 from `SSAnne_CaptainsOffice`; Cut gated by
>    **Cascade** (badge 02, see §2); a `EventScript_CutTree` sits in
>    `VermilionCity` blocking the gym approach.
> 2. **Silph Scope → Tower → Poké Flute → Snorlax** — ✅ `StartMarowakBattle`
>    requires `ITEM_SILPH_SCOPE`; Snorlax (Routes 12 **and** 16) requires
>    `FLAG_GOT_POKE_FLUTE`. Hard chain confirmed.
> 3. **Surf/Soul → Cinnabar; Strength via Gold Teeth/Warden** — ✅ HM03 Surf in
>    `SafariZone_SecretHouse`; Cinnabar gym door unlocks on the **Secret Key**
>    flag (`FLAG_HIDE_POKEMON_MANSION_B1F_SECRET_KEY`); Warden trades **HM04
>    Strength** for **Gold Teeth**.
> 4. **Viridian Gym (Earth)** — ⚠️ **CORRECTED.** Not a "Silph Co. flag" — the
>    door (`ViridianCity_EventScript_TryUnlockGym`) hard-requires **badges
>    02–07** (Cascade, Thunder, Rainbow, Soul, Marsh, Volcano). So **all six
>    middle gyms precede Giovanni/Earth** (graph updated). It's *indirectly*
>    after Silph only because Marsh/Sabrina needs Saffron freed.
> 5. **Victory Road HMs** — ✅ **Strength** confirmed (multiple
>    `EventScript_StrengthBoulder` on all 3 floors). **Surf inside VR not
>    found** — the old `surfok → vroad` edge was removed; Surf stays critical
>    anyway via Blaine/Cinnabar. (A water section inside VR is still worth a
>    closer look.)
> 6. **Bill / S.S. Ticket** — ✅ **Cerulean-only.** Gated solely on
>    `FLAG_HELPED_BILL_IN_SEA_COTTAGE`; **no badge check** — Cascade not required.
> 7. **Fuchsia reachability** — ✅ **Poké Flute is the hard gate.** Fuchsia's
>    only connections are Route 19 (water/Surf), 18 (Cycling Road) and 15; both
>    land approaches route through a Snorlax (Routes 12/16), and Surf (HM03) is
>    unobtainable until *inside* Fuchsia's Safari Zone. No earlier approach.
>
> **Bonus finding:** the **league approach checks all 8 badges**, split across
> two gates — **Boulder** at `Route22_NorthEntrance`, **Cascade…Earth** at
> `Route23` (sequential per-badge guards keyed by `VAR_MAP_SCENE_ROUTE23`). So
> Brock/Boulder stays mandatory even though the Viridian door doesn't check it.

---

## Verification checklist (against `decompiled/`)

- [x] Badge → HM field-use authorization map — `party_menu.c:3926`.
- [~] Which HMs are **strictly required** to reach the E4 — Cut/Strength/Surf
      confirmed required; Fly/Flash/Rock Smash/Waterfall still to disprove.
- [x] Gym entry gates — Cut tree at Vermilion; Secret Key at Cinnabar; **Viridian
      door = badges 02–07** (NOT a Silph flag — corrected).
- [x] Key-item givers & flags — S.S. Ticket (Bill), Silph Scope (Rocket Hideout
      B4F), Poké Flute (Mr. Fuji), Tea (Celadon Condominiums), Gold Teeth →
      Strength (Warden), Secret Key (Pokémon Mansion B1F), Surf (Safari Zone).
- [x] Snorlax block flags (Routes 12 & 16) → `FLAG_GOT_POKE_FLUTE`.
- [x] Saffron gatehouse guard gate (Tea) — four entrances confirmed.
- [x] League badge gate — **all 8** (Boulder @ Route 22 gate, Cascade…Earth @
      Route 23). Victory Road traversal → **Strength** (boulders); Surf-inside
      not found.
- [ ] Exact final-input endpoint at the Champion→Hall-of-Fame transition
      (from scripts, per `tas-rules.md`).
- [ ] FireRed vs LeafGreen differences in any of the above (should be none for
      structure, but confirm version-exclusive item/encounter givers).

---

## 6. Decomp source citations

All paths relative to `decompiled/`. Verified 2026-06-22.

| Gate / fact | Source |
|---|---|
| Badge→HM field-use map | `src/party_menu.c:3926` (`FLAG_BADGE01_GET + fieldMove`); enum `include/constants/party_menu.h:34-41` |
| Surf auto-prompt badge | `src/field_control_avatar.c:605` (`FLAG_BADGE05_GET`) |
| Badge flag constants | `include/constants/flags.h:1364-1372` |
| S.S. Ticket (Bill, no badge) | `data/maps/Route25_SeaCottage/scripts.inc` (`FLAG_HELPED_BILL_IN_SEA_COTTAGE`) |
| HM01 Cut giver | `data/maps/SSAnne_CaptainsOffice/scripts.inc` |
| Vermilion Cut tree | `data/maps/VermilionCity/map.json` (`EventScript_CutTree`, ~tile 19,24; gym warp 14,25) |
| Silph Scope giver | `data/maps/RocketHideout_B4F/scripts.inc` |
| Marowak ghost needs Silph Scope | `src/battle_setup.c:221,320` (`StartMarowakBattle`, `CheckSilphScopeInPokemonTower`) |
| Poké Flute giver | `data/maps/LavenderTown_VolunteerPokemonHouse/scripts.inc` |
| Snorlax needs Poké Flute | `data/maps/Route12/scripts.inc:16`, `data/maps/Route16/scripts.inc:34` |
| HM03 Surf giver | `data/maps/SafariZone_SecretHouse/scripts.inc` |
| Gold Teeth → HM04 Strength | `data/maps/FuchsiaCity_WardensHouse/scripts.inc:7,26` |
| Secret Key → Cinnabar gym | `data/maps/CinnabarIsland/scripts.inc:40-44` (`FLAG_HIDE_POKEMON_MANSION_B1F_SECRET_KEY`); item in `data/maps/PokemonMansion_B1F/map.json` |
| Viridian door = badges 02–07 | `data/maps/ViridianCity/scripts.inc:29-39` (`TryUnlockGym`) |
| Giovanni/Earth = badge 08 | `data/maps/ViridianCity_Gym/scripts.inc:20` |
| Tea gate (4 gatehouses) | `data/maps/Route{5_South,6_North,7_East,8_West}Entrance/scripts.inc`; Tea from `data/maps/CeladonCity_Condominiums_1F/scripts.inc` |
| Victory Road Strength boulders | `data/maps/VictoryRoad_{1F,2F,3F}/map.json` (`EventScript_StrengthBoulder`) |
| League badge gates (all 8) | Boulder: `data/maps/Route22_NorthEntrance/scripts.inc:4`; Cascade…Earth: `data/maps/Route23/scripts.inc`; logic `data/scripts/route23.inc:46-77` |
| Fuchsia connections | `data/maps/FuchsiaCity/map.json` (Route 19 down / 18 left / 15 right) |
