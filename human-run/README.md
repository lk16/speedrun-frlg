# human-run: what is here and how to read it

The task (`PROMPT.md`): find, by simulating a human, the plan that minimises the Expected Hours to
Target (EHT) for the FireRed/LeafGreen Elite Four Round 2 category (WR 3:25:25). Everything is a score
of **a model**; nothing here has been played.

## Results at a glance (see `journal/2026-11-10-15-00-conclusions.md`)

| Plan pointer | Objective | EHT |
| --- | --- | --- |
| `plans/CURRENT` | the PROMPT's rule (tighter rung wins) | EHT(WR+40m) about 85 h; WR+80m 5.6 h |
| `plans/CHAIN-WR80` | EHT at WR+80m only | **EHT(WR+80m) 4.5 h**; WR+40m 545 h |

No plan reaches the WR rung (or WR+1m ... WR+20m). The no-reset finishing time of the bulk plan is about
4:23 (p10 4:14), against a WR of 3:25. Both plans are written out in `plans/CURRENT-SUMMARY.md`.

## Files

| Path | What |
| --- | --- |
| `PROMPT.md` | the brief and the scoring rules |
| `plans/NNN-*.md` | every accepted plan or model change with its parameter block; `CURRENT`, `CHAIN-*` name the best of each search chain |
| `plans/CURRENT-SUMMARY.md` | the two best plans in words (`tools/summary.py`) |
| `scores.jsonl` | one line per evaluation (accepted, refuted, model change, sensitivity), 2000+ lines |
| `journal/` | session notebooks: what was found when (`...-starters-audit-safari.md`), conclusions and next ideas (`...-conclusions.md`) |
| `forced-audit.md` | which trainers the route cannot walk around (sight-cone audit) |
| `calibration.md` | tier-1 (libmgba) measurements of encounters and fights (`crates/frlg-calib`) |
| `data/` | `dex.json` (species, wild tables, trainers, prize money, type chart; `tools/gen_dex.py`), `pathenc.json` (encounter tiles along the route's paths), `flydist.json` (walk from the nearest Fly point to each hunting site) |
| `tools/` | `try.py` (one idea vs the incumbent, PROMPT statistics, `--rung` fixes the objective), `descend.py` (coordinate descent that promotes and commits; `--pointer`, `--rung`), `promote.py`, `batch.py`, `summary.py`, `final_table.py` (EHT at every rung), `robust.py` (holds at other jitter/reaction/mistake/text/path values?), `sitescan.py` (seconds per new species of every site), `forced.py` / `mansion.py` / `audit_all.py` (sight-cone audit), `pathenc.py`, `flydist.py` (map-derived counts), `gen_dex.py` |

## How to reproduce a number

    cargo build --release
    ./target/release/frlg-human ladder human-run/plans/<plan>.md                       # tightest rung with >= 50 finishes
    ./target/release/frlg-human eval   human-run/plans/<plan>.md --rung 7 --R 100000  # EHT at WR+80m
    ./target/release/frlg-human report human-run/plans/<plan>.md --rung 7              # per segment: time, resets, money, balls
    python3 human-run/tools/final_table.py <plan>                                       # all rungs
    python3 human-run/tools/try.py <parent.md> "idea" '{"param": value}' [--rung WR+80m]

## Reading the numbers

* **Rung.** The ladder is WR, +1, +2, +5, +10, +20, +40, +80, +160 minutes; a plan's rung is the tightest with at
  least 50 finishes. The rung rule favours lucky tails (a plan can win a tighter rung with a handful of finishes
  in hundreds of thousands of attempts while its bulk gets worse), so two chains are kept.
* **Model-change lines.** `scores.jsonl` lines whose idea starts "MODEL CHANGE" re-baseline the model (rival ids,
  type matchups, audited trainers, Safari rules, money, measured encounter and battle costs, fishing rods, grass
  steps and fill-site walks from the map). Numbers before and after one are not comparable.
* **Sensitivity lines** (idea starts SENSITIVITY / RULES ASSUMPTION / VOID) are never accepted plans.
* **Guess share.** `frlg-human report` prints the time by source (ledger, tile count, guess); about 70% of the run
  is still guess.

## Not established

Party exp split and HM slaves, healing items and PP, move choices and the second fighter (the type floor, team
bonus and loss curve are guesses and move the result by up to +70%), walking with turns, level-up and
evolution screens, mart access en route (buying is on demand), the 1.2 human factor on measured floors, and every
segment still marked Guess. RNG manipulation is not feasible for a human here (journal).
