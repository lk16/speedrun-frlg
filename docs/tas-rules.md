# TAS category rules & replay-file formats

> This is the **only** doc sourced from the web. It contains *category rules* and
> *tooling facts* (allowed per `CLAUDE.md`), **not** route knowledge. Sources are
> linked inline.

## Glitchless category rules (TASVideos)

> **Category: glitchless.** The run must complete the game **without using any
> glitch**, and is optimized for time only within that constraint. This is a hard
> requirement on every route decision we derive: if a tactic relies on a glitch,
> it is off-limits, full stop.
>
> **What counts as a glitch (working definition).** Any behaviour that depends on
> the game executing outside its intended logic, including but not limited to:
> arbitrary code execution; out-of-bounds movement/data access; memory or save
> corruption; buffer/overflow exploits; the "old man" / Cinnabar-coast missingno
> tricks and any encounter-table desync; cloning, ID/PC underflow, and similar
> data exploits; sequence breaks achieved by clipping through walls/warps; and
> any input that triggers a crash or unintended state transition. Using a
> documented in-game mechanic as designed is **not** a glitch (e.g. legitimate
> menuing, intended warps, in-bounds movement, normal battle mechanics).
>
> When a tactic is genuinely on the boundary (intended-but-obscure mechanic vs.
> glitch), we treat it as a glitch and exclude it unless we can show from the
> decomp that it is intended behaviour; if real ambiguity remains we may consult
> the category rules from the web. Routing *within* these rules is still entirely
> ours to derive.

From the TASVideos movie rules and the FRLG game page:

- **Timing:** A run is timed **from power-on to the final necessary input**.
  Input may end early and let the game finish itself if no further input speeds
  up completion. → our sim starts at a clean power-on (no SRAM), no savestate.
  ([MovieRules](https://tasvideos.org/MovieRules))
- **Start condition:** Must begin **from power-on or SaveRAM** — starting from a
  savestate is disallowed (would require a from-power-on verification movie). We
  start from power-on with a cleared save. ([MovieRules](https://tasvideos.org/MovieRules))
- **End condition (glitchless):** Beat the **Elite Four and the Champion** to
  complete the game (the game then proceeds to the Hall of Fame / credits). The
  exact final-input endpoint (the input that triggers the Champion's defeat →
  Hall of Fame / credits) we will pin down **from the game's own scripts**, not
  from a route — this is a mechanics question, not a strategy one.
  ([FRLG game page](https://tasvideos.org/1478G))
- **ROM / region:** **USA v1.0** for both versions. These match the decomp's
  reference hashes:
  - FireRed (USA) v1.0 — SHA1 `41cb23d8dccc8ebd7c649cd8fbb58eeace6e2fdc`
    (`decompiled/firered.sha1`)
  - LeafGreen (USA) v1.0 — SHA1 `574fa542ffebb14be69902d1d36f1ec0a4afd71e`
    (`decompiled/leafgreen.sha1`)
  Use perfect `[!]` dumps. ([MovieRules](https://tasvideos.org/MovieRules),
  [FRLG game page](https://tasvideos.org/1478G))
- **Glitches:** **not allowed.** Our category forbids them (see the working
  definition above). The simulation should make glitchless the default and hard
  constraint — we never plan around a glitch, and any candidate route that
  depends on one is rejected.
- **FireRed vs LeafGreen:** Both run the same glitchless category; the page does
  **not** state a definitive speed reason for one over the other. That's exactly
  the open question we intend to answer ourselves by simulating both.

## Replay / movie file formats for verification

We want to emit a file that an emulator replays from power-on to reproduce our
inputs. GBA buttons we must encode: A, B, Select, Start, Right, Left, Up, Down,
R, L (10 buttons).

### Option A — VBM (VisualBoyAdvance-rr) — *simplest to write*

Binary format. ([VBM spec](https://tasvideos.org/EmulatorResources/VBA/VBM))

- 64-byte header, then a 192-byte info block (64 author + 128 description),
  then the controller-input stream.
- Header fields (offsets): `0x00` magic `56 42 4D 1A` ("VBM\x1A"); `0x04` major
  version (1); `0x08` UID (unix time); `0x0C` frame count; `0x10` rerecord
  count; `0x14` start-flags (bit0 quicksave, bit1 reset+SRAM — **we set 0 =
  power-on**); `0x15` controller flags (bit0 = controller 1); `0x16` system
  flags (**bit0 = GBA**); `0x24` 12-byte internal ROM title; `0x31` ROM CRC;
  `0x32` ROM checksum; `0x3C` controller-data offset.
- **Per frame: one 16-bit little-endian bitfield.** Bit→button:
  `0=A,1=B,2=Select,3=Start,4=Right,5=Left,6=Up,7=Down,8=R,9=L`, bit11=Reset.

  > **Key alignment:** these low-10 bits are **identical** to the game's own key
  > constants in `decompiled/include/gba/io_reg.h` (`A_BUTTON 0x0001` … `L_BUTTON
  > 0x0200`, `KEYS_MASK 0x03FF`). So our per-frame input value *is* the VBM word —
  > no remapping needed. This makes VBM trivial to generate from Rust.

### Option B — BizHawk .bk2 (mGBA core) — *modern accuracy standard*

Zip archive of UTF-8 text files. ([BK2 format](https://tasvideos.org/Bizhawk/BK2Format),
[GBA input key](https://tasvideos.org/Bizhawk/BKMFormat))

- Files: `Header` (key=value: `GameName`, `SHA1` of ROM unprefixed,
  `Platform`/`Core`, `rerecordCount`, `MovieVersion`…), `Input Log` (text),
  `Comments`, `SyncSettings` (JSON of core settings), optional `CoreState`.
- `Input Log`: first line is a Log Key (informational), then one `|`-delimited
  line per frame. GBA column order is **`|P|UDLRsSBALR|`** — `P` = power/reset
  column, then Up, Down, Left, Right, select, Start, B, A, L, R. A pressed button
  is any non-`.` char; `.` = released.
- Most accurate verification path (BizHawk's mGBA core), but the format is
  fiddlier (zip + exact SyncSettings must match the core) and column order
  differs from the game's bit order.

### Decision

- **Canonical target: VBM first.** Byte-trivial, bit-identical to the game's key
  layout, fast to iterate. Verify in VBA-rr (or mGBA, which can also play VBMs).
- **.bk2 (BizHawk + mGBA core) later** as a higher-accuracy cross-check, since
  mGBA is the accepted-accuracy emulator for GBA today.
