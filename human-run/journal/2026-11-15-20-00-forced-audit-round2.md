# 2026-11-15 20:00 forced-audit round 2: the skips behind plan 803

Task: plan 803's biggest gain (-13..-16 h at WR+2m) was `skip_groups` silph-grunts, and it also skips Surge gym, Mansion and
Blaine quiz trainers. Verify each against `decompiled/`.

## What is actually skipped

Under `audited_forced` only groups built with `B::group` obey `skip_groups`; `B::forced` ignores it. So of 803's eleven names only
four did anything: silph-grunts (4), surge-gym-trainers (2), mansion-trainers (3), blaine-quiz (2). The other seven are the
cone-forced lists and were always fought.

## Findings (details and citations in `forced-audit.md`, round 2)

* Silph: new tool `human-run/tools/silph_audit.py` (card-key doors closed until the Card Key, elevator, rival/Giovanni triggers
  as forced fights). Minimum = rival + Giovanni + **11F Grunt2** (its sight-5 cone covers the only corridor tile). One grunt, not
  four; three card-key doors on the path cost ~10 s of text.
* Surge gym: **Tucker is forced** (his cone covers the three tiles under the barrier gap). The old table's "0" missed him.
* Mansion: 0 forced, skip justified.
* Blaine quiz: 0 fights but six mandatory questions (the six doors chain to Blaine); the model charged nothing.
* Hideout/Warehouse/Rock Tunnel are modelled with at least the audit's forced set.
* The WR splits/scene samples cannot say what the runner fights at these places (one sample per minute, no transcript line).

## Model change and result

`audit2` param (0 = old behaviour, so every earlier plan reproduces): 1 re-adds Silph Grunt2 and Tucker and charges
`silph-card-doors` 600 and `blaine-quiz-text` 2700 frames (both guesses); 2 also adds the rotating trainers.
WR+2m, R=800k, seeds 1/2, paired: 900-audit2 is +10.4 / +9.3 h worse than 803 (803 was optimistic) but -4.4 / -2.7 h better than
802-post-evidence; 901 (rotators too) +7.8 / +7.9 h worse than 900. `plans/CURRENT` = 900-audit2.

## Not done / next

* The 2700-frame quiz and 600-frame door costs are the largest guesses in the delta; measure on tier 1 if the decision depends on them.
* Policy was not re-tuned for 900 (reset thresholds, elixir anchors); 803's were fitted with the free skips.
* The Hideout spin/arrow tiles are not modelled by the cone search.
