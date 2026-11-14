# 2026-11-15: the three descent candidates that crashed frlg-human (exit 101) on plan 801

All three were `route::build` panics ("<kind> anchor X matches no segment"), not simulation bugs.

| candidate | cause | fix |
| --- | --- | --- |
| `mewtwo` True -> False | 801's `tms` teach Fire Blast / Earthquake to Mewtwo `after: "mewtwo"`, a segment that only exists when `mewtwo` is on | a teach with `mewtwo: true` is skipped when the plan does not catch Mewtwo |
| teach `MOVE_DIG` at `tm28-dig` | 801 has `no_dig: true`, so the `tm28-dig` segment does not exist | `build` returns `Err` ("... the plan sets no_dig, so TM28 is never fetched"); `frlg-human` exits 2 on an invalid plan, `try.py` logs `rejected: <msg>` instead of crashing. `descend.py` now emits the candidate as `no_dig: false` + teach (fetch TM28, pay the Rocket and text) |
| `skip_groups` + `silph-grunts` | 801's `elixirs` has a pickup `after: "silph-grunts"`; a skipped group builds no segment | `B::skipped` remembers the segment before each skipped group; every anchor (`B::find`) resolves a skipped group to it |

`build` now returns `Result` (all anchor panics became errors); unit test
`anchors_on_skipped_or_absent_segments_are_handled_not_panics`.

## Paired runs (WR+2m, R = 400k, seeds 1 and 2, parent 801 = 129.8 / 128.6 h)

| candidate | seed 1 | seed 2 | verdict |
| --- | --- | --- | --- |
| `mewtwo` False (its tms drop with it) | 7578 h, diff +7448 CI [4791, 13978] | 4929 h, CI [3290, 8806] | worse: no Mewtwo means no Champion-2 core and a dex short of 60 |
| Dig: `no_dig` false + teach Dig after TM28 | 156.1 h, +26.3 CI [19.7, 32.0] | 156.6 h, +28.0 CI [22.1, 34.7] | worse (Rocket fight + detour dwarfs a Dig teach) |
| `skip_groups` + `silph-grunts` | 116.8 h, **-13.0 CI [-17.5, -9.1]** | 112.7 h, **-15.9 CI [-20.1, -11.7]** | accepted -> plan 802-skipsilph (CURRENT) |

Caveat on 802: skipping the Silph grunts is a human choice (walk past them), consistent with how surge-gym,
mansion, blaine-quiz and the others are already skipped, but it rests on the forced-audit assumption that the card-key
doors are opened as needed and no cone is on the 1F-11F path (`human-run/forced-audit.md:40,61`). If the
grunts are forced after all, the gain is gone (that sensitivity was 5.5 -> 10.4 h at WR+80m for the four unaudited groups together).
It changes no model constant. Rows are in `scores.jsonl` (plans `x801mew1/2`, `x801dig1/2`, `x801silph1/2`).
