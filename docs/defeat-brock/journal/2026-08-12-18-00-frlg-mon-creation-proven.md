# 2026-08-12 18:00 — frlg-mon: creation rolls proven, wild model written

New crate `frlg-mon`: mon creation (gift + wild paths), IVs/natures/stat calc, exp
thresholds, and the wild-encounter per-step state machine over both RNGs (`frlg-rng` grew
`WildRng`, the `+12345` LCG). Three research notes in `docs/defeat-brock/research/` carry
the citations.

Emulator-proven today (`crates/frlg-mon/tests/emulator.rs`, replaying the committed rival-1
route):

- **The starter's creation is exactly 4 rolls and the model reproduces it bit-for-bit**:
  PID `0xbfbc8df4` (nature 14, Naive) and IVs 28/22/15/12/22/21 recovered from
  `gPlayerParty` by XOR-decrypting the substructures, matched against `gift_mon` started at
  the right stream offset. Two facts the source alone could not pin, now measured:
  `Random32()`'s **first call is the low half**, and the creation rolls land two frames
  before `gPlayerPartyCount` flips.
- **The ROM's `gExperienceTables` match the formula-derived thresholds** for MediumFast and
  MediumSlow, L2-20 (so the L10-Vine-Whip = 560 exp arithmetic in the research notes is
  fact, not macro-reading).
- **The wild RNG is seeded twice per boot, not once**: copyright screen (`intro.c:1004`)
  and title-screen exit (`title_screen.c:737`) — the live seed rides the same press that
  seeds the main stream. Research note corrected. Consequence: one frame of title delay
  re-picks the whole forest pass/fail sequence at cost 1 frame, upstream of everything.

Not yet proven: the wild-encounter step machine itself (`wild.rs`) has unit tests only —
its emulator validation needs a run that walks Route 1 grass, which is the next milestone
(post-battle segments). Its doc says so.

Next: target-parameterize `frlg-route`, then the post-battle segments (lab exit → Route 1 →
parcel round trip → tutorial → forest → Pewter → Brock), semi-naive.
