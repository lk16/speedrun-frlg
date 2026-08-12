# 2026-08-12 (sandbox) -- 12713 -> 10946: the three cheap inefficiencies are routed out

Task: optimise the route to the rival win. The three items the 2026-08-11 tier-2 viewing put on
the list -- the seven-character names, MID text speed, battle animations -- are now all gone,
one build, measured end-to-end through the battle. **10946 frames, -1767 (-13.9%), tier-1
verified from reset, exported and queued as `route-10946f-b1a0875a77e9`.** Segment numbering
shifted: the new `04-options` pushes everything after it up by one (`09-battle-win` is the
battle now).

**What the 1767 frames are.** `03-names` types one letter, START (a documented cursor shortcut
to OK, `decompiled/src/naming_screen.c:1485`), A -- and takes KAZ off the rival's preset menu
instead of a second naming screen (rows are `sRivalNameChoices`, `oak_speech.c:647`; the menu
wraps, so it is two UPs from the top). 1450 -> 1238. `04-options` opens START -> OPTION in the
bedroom and sets text speed FAST plus battle scene OFF in one 197-frame detour. Everything
downstream got cheaper: `07-starter` -794 (its text at 1 frame/char instead of 4), `06-to-lab`
-150, `08-battle-start` -88, `05-house` -24, and the battle -- fresh stream, re-searched, 8/16
start delays win -- came in at 3322, -696.

**Two wrong assumptions the decomp corrected, worth keeping:**

- **There is no preset menu for the player.** The 2026-08-11 route notes implied preset names
  were "two D-pad presses away" for both names. The flow is asymmetric:
  `Task_OakSpeech_YourNameWhatIsIt` fades the player straight into the naming screen
  (`oak_speech.c:1352-1379`); the player's preset menu exists only on the say-NO re-ask path.
  The rival's menu is real and literal. (Near-miss worth noting: the player's name buffer is
  *prefilled* with `sMaleNameChoices[Random() % 19]` before the naming screen opens
  (`Task_OakSpeech_DoNamingScreen` -> `GetDefaultName`, `oak_speech.c:1444,2146`), so
  START+A on an untouched screen keeps a random 3-6 char preset. Rejected: a searched-delay
  3-char draw is never better than the deterministic 1-char typed name.)
- **Single-frame taps die in the start menu.** The first options attempt tapped UP twice and
  pressed A on EXIT: while the start menu is up, `gMain.newKeys` goes stale in runs of 2-3
  frames -- input reads get skipped -- and the field swallows everything for ~20 frames after
  the walk-in transition (`Task_ExitNonDoor`). The fix is structural, not a longer wait: every
  press in `04-options` is a mash-until-effect against a RAM observable
  (`sStartMenuCursorPos`, the option menu's working values, its `loadState`), which stops on
  the frame the effect lands and cannot overshoot. New observer probes for all of it, each
  checked against the running game in `tests/observe.rs`.

**Also written down while in there:** the run's RNG stream is seeded twice, both from timer 1
-- at title-screen exit and again at *player* naming-screen exit (`SeedRngAndSetTrainerId`,
`title_screen.c:735`, `naming_screen.c:722`, `main.c:264`) -- so manipulation upstream of the
naming exit cannot reach the battle except by moving the exit itself. In `docs/rival-1/route.md`'s RNG
section now.

**Unverified.** Tier 2 for the new movie: queued, not replayed (the 12713 pass covers the
previous movie only -- same boot, core and format, but plausible is not proven). The
`turn_hold` sweep is still the mGBA-0.10.5 one, now two route generations stale; `frlg route
tune` on the current route has not been re-run. Whether the new battle contains a crit either
way: not checked.
