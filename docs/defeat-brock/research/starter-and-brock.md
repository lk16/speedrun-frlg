# Research: the starter's generation, Brock's party, and the numbers between them

Derived 2026-08-12 from `decompiled/` only. Every claim cites a path; the two marked
*needs-emulator* are the ones the text alone cannot pin. This is research, not route — the
route document consumes it.

## The starter is 4 `Random()` calls, all at `givemon` time

The ball scripts set `PLAYER_STARTER_SPECIES` (`data/maps/PalletTown_ProfessorOaksLab/scripts.inc:1073,1216,1229`)
and converge on `..._EventScript_ChoseStarter` (`:1115`), which runs
`givemon PLAYER_STARTER_SPECIES, 5` (`:1122`) — level 5. `ScrCmd_givemon`
(`src/scrcmd.c:1734-1757`) → `ScriptGiveMon` (`src/script_pokemon_util.c:48-72`) →
`CreateMon(mon, species, level, 32, 0, 0, OT_ID_PLAYER_ID, 0)` (`:55`).

With `fixedIV = 32 = USE_RANDOM_IVS` (`include/constants/pokemon.h:232`), no fixed
personality, and `OT_ID_PLAYER_ID` (no anti-shiny reroll loop — that `do/while` at
`src/pokemon.c:1786-1792` only runs for `OT_ID_RANDOM_NO_SHINY`), `CreateBoxMon` consumes
exactly:

1. **PID = `Random32()`** (`src/pokemon.c:1778`; macro `(Random() | (Random() << 16))`,
   `include/random.h:14`) — 2 calls. *Needs-emulator:* which call becomes the low vs high
   half is evaluation-order, i.e. compiler-determined; confirm against libmgba before the
   model trusts it.
2. **IVs = 2 more `Random()`** (`src/pokemon.c:1836-1852`): first roll packs HP/Atk/Def in
   bits 0-4/5-9/10-14, second packs Speed/SpA/SpD the same way.

Nature is `PID % 25` (`src/pokemon.c:5020-5023`); gender is `genderRatio > (PID & 0xFF)`
(`:2743-2746`, starters 12.5% female, `species_info.h:59,146,233`); no ability split for
starters (second ability is NONE, `species_info.h:62,149,236`). Initial moves are
deterministic (`GiveBoxMonInitialMoveset`, `src/pokemon.c:2265-2286`).

**Consequence for the route:** the starter's nature and all six IVs are decided by 4
consecutive rolls of the stream at one script frame. Idle frames before pressing A on the
ball are a stat dial — same currency as the battle-delay searches.

Trainer mons, for contrast, are deterministic in stats (PID from a name hash,
`src/battle_main.c:1556-1589`; IVs from `.iv * 31 / 255`, `:1588`) but burn ≥2 rolls each in
the anti-shiny loop at party creation.

## Brock

`src/data/trainers.h:4135-4144`: class LEADER, **no items**, AI
`CHECK_BAD_MOVE | TRY_TO_FAINT | CHECK_VIABILITY`, custom-moves party
(`src/data/trainer_parties.h:5604-5616`):

- **Geodude L12**, moves {Tackle, Defense Curl} — *no Rock Throw* (the L11 learnset default
  is overridden by the custom moveset).
- **Onix L14**, moves {Tackle, Bind, Rock Tomb}.
- Both `.iv = 0` → all IVs 0 (`src/battle_main.c:1588`).

Defeat observables (`data/maps/PewterCity_Gym/scripts.inc:12-21`): `FLAG_DEFEATED_BROCK`
(`:14`) and `FLAG_BADGE01_GET` (`:15`), both set before the TM39 handout dialogue.

Pewter Gym's optional trainer, Camper Liam (`src/data/trainers.h:1415-1424`,
`trainer_parties.h:858-871`): Geodude L10 {Tackle, Defense Curl}, Sandshrew L11 {Scratch,
Defense Curl, Sand Attack}, both IV 0, AI CHECK_BAD_MOVE only.

Route 22 early rival (optional; `data/maps/Route22/scripts.inc:61-69`,
`trainer_parties.h:3744-3787`): Pidgey L9 + your-starter's-counter L9, `.iv = 50` → IV 6.

## Type math vs Rock/Ground (`gTypeEffectiveness`, `src/battle_main.c:306-400`)

- Grass → Rock 2.0 (`:343`), Grass → Ground 2.0 (`:340`) ⇒ Vine Whip is **4×** on both.
- Water → Rock 2.0 (`:328`), Water → Ground 2.0 (`:327`) ⇒ Bubble/Water Gun **4×**.
- Normal → Rock 0.5 (`:314`), no Normal → Ground entry ⇒ Tackle/Scratch **0.5×**.
- Fire → Rock 0.5 (`:321`) ⇒ Ember **0.5×**; Rock → Fire 2.0 (`:397`) ⇒ Rock Tomb hits
  Charmander super-effectively.

Move data (`src/data/battle_moves.h`): Tackle 35/95 (`:432`), Vine Whip 35/100 pp10 (`:289`),
Bubble 20/100 spd-down 10% (`:1888`), Water Gun 40/100 (`:718`), Ember 40/100 burn 10%
(`:679`), Metal Claw 50/95 (`:3019`), Rock Tomb 50/80 spd-down 100% pp10 (`:4124`), Bind
15/75 trap (`:263`), Defense Curl def-up (`:1446`), Growl (`:588`), Tail Whip (`:510`).

## Learnsets to L16 (`src/data/pokemon/level_up_learnsets.h`)

- **Bulbasaur** (`:4-17`): Tackle 1, Growl 4, Leech Seed 7, **Vine Whip 10**, Poison
  Powder/Sleep Powder 15.
- **Charmander** (`:54-66`): Scratch/Growl 1, Ember 7, **Metal Claw 13** (Steel: 2× vs Rock —
  but Rock Tomb is 2× back and Ember is resisted).
- **Squirtle** (`:101-114`): Tackle 1, Tail Whip 4, **Bubble 7**, Withdraw 10, **Water Gun
  13**.

## Base stats (`src/data/pokemon/species_info.h`)

| Mon | HP/Atk/Def/Spe/SpA/SpD | exp yield | growth |
| --- | --- | ---: | --- |
| Bulbasaur (`:38-65`) | 45/49/49/45/65/65 | 64 | MEDIUM_SLOW |
| Charmander (`:125-152`) | 39/52/43/65/60/50 | 65 | MEDIUM_SLOW |
| Squirtle (`:212-239`) | 44/48/65/43/50/64 | 66 | MEDIUM_SLOW |
| Geodude (`:2155-2182`) | 40/80/100/20/30/30 | 86 | MEDIUM_SLOW |
| Onix (`:2764-2791`) | 35/45/160/70/30/45 | 108 | MEDIUM_FAST |
| Caterpie (`:299-326`) | 45/30/35/45/20/20 | 53 | MEDIUM_FAST |
| Weedle (`:386-413`) | 40/35/30/50/20/20 | 52 | MEDIUM_FAST |
| Metapod (`:328-355`) | 50/20/55/30/25/25 | 72 | MEDIUM_FAST |
| Kakuna (`:415-442`) | 45/25/50/35/25/25 | 71 | MEDIUM_FAST |
| Pikachu (`:734-761`) | 35/55/30/90/50/40 | 82 | MEDIUM_FAST |

Geodude/Onix both carry Rock Head or **Sturdy** (`species_info.h:2180,2788`) — which ability
each instance has follows from its deterministic trainer-mon PID; worth computing, since
Sturdy blocks OHKO *moves* only in Gen 3 (to be cited before relied on).

## Exp (`src/battle_script_commands.c:3113-3237`)

- Base: `expYield * level / 7` (`:3166`), split over participants (`:3179`, min 1), **×1.5
  for trainer battles** (`:3231-3232`), all truncating integer math.
- Growth table: starters are MEDIUM_SLOW (`species_info.h:60,147,234`), table at
  `src/data/pokemon/experience_tables.h:329+`, macro
  `(6n³)/5 − 15n² + 100n − 140` (`:7`). Cumulative: L5 135, L6 179, L7 236, L8 314, L9 419,
  L10 560, L11 742, L12 973, L13 1261, L14 1612, L15 2035, L16 2535. *Needs-emulator:*
  values computed from the macro, spot-check one against the ROM.

Worked yields (integer math, single participant):
lab rival's starter L5, yield 64-66 → `yield*5/7` ×1.5 = 67-70.
Camper Liam: Geodude L10 → 86*10/7=122, ×1.5=183; Sandshrew L11 (yield 93,
`species_info.h` Sandshrew — **not yet quoted, verify before use**) ≈ 219. 
Route 22 rival: Pidgey L9 (yield 55, same caveat) ≈ 105; starter-counter L9 ≈ 123.

## Stat formula (`src/pokemon.c:2093-2170`, `:5404-5438`)

- HP: `((2·base + IV + EV/4)·level)/100 + level + 10` (`:2130-2131`), never
  nature-modified.
- Others: `((2·base + IV + EV/4)·level)/100 + 5`, then nature ×1.1 or ×0.9 with truncation
  (`ModifyStatByNature`, `:5404-5438`; table `sNatureStatTable`, `:1360+`, columns
  Atk/Def/Spe/SpA/SpD). Nature = PID % 25 (`:5022`).

## What this says about the route (plan-level, to be measured)

Vine Whip at L10 (needs 560 cumulative exp; the L5 starter has 135) is the cheapest 4×
attack unlock of the three starters. The lab rival fight yields ~68; the remaining ~357
has to come from optional fights — Camper Liam alone (~400 total) covers it, and he stands
in Brock's own gym. Squirtle's Bubble-at-7 (20 power) is weaker but nearly free in exp;
whether "L7 Squirtle spamming Bubble" beats "L10 Bulbasaur spamming Vine Whip" end-to-end
is a measurement, not an argument — both survive as candidates for the version × starter
sweep. Charmander looks structurally worst (resisted until L13, weak to Rock Tomb) but is
swept anyway, because the rival-1 tables proved unmanipulated intuition ranks nothing.
