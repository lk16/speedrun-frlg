# 2026-11-22: controller restart on `human-run-play`

The 2026-11-21 sandbox (frlg-3) built a playing controller and overlay renderer (S0-S6 up to
Pokemon Tower 5F) but never committed: `refs/sandboxes/frlg-3/human-run` on the host is still
3ca8859, so its code is gone. What survived in `$FRLG_ARTIFACTS`:

- `runs/human-play/1122-lgdescent-s11/run.ilog` (331053 frames, LeafGreen sha1 574fa542...) and its
  `events.jsonl`: boot to the Giovanni 1 split, seed 11. Replayed here from boot:
  ram sha1 `3d21d7d6bde6dc600e7887e353c6934cf75a4d46`, identical to
  `states/human-play/1122-lgdescent-s11/0331053-giovanni1.state`. The prefix is verified and is
  reused as is; the new controller resumes from that state.
- savestates up to `0396064-tower-5f`, but no input log past frame 331053, so everything after
  Giovanni 1 is replayed again.
- review videos (and their `.ass` panels) of the old renderer, used as the layout reference.

Old split times against the model p50 (from the surviving events): Brock 1074 s vs 877 (+197),
Misty 2301 vs 1579 (+722), Bill 3118 vs 2185 (+933), Bike 3738 vs 2521 (+1217), Rock Tunnel 4679
vs 3068 (+1611), Giovanni 1 5532 vs 3654 (+1878). Four rollbacks (naming x22 seed-table, tutorial,
Misty/rival stretch, bike voucher/rival). Rock Tunnel was walked without Flash (the sim assumes HM05).

This session: branch `human-run-play`, shared skeleton `crates/frlg-human/src/play/` (HumanFeed,
EventLog), then video overlay, controller from Giovanni 1, battle bridge and the optimisation
queue in parallel agents. Commit after every working step.
