# 2026-08-11 (host, night) -- TIER 2 PASSES: BizHawk replays the whole route frame for frame

**The headline, for whoever picks this up in a sandbox: `route-12713f-a4ad4280bbdc` passed.**
Luuk ran `tools/verify-runner.sh` on the host and the result is in
`$FRLG_ARTIFACTS/verify/results/route-12713f-a4ad4280bbdc.json`:

    "verdict": "pass"
    "ram_hash": "73b329af5d561a864cc4b0d46e8d4c409ce1b6df"   (== expected_ram_hash)
    "notes":   "replayed 12713 frames; fingerprint matches; probe trace matched every frame"

Read the third line twice. The final fingerprints agreeing would have been a pass; the
**`gRngValue` trace matching on all 12713 frames** means the two emulators never diverged for a
single frame anywhere in the run. The boot fix was the whole desync, the 2026-08-12 audit that
found no second cause was right, and the trace machinery that was built to name a desync frame
instead got used to prove there was no desync to name.

What this closes: the ledger's per-segment `tier2` no longer says "not replayed" (it names the
passing run and the `ilog` digest it was built from); `docs/rival-1/route.md`'s header no longer says
"tier 1 only"; and `tools/verify-runner.lua`, three sessions old and never once having completed
a report, has now completed one end to end -- status file, 288K RAM dump, per-frame compare.
Rebuilding the route resets the tier-2 stamp, and should: a rebuilt movie has not been replayed.

**One inconsistency worth knowing before it wastes someone's afternoon.** Re-exporting the same
route produces the same `ilog_sha1` and a *different* `bk2_sha1` (`f60e7120…` then `4d947b73…`).
The `.bk2` is a zip and its entry timestamps move; the input log is identical, which is why the
re-exported movie replayed to the same fingerprint. The `.ilog` digest is the identity, the
`.bk2` hash is not. Noted in `docs/rival-1/route.md`.

### Three inefficiencies, now written down with citations

Luuk watched the run and named three. All three are real, and one of them contradicts something
this journal has claimed since 2026-08-10.

**Battle animations are on.** `NewGameInitData` sets `optionsBattleSceneOff = FALSE`
(`decompiled/src/new_game.c:66`) and nothing in the route ever opens OPTIONS, so
`BattleStartClearSetData` never sets `HITMARKER_NO_ANIMATIONS` (`decompiled/src/battle_main.c:2259`
-- and neither of the two battle types that would block it, LINK and POKEDUDE, applies here).
Every attack in the 4018-frame battle plays its animation. The switch is `MENUITEM_BATTLESCENE`
(`decompiled/src/option_menu.c:514`), reached through START -> OPTION
(`decompiled/src/start_menu.c:531`).

**The name is seven characters, and each surplus character has a price now.** The naming screen
mash types A until the name fills up, and every later message box that prints the name pays for
all seven. The price: `sTextSpeedFrameDelays` is `{SLOW: 8, MID: 4, FAST: 1}` frames per
character (`decompiled/src/new_menu_helpers.c:27-32`), and the route runs at the default MID. Six
surplus characters is 24 frames per message box that prints the name. This also prices the text
speed item that has been on the list since 2026-08-10: FAST is a 4x on every character in the
run, and it is the *same* OPTIONS detour as the battle animations, so the two should be priced
together rather than one at a time.

**"Bulbasaur crits us" -- and this journal said that was impossible.** It was wrong, and the
decomp says so plainly. The 2026-08-10 entry (and `docs/rival-1/route.md`) claimed criticals are off for
the whole first battle because of `BATTLE_TYPE_FIRST_BATTLE`. The actual condition is
`&& (!(gBattleTypeFlags & BATTLE_TYPE_FIRST_BATTLE) || BtlCtrl_OakOldMan_TestState2Flag(1))`
(`decompiled/src/battle_script_commands.c:1200`), and that second clause was never read.
`BtlCtrl_OakOldMan_TestState2Flag(1)` tests `FIRST_BATTLE_MSG_FLAG_INFLICT_DMG`
(`decompiled/src/battle_controller_oak_old_man.c:2228`, `decompiled/include/battle_controllers.h:287`),
which `CompleteOnHealthbarDone` sets the first time an opponent's hit finishes draining the health
bar, on its way to Oak's "inflicting damage is key" line
(`decompiled/src/battle_controller_opponent.c:304-306`). So criticals are suppressed for the
opening exchange only and are live for the rest of the battle, for both sides. Two things follow:
the crit `Random()` call is consumed on every damaging hit regardless, because `&&` short-circuits
left to right and the roll sits *before* the `FIRST_BATTLE` clause; and the rival's crit is a
search target, not a fact of life -- it costs damage (possibly a turn) plus a message box, in a
stream `08-battle-win` already re-searches. Nobody has measured the battle without it yet.

The lesson is the boring one: a citation that stops at the first `&&` is not a citation. This one
survived two sessions because it was *nearly* right.

### The runner: 213s -> 31s, once it stopped talking to a real X server

Replaying at 100% costs exactly what the TAS costs. `tools/verify-runner.sh` now seeds EmuHawk's
`config.ini` (plain JSON) before each launch: `Unthrottled`, no clock/vsync/sound throttle,
`DispSpeedupFeatures: 0` (its `MainForm::Render` returns immediately -- read from EmuHawk.exe IL,
not guessed), sound off, and the dialog suppressors that keep an unattended run from parking on a
modal window. `--realtime` puts the desk settings back, because watching a replay is what
produced three of the findings above and must stay one flag away.

**Determinism is checked, not assumed**: the same movie, replayed fast and silent, produced the
same fingerprint *and* the same 12713-frame probe trace as the 100%-with-sound run. That is the
pass quoted at the top of this entry -- it was replayed twice.

Unthrottling on the desktop bought only 1.6x -- 134s, ~95 fps -- and the shape of the miss was
the clue: 36s of CPU across 134s of wall clock, so the process was *waiting* ~8ms per frame
rather than working. Luuk installed `xvfb`, and `--headless` (`xvfb-run`) answered it:

    stock EmuHawk                      ~213s    59.7 fps, i.e. the movie's own length
    seeded config, on the desktop       134s    ~95 fps
    seeded config, --headless            31s    ~410 fps      <- default
    --headless, DispSpeedupFeatures 1    47s    ~270 fps
    --headless, DispSpeedupFeatures 2    47s    ~270 fps

**6.9x, and the win is the X server rather than the throttle.** Headless is 32s of CPU across
31s of wall -- the replay is finally CPU-bound, nothing waits. So the ~8ms/frame was the desktop
X connection, most likely the per-frame `UpdateWindowTitle()` that `DispSpeedupFeatures == 0`
switches on (`CalcFramerateAndUpdateDisplay`, EmuHawk.exe IL): a round trip per frame, cheap
against a local Xvfb, expensive against a real desktop. The last two rows are the same
experiment run the other way and they justify keeping `DispSpeedupFeatures: 0` -- letting
EmuHawk render costs 16s even with nothing to display it on. All four replays passed with the
identical fingerprint and trace, which is four more independent confirmations of the tier-2
result at the top of this entry.

`FRLG_VERIFY_CONFIG_EXTRA` (a JSON object applied on top of the seeded config) exists so the
next person to doubt one of these settings can measure it instead of arguing with the IL.

Note what headless is and is not: the sandbox still cannot run BizHawk (no mono, no installs),
so this does not move tier 2 into the sandbox. What it buys is `--watch --headless` draining
the queue on the host without taking over a screen.

**Unverified.** Segment-level tier-2 requests, which have never been made -- only the whole
route has ever been replayed. Everything else in this entry was measured.
