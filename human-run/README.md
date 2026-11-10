# human-run: what is here and how to read it

The task (`PROMPT.md`): find, by simulating a human, the plan that minimises the Expected Hours to
Target (EHT) for the FireRed/LeafGreen Elite Four Round 2 category (WR 3:25:25). Everything is a score
of **a model**; nothing here has been played.

| Path | What |
| --- | --- |
| `PROMPT.md` | the brief and the scoring rules |
| `plans/NNN-*.md` | every accepted plan or model change, with its parameter block (`plans/CURRENT` names the best) |
| `plans/CURRENT-SUMMARY.md` | the best plan in words (`tools/summary.py`) |
| `scores.jsonl` | one line per evaluation, accepted or refuted, including the model-change lines |
| `journal/` | session notebooks: what was tried, what failed, what is next |
| `forced-audit.md` | which trainers the route cannot walk around (sight-cone audit, `tools/forced.py`) |
| `calibration.md` | tier-1 (libmgba) measurements of encounters and fights (`crates/frlg-calib`) |
| `data/dex.json` | species, wild tables, trainers, prize money, type chart, generated from `decompiled/` by `tools/gen_dex.py` |
| `tools/` | `try.py` (one idea vs the incumbent with the PROMPT's statistics), `descend.py` (coordinate descent, promotes and commits), `batch.py`, `promote.py`, `summary.py`, `forced.py`, `audit_all.py`, `gen_dex.py` |

## How to reproduce a number

    cargo build --release
    ./target/release/frlg-human ladder human-run/plans/CURRENT.md ...   # (use the plan file named in plans/CURRENT)
    ./target/release/frlg-human report <plan.md> --rung 8 --R 20000    # per-segment mean, resets, money, balls
    python3 human-run/tools/try.py <parent.md> "idea" '{"param": value}' --cap 400000

## Reading the plan numbers

* **Rung.** The ladder is WR, +1, +2, +5, +10, +20, +40, +80, +160 minutes; a plan's rung is the
  tightest with at least 50 finishes. Under the calibrated model the plans sit at WR+80m to WR+160m.
  The WR rung is not reached: the earlier "WR rung" results came from uncalibrated battle costs.
* **Model-change lines.** `scores.jsonl` lines whose idea starts "MODEL CHANGE" re-baseline the model
  (rival ids, type matchups, audited trainers, Safari rules, money, measured encounter and battle costs,
  fishing rods). Numbers before and after one are not comparable.
* **Guess share.** `frlg-human report` prints the time by source (ledger, tile count, guess); most of the
  run is still guess.

## What a human would actually do (from the best plan, in order of importance)

See `plans/CURRENT-SUMMARY.md`. The big levers found, each isolated before combining:

1. Reset the starter on level-5 stats that are visible on the summary screen (SpA and HP gates).
2. Walk around every trainer the sight-cone audit says is dodgeable; fight the rest.
3. Catch species on the way only when the catch rate is at least 45; carry the dex to the Hall of Fame
   with fewer than 60 and fill the rest after, from the cheapest sites, Super Rod and Great/Ultra Balls
   bought as money allows.
4. Do the National Dex fill after the first Hall of Fame (resets before it are cheaper).
5. Use the Abra Teleport, the in-game trades and the gifts.

## Not established

Party exp split and HM slaves, healing items and PP, move choices, the Mansion and Silph door puzzles,
real step costs of walking (path overhead is a guess), mart access en route (buying is on demand),
the 1.2 human factor on measured floors, and every segment still marked Guess.
