# 2026-08-12 (sandbox, after the tier-2 pass) -- hold A, search to a fixpoint, race all six cells: 10085 -> 9658

Three route changes landed together, because each re-rolls the battle and the honest score
is one full rebuild:

1. **`text_hold`** -- every dialogue mash now *holds* A (or B) for N frames per one-frame
   release instead of alternating. `RenderText` prints a character on every held frame once
   one press lands in the box (`decompiled/src/text.c:639-650`); the `[A, 0]` mash only held
   half the frames. Measured on the intro alone (upstream of the naming-screen reseed):
   hold 4 = 3229 frames vs mash's 3699, non-monotonic across N because the release phase has
   to line up with when boxes become ready (full table in `docs/rival-1/route.md`).
2. **The battle search's per-turn stage repeats until a pass adopts nothing** (bounded at 8).
   Across the day's 146 builds, pass 2 adopted further cuts in 13, and one build (the
   LeafGreen/Charmander th8-xh1 re-run) kept adopting into pass 3 -- the repeat loop is not
   paranoia. Two lg-charmander variants were re-run because orphaned builds from a killed
   sweep briefly raced their directories; both re-runs reproduced the table's numbers, so
   the published table stands.
3. **Version and starter are swept, not assumed.** `bin/frlg-sweep` runs a 24-variant tuning
   sweep (`turn_hold` 1-8 x `text_hold` {1,2,4}) as parallel builds; six sweeps covered
   every version x starter cell. Best of each, total frames:

   |            | Squirtle | Charmander | Bulbasaur |
   | ---        | ---:     | ---:       | ---:      |
   | FireRed    | 9789     | 9749       | 9666      |
   | LeafGreen  | 9747     | 9741       | **9658**  |

   **LeafGreen with Bulbasaur wins at 9658** (`turn_hold` 4, `text_hold` 4, battle plan
   `[4, 3, 3, 3]`, 3 turns, 2409 frames) and is now the committed route. Bulbasaur was the
   *worst* starter in the old mashed table; both its cells win here with 3-turn battles, and
   both are also the most fragile (10/24 and 5/24 variants lose outright). The `text_hold 1`
   column of the FireRed/Squirtle sweep reproduced the previous sweep's totals exactly where
   the stream was unchanged (10085 at th2, 10531 at th8, th7 loses), which is the
   determinism check for free.

   LeafGreen needed: the version read from the ROM header (BPRE/BPGE at 0xAC,
   `decompiled/config.mk:29-57`) because the rival's preset rows differ
   (`sRivalNameChoices`, `decompiled/src/oak_speech.c:649-658` -- RED is row 1 on LG, one
   DOWN, where FR's KAZ was two wrapping UPs); the `.bk2` export writing the movie's own
   ROM name and sha1 into `Header.txt` (everything else stays the template's bytes); and
   `tools/verify-runner.sh` picking the ROM the movie header names out of
   `$FRLG_ARTIFACTS/rom` instead of playing everything on FireRed.

**Unverified:** the 9658 movie's tier-2 replay (queued), and with it the Header.txt rewrite
-- the first LeafGreen replay is what proves that format move. The crit census has not been
re-run on the new battle. The sweep tables above are tier-1 evidence.

### The six sweeps, per variant (total frames; tier-1 evidence, sweep dirs die with the sandbox)

**fr-squirtle** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10483 | 10043 | 9852 |
| 2 | 10085 | 10207 | 9960 |
| 3 | 10540 | 9951 | 9846 |
| 4 | 10386 | 9953 | 9789 |
| 5 | 10267 | 10206 | 9847 |
| 6 | 10087 | 9952 | 9929 |
| 7 | loses | 9852 | 10175 |
| 8 | 10531 | 10002 | 10117 |

**fr-charmander** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10075 | 10165 | 9945 |
| 2 | 10526 | 10162 | 9978 |
| 3 | 10076 | 10166 | 9937 |
| 4 | 10551 | 10043 | 9753 |
| 5 | 10077 | 10051 | 9938 |
| 6 | 10356 | 10167 | 9749 |
| 7 | 10165 | 10090 | 9939 |
| 8 | 10352 | 10164 | 9947 |

**fr-bulbasaur** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10205 | 10041 | 9748 |
| 2 | 10318 | 10038 | 10211 |
| 3 | 10359 | loses | loses |
| 4 | 10356 | 9776 | 9951 |
| 5 | 10360 | loses | 9861 |
| 6 | 10357 | 10230 | 9666 |
| 7 | 10358 | 9780 | 9939 |
| 8 | loses | 9780 | loses |

**lg-squirtle** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10260 | 9946 | 10105 |
| 2 | 10343 | 9747 | 9921 |
| 3 | 10261 | 9947 | 9839 |
| 4 | 10572 | 9751 | 9954 |
| 5 | 10208 | 10213 | 9845 |
| 6 | 10573 | 10228 | 9840 |
| 7 | 10578 | 10023 | 9802 |
| 8 | 10396 | 10023 | 9781 |

**lg-charmander** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10160 | 9750 | 9741 |
| 2 | 10474 | 10168 | 9947 |
| 3 | 10161 | 10280 | 9742 |
| 4 | 10475 | 10170 | 10063 |
| 5 | 10276 | 10197 | 9748 |
| 6 | 10476 | 10181 | 9932 |
| 7 | 10473 | 10167 | 9744 |
| 8 | 10476 | 10167 | 9744 |

**lg-bulbasaur** (rows `turn_hold` 1-8, columns `text_hold` 1/2/4):

| `turn_hold` | xh1 | xh2 | xh4 |
| ---: | ---: | ---: | ---: |
| 1 | 10344 | 10117 | loses |
| 2 | loses | 10227 | loses |
| 3 | 10349 | 10033 | 9858 |
| 4 | loses | 10109 | 9658 |
| 5 | 10350 | loses | 10047 |
| 6 | loses | 9766 | loses |
| 7 | 10345 | 10044 | loses |
| 8 | loses | 10048 | loses |

Sweep mechanics worth keeping: 12 parallel builds on 16 cores, ~12.5 min per build wall
clock, six 24-variant sweeps in an afternoon. `frlg route tune` serially would have been
~30 hours; `bin/frlg-sweep` exists because of that arithmetic.
