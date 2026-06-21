# Simulation design — what we must replicate

Implementation-level notes for the deterministic Rust simulation, derived from
`decompiled/`. The bar: **our input log, replayed from power-on, reproduces the
same outcomes as the real game** — so anything affecting RNG or frame timing
must match the decomp *exactly*. All paths are relative to `decompiled/`.

---

## 1. The frame model

The whole sim is driven one **frame = one VBlank** at a time. The real main loop
(`src/main.c` `AgbMain`, ~lines 100–217) per frame:

1. `ReadKeys()` — sample the joypad (this is where our movie's input enters).
2. Soft-reset check (`A+B+START+SELECT`), then `UpdateLinkAndCallCallbacks()`
   which calls `gMain.callback1()` then `gMain.callback2()` (the active game
   state — overworld, battle, menu…).
3. `PlayTimeCounter_Update()`, `MapMusicMain()`.
4. `WaitForVBlank()` — busy-waits until the VBlank interrupt fires.

The **VBlank interrupt** (`VBlankIntr`, `src/main.c` ~383–417) runs once per
frame and, critically, **calls `Random()` exactly once** (advancing the RNG)
plus increments frame counters (`gMain.vblankCounter2`, optional
`*vblankCounter1`). During battle the analogous `VBlankCB_Battle`
(`src/battle_main.c` ~1650) also calls `Random()` once per frame.

**Implication:** the RNG advances once per displayed frame *regardless of input*,
on top of any gameplay RNG calls. Our sim must model time as a frame counter and
advance the RNG per frame in lockstep with the game state.

### Input (`src/main.c` `ReadKeys` ~296–338; `include/main.h`)

Button bits (`include/gba/io_reg.h` ~698–707) — **same layout as the VBM word**:

| Button | Bit |
|--------|-----|
| A | `0x0001` |
| B | `0x0002` |
| SELECT | `0x0004` |
| START | `0x0008` |
| RIGHT | `0x0010` |
| LEFT | `0x0020` |
| UP | `0x0040` |
| DOWN | `0x0080` |
| R | `0x0100` |
| L | `0x0200` |

`KEYS_MASK = 0x03FF`. The game derives, each frame:
- `heldKeys` (raw held this frame), `newKeys` (`held & ~prevHeld`),
- `newAndRepeatedKeys` — `newKeys` plus key-repeat: after
  `gKeyRepeatStartDelay = 40` frames held, repeats every
  `gKeyRepeatContinueDelay = 5` frames.
- Optional **L=A remapping** if the save's button mode is L=A (off for a fresh
  save by default — verify).

Menus test `JOY_NEW` (newKeys), held movement tests `JOY_HELD`, list cursors use
`JOY_REPT` (newAndRepeatedKeys). Our sim must reproduce the new/held/repeat
derivation from the raw per-frame button word so that menu navigation timing
matches.

> **We do not record key-repeat in the movie** — we record the raw held buttons
> per frame; the engine derives repeat. The movie is just the per-frame 10-bit
> joypad state.

---

## 2. RNG (the crux of determinism)

`src/random.c`, `include/random.h` — **verified by direct read**:

```c
gRngValue: u32                       // state, starts 0 at boot
Random()  = { gRngValue = 1103515245 * gRngValue + 24691; return gRngValue >> 16; }  // u16
Random32()= Random() | (Random() << 16)
SeedRng(seed: u16) = { gRngValue = seed; }   // zero-extends to u32
```

- LCG: multiplier `1103515245` (`RAND_MULT`), increment **`24691`**
  (`ISO_RANDOMIZE1`). `Random()` returns the **high 16 bits**.
- `ISO_RANDOMIZE2` (`+12345`) / `Random2` / `gRng2Value` exist in headers but are
  **unused** — ignore them. (An earlier note citing `12345` was wrong.)

### Seeding — the determinism hinge

`SeedRngAndSetTrainerId()` (`src/main.c` ~264) does:
```c
u16 val = REG_TM1CNT_L;   // free-running hardware Timer 1 count
SeedRng(val);             // seed = timer value
gTrainerId = val;
```
- Timer 1 starts during the title screen (`StartTimer1()`,
  `src/title_screen.c` ~351) and is read to seed the RNG when the player starts
  a new game (`src/title_screen.c` ~735) and/or at end of the naming screen
  (`src/naming_screen.c` ~722).
- **So the initial seed = Timer 1 value at the moment of seeding, which is a
  function of exactly how many frames/cycles elapsed from boot to that input.**
  For a TAS this is *controllable but not fixed*: our chosen input timing
  determines the seed. We must (a) model Timer 1 vs frame timing, or (b) treat
  the seed as a search parameter and find input timings that yield a favorable
  seed. This is a core thing to nail in milestone 1.

> **To replicate Timer 1 precisely** we need its clock rate relative to VBlank
> (cycles/frame). That's a hardware-timing detail to confirm against mGBA / the
> register setup (`REG_TM1CNT_H`) — flagged as an open item.

---

## 3. Milestone 1 control flow: power-on → starter → first rival

### Boot / intro
`src/intro.c` (copyright → Game Freak logo → Nidorino/Gengar cutscene) →
`src/title_screen.c` (title; A or timeout) → main menu (New Game / Continue) →
`src/oak_speech.c` (Oak intro + naming). Each screen is a small state machine
advanced by input/timers; no gameplay RNG of consequence except the per-frame
VBlank `Random()` advancing the state. Frame costs are timer-driven and must be
measured, not assumed.

### New-game init — `src/new_game.c` `NewGameInitData()` (~107–152)
- `InitPlayerTrainerId()` — **one RNG-derived trainer ID** (`Random32()`-style)
  — but note seeding via the Timer-1 path above; reconcile which happens first.
- `SetMoney(3000)`, clear Pokédex/flags/playtime, zero parties.
- `WarpToPlayersRoom()` → Pallet Town, player's house 2F (6,6).

### Starter selection — `data/maps/PalletTown_ProfessorOaksLab/scripts.inc`
Three balls; each sets vars (player/rival starter species). Player gets the mon
via `givemon SPECIES, 5` → `ScriptGiveMon` (`src/script_pokemon_util.c` ~48) →
`CreateMon(mon, species, 5, 32, 0, 0, OT_ID_PLAYER_ID, 0)`.
- **IV arg = 32 ⇒ fixed/maxed IVs, no IV roll. Starter creation is fully
  deterministic — NO RNG.** Rival's starter is the type-advantaged one,
  hardcoded by the player's choice (Bulba→rival Charmander, Squirtle→rival
  Bulba, Charmander→rival Squirtle).

> **Verify:** whether `CreateMon` with that arg consumes any RNG for PID/nature/
> gender. The script report says no IV roll; confirm the PID path in `pokemon.c`
> `CreateMon`/`CreateBoxMon` doesn't pull RNG, since PID would matter for later
> mechanics (it likely does call `Random` for personality — re-check before
> trusting "no RNG").

### First rival battle — same `scripts.inc`
Triggered post-pick via `trainerbattle_earlyrival`:
- player chose Bulbasaur → `TRAINER_RIVAL_OAKS_LAB_BULBASAUR`
- Squirtle → `TRAINER_RIVAL_OAKS_LAB_SQUIRTLE`
- Charmander → `TRAINER_RIVAL_OAKS_LAB_CHARMANDER`

Parties in `src/data/trainer_parties.h`, trainer entries in `src/data/trainers.h`
(`include/constants/opponents.h` for IDs). Rival has one level-5 starter, default
moves. Post-battle `special HealPlayerParty`.

---

## 4. Battle engine (enough for the rival fight)

Stat calc — `src/pokemon.c` `CalculateMonStats` (~2102):
```
stat = ((2*base + IV + EV/4) * level)/100 + 5,   then ×nature
HP   = ((2*baseHP + IV + EV/4) * level)/100 + level + 10
```
At level 5 with EV 0, IV 32(? confirm), known base stats → fixed stats.

Damage — `src/pokemon.c` `CalculateBaseDamage` (~2385):
```
dmg = ((2*level/5 + 2) * power * atk / def) / 50 + 2
```
then in battle scripts: × crit × type × STAB × random(85–100%), plus burn,
screens, badges, items, abilities. STAB ×1.5, type table at `src/battle_main.c`
~312, applied in `Cmd_typecalc` (`src/battle_script_commands.c` ~1274).

### RNG consumption order per attacking turn (must match exactly)
`src/battle_script_commands.c` / `src/battle_main.c`:
1. **Turn start:** `gRandomTurnNumber = Random()` (used for Quick Claw etc.).
2. **Turn order:** if priority equal and **speed tie** → `Random() & 1` decides.
   Speed in `GetWhoStrikesFirst` (`battle_main.c` ~3400).
3. **Accuracy:** `(Random() % 100) + 1 > acc` ⇒ miss (`Cmd_accuracycheck` ~1003).
   (Some moves skip the roll.)
4. **Crit:** `Random() % sCriticalHitChance[stage] == 0` ⇒ crit, ×2
   (`Cmd_critcalc` ~1170; table `{16,8,4,3,2}`).
5. **Damage roll:** `100 - (Random() % 16)` ⇒ 85–100% (`ApplyRandomDmgMultiplier`).
6. **Secondary effects** (status chance, Focus Band, etc.) as applicable.

…plus the **once-per-frame VBlank `Random()`** ticking underneath all of the
above. The number of frames a battle animation/text takes therefore changes the
RNG state — so battle outcomes depend on *how many frames elapse*, which depends
on our text-speed/input timing. This couples timing and combat RNG; the sim must
account for it.

### Enemy AI — `src/battle_ai_script_commands.c` `BattleAI_ChooseMoveOrAction` (~363)
Scores moves deterministically; ties broken by `Random() % numBestMoves` (~408).
Some AI script commands consume RNG (`Cmd_if_random_less_than`). Early rival AI
is simple but **not** RNG-free — model the scoring + tie-break.

---

## 5. FireRed vs LeafGreen

Same engine and code paths; differences are data (version-exclusive species/
encounters, a few text/flag bits). Plan: build version as a compile/runtime flag
over the same simulation, load the right data tables, and compare frame costs of
identical milestones. The decomp builds both from the same tree, so divergences
are localized — track them in this doc as we hit them.

---

## 6. Build order & open questions

**Suggested Rust build order for milestone 1:**
1. RNG (LCG + `Random`/`Random32`/`SeedRng`) — trivial, exact.
2. Frame/input model: a per-frame loop that advances RNG once/frame and derives
   new/held/repeat keys from the raw movie word.
3. Movie writer (VBM first — word = our key bits directly; see `tas-rules.md`).
4. Enough of the intro/new-game/starter state machine to reach the rival battle,
   counting frames.
5. A minimal battle engine: stats, damage, the 6-step RNG order, simple AI.
6. Emit a VBM from power-on; replay in an emulator; assert the rival is beaten.

**Open questions to resolve before/while building:**
- **Timer 1 → seed**: exact clock rate vs frames; how to compute (or search for)
  the seed our input timing produces. *(highest priority — without this the
  whole replay desyncs.)*
- Does `CreateMon` for the starter consume RNG (PID/nature)? Re-verify in
  `src/pokemon.c`; "no RNG" is asserted but unconfirmed.
- Exact frame costs of each intro/menu/text transition (measure against emulator).
- Confirm starter IV arg semantics (`32` = per-IV value vs flag).
- Whether `NewGameInitData`'s trainer-ID RNG call happens before or after the
  Timer-1 seed, and its effect on the seed stream.
- Canonical verification emulator/format (VBM vs BizHawk .bk2) — user decision.
