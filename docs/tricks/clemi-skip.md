# Clemi skip: cutting the catch jingle with the Help menu

> **Status: mechanism CONFIRMED in source; jingle length COMPUTED from song data (≈213
> frames); saving still UNMEASURED.** The input window, the open/close overhead, and the
> claimed ~2.68 s saving have not been measured in this project at any tier — but the
> computed jingle length is consistent with the claim under the corrected cost model
> below. No committed route contains a catch yet, so nothing in any ledger uses this.

## Provenance

Supplied externally on 2026-08-15. Attributed to Swiss runner **iamClemi**, discovery dated
March 2026: opening the Help menu at a precise frame during a Pokémon catch cuts the caught
jingle short, claimed to save **~2.68 s per successful catch**. The closed network means the
original source is unverifiable here; the mechanism below is re-derived from `decompiled/`
and every step of it is cited. The 2.68 s figure and the "precise frame" window are the
external claim, repeated here unverified. Second pass 2026-08-15: every citation below
re-checked against `decompiled/` line by line; the jingle-intro length computed from the
song data; the original cost model in this note (which would have made the claim
impossible) corrected — see below.

## The unskippable wait it attacks

The catch-success sequence hard-blocks on a *sound effect*, and no button can skip it:

1. When the capture succeeds, the ball sprite callback counts frames in `data[4]`; at tick
   95 it silences everything and starts the jingle intro **on a sound-effect player**:
   `m4aMPlayAllStop()` then `PlaySE(MUS_CAUGHT_INTRO)`
   (`decompiled/src/pokeball.c:891-900`; same pattern in the wild-battle anim variant,
   `decompiled/src/battle_anim_special.c:1195-1201`). The same tick-95 branch clears
   `gDoingBattleAnim` (`src/pokeball.c:898`, `src/battle_anim_special.c:1197`), and that
   flag is exactly what the controller's completion wait checks
   (`CompleteOnSpecialAnimDone`, `decompiled/src/battle_controller_player.c:1345-1349`,
   installed by both ball-throw handlers, `:2278-2294`) — so the battle script resumes
   **at tick 95**, while the animation runs on visually to its teardown at tick 315
   (`src/pokeball.c:902`). The caught message below prints *during* the animation, not
   after it; the tick-315 teardown gates nothing the script waits on.
2. `MUS_CAUGHT_INTRO` is assigned to **SE player 2** in the song table
   (`song mus_caught_intro, 2, 2`, `decompiled/sound/song_table.inc:321`) — unlike
   `MUS_CAUGHT`, which is a real BGM (`:324`). So while it plays, `IsSEPlaying()` is TRUE
   (`decompiled/src/sound.c:612-619`). How long is that? The song is 180 ticks at 24
   ticks per beat — 12 ticks at 64 bpm, then 168 at 136 bpm
   (`decompiled/sound/songs/midi/mus_caught_intro.mid`, converted with the repo's own
   `tools/mid2agb`, whose output carries `TEMPO, 64*…/2` then `TEMPO, 136*…/2`; the
   TEMPO byte is bpm/2). The engine adds `tempoI` once per frame and processes one tick
   per 150 accumulated, with `tempoD = 2 × byte` and `tempoU = 0x100`
   (`decompiled/src/m4a_1.s:1169-1172`, `:1353-1360`; `ply_tempo`, `:920-931`;
   `src/m4a.c:634-636`), so the intro holds `IsSEPlaying()` TRUE for
   12·150/64 + 168·150/136 ≈ **213 frames (~3.6 s)** — computed from source, not yet
   tier-1 measured.
3. The caught message is
   `"Gotcha!\n… was caught!{WAIT_SE}{PLAY_BGM MUS_CAUGHT}\p"`
   (`decompiled/src/battle_message.c:475`), printed by `BattleScript_SuccessBallThrow`
   (`decompiled/data/battle_scripts_2.s:73-77`). `{WAIT_SE}` parks the text printer in
   `RENDER_STATE_WAIT_SE`, which spins until `!IsSEPlaying()`
   (`decompiled/src/text.c:718-719`, `:902-904`).
4. The battle controller only reports the print finished when the printer goes inactive
   (`PlayerHandlePrintString` → `CompleteOnInactiveTextPrinter2`,
   `decompiled/src/battle_controller_player.c:2379-2392`, `:1291-1295`), so the whole
   battle script — and everything after it — is gated on the jingle intro ending. Mashing
   A/B is useless: the `\p` button wait (`src/text.c:859-861`, `:569-584`) is *after*
   `{WAIT_SE}` in the string.

This wait binds a TAS exactly as it binds a human; it is dead time on every catch.

## Why the Help menu kills it

The Help system is polled **before** both main callbacks, every frame
(`CallCallbacks`, `decompiled/src/main.c:241-250`). When the button mode is HELP (the
game's default) it opens on a fresh `L_BUTTON | R_BUTTON` press, and the very first thing
the open path does is stop both SE players:

- gate and press: `decompiled/src/help_system_util.c:47-52`
- `m4aMPlayStop(&gMPlayInfo_SE1); m4aMPlayStop(&gMPlayInfo_SE2);` — `:59-60`

That kills `MUS_CAUGHT_INTRO` mid-note. While Help is open, `callback1`/`callback2` are
frozen (battle, text printer, sprite anims all stop). On close it plays `SE_HELP_CLOSE`
(`:109`), a short SE; once that ends, the printer's `{WAIT_SE}` gate passes, `{PLAY_BGM
MUS_CAUGHT}` starts the (non-blocking) BGM half of the jingle, and `\p` is dismissed on the
next A/B press.

Help **is** available during battles: it is disabled only for the battle-transition
animation and re-enabled the frame the transition completes
(`decompiled/src/battle_setup.c:192-202`); battle-specific help contexts are set at
`decompiled/src/battle_main.c:632-642`. Preconditions (gate at
`src/help_system_util.c:44-67`): options button mode = HELP — the new-game default
(`decompiled/src/new_game.c:68`; touching the button-mode option would break the trick);
single player, meaning no remote link players (`HelpSystem_IsSinglePlayer`,
`decompiled/src/help_system.c:1898-1903`); and `gHelpSystemEnabled` TRUE — cleared during
quest-log playback (`decompiled/src/quest_log.c:462`, restored `:1232`) and in the Hall
of Fame (`src/hall_of_fame.c:322`). An R-only press can additionally be swallowed by
`gHelpSystemToggleWithRButtonDisabled` (`src/help_system_util.c:50-51`); an L press never
is, so the input below is L.

## Cost model (length computed, overhead to be measured)

The script resumes at tick 95, the same tick the jingle starts. The printer renders
"Gotcha! …" and parks in `{WAIT_SE}` until the intro's ≈213 frames run out at roughly
tick 308 — so the natural dead time is about **213 frames minus the print time**, and the
animation finishing at tick 315 hides almost none of it. (An earlier revision of this
note modelled the message as starting only after the tick-315 teardown, leaving
`intro − 220` frames to save; the computed 213-frame length makes that model impossible —
it would predict *negative* saving — while the corrected resume-at-95 model, cited in
step 1 above, is what the controller code actually does.)

Saving per catch = (frames of `{WAIT_SE}` still remaining at the L press)
− (open/close overhead while `callback1`/`callback2` are frozen, plus the tail of
`SE_HELP_CLOSE`, which `{WAIT_SE}` also waits out). The external ~2.68 s (≈160 frames)
is consistent with this model iff print time plus round-trip overhead comes to ~50
frames — plausible, but unmeasured. *Needs-emulator:* the earliest and latest working
press frame, the frozen-frame overhead of the open/close round trip, the
`SE_HELP_CLOSE` length, and the print time at the route's text speed.

One-time cost: the first Help open of a save shows a welcome message
(`HelpSystem_UpdateHasntSeenIntro`, `src/help_system_util.c:89`) — the first use may be
longer than steady-state, or worth burning somewhere free.

## Where it applies here

- **rival-1, defeat-brock: no effect.** Neither committed route catches anything; nothing
  in their ledgers changes.
- **glitchless-run: real.** The mandatory Old Man tutorial catch uses the sibling message
  `"…{WAIT_SE}{PLAY_BGM MUS_CAUGHT}{PAUSE 127}"`
  (`decompiled/src/battle_message.c:476`, `BattleScript_OldMan_Pokedude_CaughtMessage`,
  `data/battle_scripts_2.s:100`) — same `{WAIT_SE}` gate, so the skip plausibly applies to
  the scripted Weedle catch too (tier-0 hypothesis; the pokedude controller path may
  differ — check before counting it). Every planned real catch (second party mon,
  `docs/glitchless-run/plan.md` §"which second mon") saves once per catch.
- The L press is an ordinary input in the `.bk2` L column; no settings or menu detour is
  needed since HELP is the default button mode.
