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

> **⚠️verify (gym order):** The gym *badge* numbers above are the conventional
> order, but FRLG does **not** strictly force all of them in this sequence — the
> real constraints are the **HM/area gates** in §2–§4, plus the Earth-Badge gym
> opening only after a mid-game event (Silph Co). The router's true ordering
> freedom is whatever those gates leave open. Treat this column as illustrative
> until the gate graph (§5) is decomp-confirmed.

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

> **⚠️verify (badge↔HM map):** the badge→HM authorization pairs above are the
> standard FRLG mapping but MUST be confirmed against the decomp's field-use
> badge checks (look for the per-HM `BADGE0x_GET` / `FLAG_BADGE0x_GET` checks in
> the field-move scripts). **Fly/Flash/Rock Smash/Waterfall are likely
> *optional*** for a glitchless completion (convenience, not gating) — confirm
> which HMs are *strictly required* to reach the E4 vs. merely useful.

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

> **⚠️verify (FRLG-specific items & givers):**
> - **Tea** is an FRLG-only gate for the Saffron guards (replaces RBY drinks) —
>   confirm the item id, where it's obtained, and which gatehouses it opens.
> - Confirm **Surf** location (Safari Zone Secret House) and that **Strength**
>   comes from the **Warden** in exchange for **Gold Teeth**.
> - Confirm **Silph Scope** comes from Giovanni at the Game Corner Rocket
>   Hideout, and is the hard gate for Pokémon Tower.
> - Confirm **Secret Key** (Pokémon Mansion) gates the Cinnabar Gym door.

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
    saffron --> viridian8_open[Viridian Gym opens]

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

    %% --- Earth-Badge gym opens after Silph Co ---
    viridian8_open --> giovanni8[Giovanni / Earth]
    blaine --> giovanni8

    %% --- Endgame ---
    giovanni8 --> badgecheck[Route 23 - all 8 badges]
    badgecheck --> vroad[Victory Road - Strength + Surf]
    strok --> vroad
    surfok --> vroad
    vroad --> e4[Elite Four]
    e4 --> champ[Champion]
    champ --> END([Hall of Fame])
```

> **⚠️verify (the whole DAG):** §5 is the highest-value artifact and the most
> assumption-laden. The key claims to confirm from decomp:
> 1. **Cut** (HM01 from S.S. Anne) + **Cascade** is the gate to **Surge's gym**.
> 2. **Silph Scope** → Pokémon Tower → **Poké Flute** → Snorlax is a hard chain
>    on the way south to Fuchsia.
> 3. **Surf** (Safari Zone) + **Soul** is the gate to **Cinnabar**, and
>    **Strength** (Warden, via Gold Teeth) + **Rainbow** is a Victory Road gate.
> 4. **Viridian Gym (Giovanni / Earth)** only opens **after Silph Co.**
> 5. Victory Road's real internal HM requirements.
> 6. **Bill / S.S. Ticket** prerequisite: is it truly just "reach Cerulean," or
>    is **Cascade** (Misty) actually required? Drawn here as Cerulean-only.
> 7. **Fuchsia reachability** — is it gated *solely* by Poké Flute → Snorlax, or
>    is there an **earlier** legitimate (glitchless) approach (e.g. via Surf, or
>    a southern route) that would let Fuchsia/Koga/Safari happen sooner? If so,
>    the `flute → fuchsia` edge is too strict and should be loosened.

---

## Verification checklist (next pass against `decompiled/`)

- [ ] Badge → HM field-use authorization map (field-move scripts / badge flags).
- [ ] Which HMs are **strictly required** to reach the E4 vs. optional.
- [ ] Gym entry gates (Cut tree at Vermilion Gym; Secret Key at Cinnabar Gym;
      Viridian Gym open-flag tied to Silph Co.).
- [ ] Key-item givers & flags: S.S. Ticket (Bill), Silph Scope (Rocket Hideout),
      Poké Flute (Mr. Fuji), Tea (Celadon), Gold Teeth → Strength (Warden),
      Secret Key (Pokémon Mansion), Surf (Safari Zone).
- [ ] Snorlax block flags (Routes 12 & 16) and Poké Flute dependency.
- [ ] Saffron gatehouse guard gate (Tea) — FRLG-specific, confirm item + scripts.
- [ ] Route 23 badge-check count and Victory Road traversal HMs.
- [ ] Exact final-input endpoint at the Champion→Hall-of-Fame transition
      (from scripts, per `tas-rules.md`).
- [ ] FireRed vs LeafGreen differences in any of the above (should be none for
      structure, but confirm version-exclusive item/encounter givers).
