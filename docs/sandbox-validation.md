# Sandbox validation

First exercise of the environment produced by `tools/host-prep.sh` + `.sbx/kit/spec.yaml` +
`tools/run-sandbox.sh`. Sandbox `frlg`-series, image glibc 2.43 (Ubuntu), 16 cpus, 11 GiB RAM,
24 GiB root.

> **Status: every problem below has been fixed on the host; this file is the record of the run
> that found them, not of the environment as it stands.** The next sandbox needs a fresh run to
> confirm. What changed, and what was verified on the host in the process:
>
> | | Fix | Verified on the host |
> | --- | --- | --- |
> | P1 | The sandbox now runs an image `tools/host-prep.sh` builds from sbx's claude image plus `build-essential binutils-arm-none-eabi libpng-dev zlib1g-dev pkg-config cmake perl`, loaded with `sbx template load` and passed by `run-sandbox.sh --template`. sbx has `-t/--template`, so the sysroot-gcc plan below was not needed. | `make tools` builds all 11 tools with gcc 15; `make COMPARE=1` produces `41cb23d8…`, matching `firered.sha1`; `make syms` gives the five addresses |
> | P2 | `-DUSE_ELF=OFF` in `do_mgba()`, and the step now `dlopen`s the result *inside the image* before it will stamp itself done | `ldd` shows no `libelf`; the library loads; a C smoke test runs 3000 frames, feeds a key mask, reads EWRAM and writes a PNG of the title screen |
> | P3 | Sysroot cut to `binutils-arm-none-eabi` (host-side only, for the agbcc build) and dropped from the sandbox's `PATH` entirely; `LD_LIBRARY_PATH` is now just `$MGBA_PREFIX/lib` | `python3 -c 'import ssl'` works in the image with the profile sourced; 151 MB → 20 MB |
> | P4 | `bin/frlg-doctor` matches the proxy's own "blocked by network policy" body instead of curl's exit code — a 403 is not enough either, crates.io returns one by itself | all four verdict branches exercised, including a stubbed broken curl |
> | P5 | `deps/rust` symlink is relative | resolves; `cargo --version` through it |
> | P6 | The startup command appends `. /etc/profile.d/10-frlg.sh` to `/etc/sandbox-persistent.sh` (`BASH_ENV`/`CLAUDE_ENV_FILE`), and the profile is idempotent so per-shell sourcing cannot grow `PATH` | `env -i BASH_ENV=… bash -c` sees `FRLG_DEPS` and `cargo`; `PATH` identical three subshells deep |
> | P7 | Not fixable in the sandbox. Recorded in `docs/sandbox.md` as a standing instruction: names are authoritative, order is not derivable, wait for `route/template.bk2` | — |
> | P8 | New `sccache` step fetches the pinned musl release into `$FRLG_DEPS/bin` | `sccache --version` in the image |
> | P9 | Dead `PATH` entries gone; the `gcc -dumpmachine` triple guess is gone with the sysroot that needed it | — |
> | P10 | `docs/sandbox.md` states the 24 GiB root | — |
> | P11 | `docs/sandbox.md` records the venv-has-no-pip and PEP 668 behaviour | — |

**Bottom line: milestone 1 is blocked. The sandbox image ships no native C/C++ compiler and no
native linker, and nothing in `deps/` supplies one.** That single fact takes out the ROM build,
the symbol table, the libmgba harness *and* Rust — every executable-producing step in the project.
Two more independent problems would have bitten right after: `libmgba.so` has an unsatisfiable
`libelf.so.1` dependency, and `LD_LIBRARY_PATH` points at a 60-package sysroot that shadows the
image's own OpenSSL/curl and breaks TLS for every system tool.

## Summary

| # | Check | Result | One line |
|---|-------|--------|----------|
| 1 | `bin/frlg-doctor` | **PARTIAL** | Runs, reports 4 FAILs (gcc, g++, cargo, rustc) correctly; its network section is a false pass — it cannot detect a reachable host |
| 2 | Env plumbing | **PARTIAL** | `FRLG_*` reach a non-interactive shell by inheritance, but `PATH` additions do **not** — `arm-none-eabi-as`, `cmake`, `pkg-config` are off `PATH` unless you `bash -l` |
| 3 | Decomp copy | **PASS** | `~/decomp` present, writable, 81 MB, agbcc + libgcc.a + libc.a all there; the copy takes <1 s |
| 4 | `make -C ~/decomp tools` | **FAIL** | `make[1]: cc: No such file or directory` on the very first tool |
| 5 | ROM build | **BLOCKED** | Not attempted past check 4; no compiler, so no ROM, no SHA1 comparison, no LeafGreen, nothing in `$FRLG_ARTIFACTS/rom/` |
| 6 | Symbols | **BLOCKED** | No ROM ⇒ no `.sym`. `perl` **is** present (`/usr/bin/perl`), so the `syms` rule is not the problem |
| 7 | Disk headroom | **PASS (with a doc correction)** | Root is **24 GiB**, not 16; 22 GiB free with the decomp unpacked. Ample |
| 8 | Rust offline | **PARTIAL** | Vendored tree is **complete**: `cargo generate-lockfile --offline` locks all 111 packages. `cargo build` then dies at `linker \`cc\` not found` |
| 9 | libmgba FFI | **FAIL** | Cannot compile anything (no cc), and the library does not even `dlopen`: `libelf.so.1: cannot open shared object file` |
| 10 | Python | **PASS** | Python 3.14.4; `pip install requests` fails offline as intended. But `import ssl` is broken by check 11's cause |
| 11 | Network | **PASS** | crates.io / pypi.org / github.com / tasvideos.org all 403 "blocked by default deny policy"; api.anthropic.com reachable. Policy is correct; the *doctor's test of it* is not |
| 12 | Artifacts + budget | **PASS** | Layout correct, writable, gc table right, trimming verified end-to-end (deleted oldest-first, returned under cap) |
| 13 | BizHawk format reference | **PARTIAL** | Button **names** are authoritative from `defctrl.json`. Input Log **column order** is *not determinable* from the mounted files — it lives in compiled IL |
| 14 | Other | — | sccache referenced but never built; two dead `PATH` entries; `deps/rust` is a dangling symlink; `route/template.bk2` does not exist |

---

# Problems that block milestone 1

## P1. No native C/C++ compiler and no native linker in the image

**What failed** — three separate commands, one cause.

```
$ make -C ~/decomp tools
cc -Wall -Wextra -Werror -std=c11 -O2 bin2c.c -o bin2c
make[1]: cc: No such file or directory
make[1]: *** [Makefile:19: bin2c] Error 127
make: *** [make_tools.mk:19: tools/bin2c] Error 2
```

```
$ rustc hello.rs -o hello
error: linker `cc` not found
  |
  = note: No such file or directory (os error 2)
```

```
$ for t in cc gcc g++ cpp clang ld as; do command -v $t; done      # all empty
$ dpkg -l | grep -E 'gcc|g\+\+'
ii  gcc-16-base:amd64   16-20260322-1ubuntu1   GCC, the GNU Compiler Collection (base package)
ii  libgcc-s1:amd64     16-20260322-1ubuntu1   GCC support library
```

Only the *runtime* pieces of GCC are installed. There is no compiler driver, no `cpp`, no native
`as`, no native `ld`, no `crt1.o`/`crti.o`. `apt` cannot help — no network, and even the package
index has nothing:

```
$ sudo apt-get install -y --no-install-recommends gcc
Package gcc is not available, but is referred to by another package.
E: Package 'gcc' has no installation candidate
```

`$FRLG_DEPS/sysroot/usr/bin` contains only `arm-none-eabi-*` binutils, `cmake`, `pkg-config`,
`procps`. `linux-libc-dev` and `libc-dev-bin` did get extracted, so some headers are there, but
headers without a compiler are furniture.

**Impact** — total, and it is the whole of milestone 1:

- check 4 `make tools` — every host tool (`bin2c`, `gbagfx`, `preproc`, `scaninc`, …)
- check 5 the ROM build, and therefore the `firered.sha1` comparison and LeafGreen
- check 6 `make syms`, and therefore `gRngValue` / `gSaveBlock1` / `gSaveBlock2` / `gBattleMons` /
  `gMain` addresses — the artifact the sandbox notes call "the highest-leverage in the project"
- check 9 the libmgba harness, in C *or* in Rust
- check 8 all of Rust: `rustc` shells out to `cc` as its linker driver, so not one executable or
  build script can be produced. `cargo check` on pure-lib crates is the ceiling
- the documented in-sandbox fallback "rebuild mGBA from `$FRLG_DEPS/mgba/src.tar.gz` against the
  mounted cmake" is dead for the same reason: cmake is there, a compiler is not

**Where the fix belongs** — the sandbox image, which we do not control, so in practice
`tools/host-prep.sh`. `bin/frlg-doctor` already anticipates this exactly, under the heading
*"image (not ours -- if one of these fails, extract it into the sysroot on the host)"*.
`tools/run-sandbox.sh` runs `sbx run claude .` with no image override, so an image-level
`apt install build-essential` is not reachable from this repo unless sbx grows a way to pin a
custom image (worth checking on the host — it is the cleaner fix if it exists).

**The fix** — add the native toolchain to the sysroot step in `tools/host-prep.sh`:

```bash
SYSROOT_PKGS=(binutils-arm-none-eabi libpng-dev zlib1g-dev pkg-config
              binutils gcc-13 g++-13 cpp-13 libgcc-13-dev libstdc++-13-dev
              libc6-dev linux-libc-dev libelf1)
```

with three consequential changes:

1. `SYSROOT_EXCLUDE` currently drops `libc6-dev` and `libgcc-\d+-dev` on purpose ("mixing a second
   glibc into `LD_LIBRARY_PATH` breaks everything downstream"). That reasoning is right about
   `libc.so.6` and wrong about `libc6-dev`: we need only its *headers and crt objects*
   (`crt1.o`, `crti.o`, `crtn.o`, `/usr/include/*`), which are forward-compatible — link against
   24.04's glibc 2.39 headers, run on the image's 2.43, fine. Keep `libc6` / `libc-bin` /
   `libgcc-s1` / `libstdc++6` excluded; stop excluding `libc6-dev` and `libgcc-\d+-dev`.
2. The sysroot's runtime `lib/` must stop being prepended to `LD_LIBRARY_PATH` — see P3.
3. Add `cc`/`gcc`/`g++` entry points. `gcc-13` installs as `gcc-13`, and the decomp Makefile calls
   plain `cc`. Either symlink them in `$DEPS/sysroot/usr/bin` (`cc`→`gcc-13`, `gcc`→`gcc-13`,
   `g++`→`g++-13`) or ship small wrappers that add
   `--sysroot=$FRLG_DEPS/sysroot -B$FRLG_DEPS/sysroot/usr/lib/gcc/x86_64-linux-gnu/13`.
   Wrappers are the safer bet: a bare `gcc-13` binary will not find its own `cc1`/crt objects when
   its prefix is not `/usr`.

Verify on the host after the change with the same three commands above, and add a `cc`/`ld` row to
`bin/frlg-doctor`'s toolchain section so a regression is one line, not a session.

## P2. `libmgba.so` cannot be loaded: unsatisfiable `libelf.so.1`

**What failed**

```
$ ldd $MGBA_PREFIX/lib/libmgba.so
        libz.so.1 => .../deps/sysroot/usr/lib/x86_64-linux-gnu/libz.so.1
        libpng16.so.16 => .../deps/sysroot/usr/lib/x86_64-linux-gnu/libpng16.so.16
        libelf.so.1 => not found
        libm.so.6 => /usr/lib/x86_64-linux-gnu/libm.so.6
        libc.so.6 => /usr/lib/x86_64-linux-gnu/libc.so.6

$ python3 -c 'import ctypes,os; ctypes.CDLL(os.environ["MGBA_PREFIX"]+"/lib/libmgba.so")'
OSError: libelf.so.1: cannot open shared object file: No such file or directory
```

There is no `libelf` anywhere — not in the image, not in the sysroot. It cannot be shimmed: the
library imports versioned elfutils symbols (`ELFUTILS_1.0`), so pointing `libelf.so.1` at any other
library fails at relocation:

```
OSError: /usr/lib/x86_64-linux-gnu/libm.so.6: version `ELFUTILS_1.0' not found
         (required by .../libmgba.so)
```

**This is not the glibc/ABI mismatch the sandbox notes were braced for.** The library's maximum
required glibc symbol version is `GLIBC_2.38`; the image ships 2.43. That direction is fine. The
only unsatisfied dependency is libelf. So: **rebuild on the host, not in the sandbox** — and the
sandbox could not rebuild it anyway (P1).

**Impact** — tier-1 verification, i.e. the entire inner loop, is impossible. Currently masked by
P1 (you cannot write the harness either), but it would be the very next wall.

**Where the fix belongs** — `tools/host-prep.sh`, `do_mgba()`.

**The fix** — preferred, drop the dependency, since the harness hands mGBA a plain `.gba` path and
never loads an ELF:

```bash
  cmake -S "$WORK/mgba/src" -B "$WORK/mgba/build" \
    ... \
    -DUSE_LIBZIP=OFF -DUSE_MINIZIP=OFF -DUSE_SQLITE3=OFF \
    -DUSE_ELF=OFF                       # <-- add
```

This matches the existing comments' logic for libzip/sqlite: turn off what we do not use rather
than mount a library to satisfy it. Alternative if ELF support is ever wanted: add `libelf1`
(and its `libzstd1`) to `SYSROOT_PKGS`. Either way, add a doctor row that actually `dlopen`s the
library rather than just checking the file exists — the current `have libmgba <path>` check passes
on a library that cannot load:

```bash
chk "libmgba loads" python3 -c 'import ctypes,os;ctypes.CDLL(os.environ["MGBA_PREFIX"]+"/lib/libmgba.so")'
```

## P3. `LD_LIBRARY_PATH` poisons the image: TLS is broken for every system tool

**What failed**

```
$ curl -s https://api.anthropic.com/
curl: symbol lookup error: curl: undefined symbol: curl_multi_notify_enable, version CURL_OPENSSL_4

$ openssl version
openssl: .../deps/sysroot/usr/lib/x86_64-linux-gnu/libssl.so.3: version `OPENSSL_3.4.0' not found

$ python3 -c 'import ssl'
ImportError: .../deps/sysroot/usr/lib/x86_64-linux-gnu/libcrypto.so.3: version `OPENSSL_3.3.0'
             not found (required by /usr/lib/python3.14/lib-dynload/_ssl...so)
```

All three work the moment `LD_LIBRARY_PATH` is unset:

```
$ env -u LD_LIBRARY_PATH curl -s -o /dev/null -w '%{http_code}\n' https://api.anthropic.com/
404
$ env -u LD_LIBRARY_PATH python3 -c 'import ssl; print(ssl.OPENSSL_VERSION)'
OpenSSL 3.5.5 27 Jan 2026
```

**Cause.** `cmake` in `SYSROOT_PKGS` drags in its whole dependency closure. The sysroot is 151 MB
and 60 packages, including `libcurl4t64 libssl3 libssl3t64 libgnutls30t64 libcrypto libarchive13t64
libncursesw6 libkrb5-3 libxml2 libicu74 …` — 123 shared objects. `/etc/profile.d/10-frlg.sh`
prepends that directory to `LD_LIBRARY_PATH` for every process in the sandbox, so Ubuntu 24.04
libraries shadow the image's much newer ones for binaries built against the newer ones. Only two
libraries in there are actually needed by anything we run: `libpng16.so.16` (the image lacks it)
and `libz.so.1` (the image has its own).

**Impact** — not a milestone-1 blocker on its own, but it is a landmine and it already produced a
false PASS (P4). Anything doing TLS from a shell is broken; `git fetch`/`clone` over HTTPS would be
too. Any future harness that links a system library will get the wrong one silently.

**Where the fix belongs** — `tools/host-prep.sh` (what goes into the sysroot) and
`.sbx/kit/spec.yaml` (what goes on `LD_LIBRARY_PATH`).

**The fix** — both halves:

1. Drop `cmake` from `SYSROOT_PKGS`. It exists only for the "rebuild mGBA in the sandbox"
   fallback, which is dead without a compiler anyway, and it is what pulls in curl/OpenSSL/krb5.
   That alone removes ~55 of the 60 packages.
2. Stop putting a general-purpose lib directory on `LD_LIBRARY_PATH`. Either narrow it to a
   curated dir (host-prep symlinks only `libpng16.so.16*` and, if kept, `libelf.so.1*` into
   `$DEPS/lib`, and the profile exports `LD_LIBRARY_PATH="$DEPS/lib:$MGBA_PREFIX/lib"`), or better,
   drop `LD_LIBRARY_PATH` entirely and give `libmgba.so` an RPATH at build time
   (`-DCMAKE_INSTALL_RPATH=$DEPS/sysroot/usr/lib/x86_64-linux-gnu -DCMAKE_BUILD_WITH_INSTALL_RPATH=ON`),
   which confines the override to the one library that needs it. If `-DUSE_ELF=OFF` and a curated
   `libpng` land together, `LD_LIBRARY_PATH` can shrink to `$MGBA_PREFIX/lib` alone.

## P4. `bin/frlg-doctor`'s network check cannot detect a reachable host

**What failed** — `bin/frlg-doctor` printed:

```
network (all four should fail -- that is the point)
   ok   crates.io                  blocked
   ok   pypi.org                   blocked
   ok   github.com                 blocked
   ok   tasvideos.org              blocked
```

Every one of those is a false pass. The implementation (`bin/frlg-doctor:67`) is

```bash
if curl -sS --max-time 4 -o /dev/null "https://$h" 2>/dev/null; then ... REACHABLE ... else ... blocked
```

It is wrong in both directions at once:

- Today it says "blocked" because **curl itself is broken** (P3) and exits 127. It would print
  "blocked" for a wide-open network.
- Fix curl and it flips to the opposite error: the sandbox proxy answers a denied host with a
  **403 page**, and `curl` without `-f` treats that as success and exits 0. Verified:
  `env -u LD_LIBRARY_PATH curl -sS --max-time 4 -o /dev/null https://crates.io; echo $?` → `0`.
  Doctor would then report all four as REACHABLE.

**Impact** — the check most likely to be trusted blindly ("network is closed, good") carries no
information. Not a milestone-1 blocker; a correctness blocker for every future session's first
five minutes.

**Where the fix belongs** — `bin/frlg-doctor` (in-sandbox code).

**The fix** — test the proxy's verdict, not curl's exit code:

```bash
netchk() { # netchk <host> <want: blocked|open>
  local code
  code=$(curl -sS --max-time 6 -o /dev/null -w '%{http_code}' "https://$1" 2>/dev/null) || code=ERR
  case "$2:$code" in
    blocked:403) ok "$1" "blocked (403)" ;;
    open:2*|open:3*|open:404) ok "$1" "reachable ($code)" ;;
    *) fail "$1" "expected $2, got HTTP $code" ;;
  esac
}
```

and add `api.anthropic.com` as an expected-**reachable** row — right now nothing checks that the
one host we need actually works, which is the failure mode that leaves an agent unable to talk to
the API at all.

Ground truth today, measured with a working curl:

```
crates.io      403  Blocked by network policy: domain crates.io:443 — no matching allow rule
pypi.org       403  (same)
github.com     403  (same)
tasvideos.org  403  (same)
api.anthropic.com  404 with the Anthropic API banner body — reachable, as intended
```

The policy in `.sbx/kit/spec.yaml` is correct and is doing its job.

## P5. `$FRLG_DEPS/rust` is a dangling symlink, so `cargo` and `rustc` are not on `PATH`

**What failed**

```
$ cargo --version
bash: cargo: command not found

$ ls -l $FRLG_DEPS/rust
rust -> /home/luuk/projects/speedrun-frlg/.box/deps/rustup/toolchains/stable-x86_64-unknown-linux-gnu
$ ls /home/luuk/projects/speedrun-frlg/.box
ls: cannot access '.box': No such file or directory
```

The toolchain is present and works — it is just not where the symlink says:

```
$ $FRLG_DEPS/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo --version
cargo 1.97.1 (c980f4866 2026-06-30)
```

`do_rust()` writes an **absolute** symlink (`ln -sfn "$DEPS/rustup/toolchains/$resolved" "$DEPS/rust"`).
The deps tree was built when `DEPS` was `<repo>/.box/deps` and later moved to
`~/.cache/speedrun-frlg/deps` (commit `0afa9ad`, "drive sbx directly and drop box"); the target
followed the tree, the link text did not.

**Impact** — `cargo`/`rustc` invisible in a fresh shell. Small in absolute terms (one `PATH` entry
works around it) but it is on the critical path for the harness, and doctor reports it as a
toolchain FAIL indistinguishable from an ABI problem. Blocks milestone 1 only in combination with
P1.

**Where the fix belongs** — `tools/host-prep.sh`, `do_rust()`.

**The fix** — make the link relative so the tree is relocatable, and re-run the step:

```bash
ln -sfn "rustup/toolchains/$resolved" "$DEPS/rust"
```

then `tools/host-prep.sh --force rust`. Cheaper alternative: point the `PATH` entry in
`.sbx/kit/spec.yaml` at `$DEPS/rustup/toolchains/*/bin` and delete the symlink entirely.

## P6. `PATH` from `/etc/profile.d/10-frlg.sh` does not reach the agent's shell

**What failed** — the `FRLG_*` variables arrive, but `PATH` does not:

```
$ bash -c 'echo $FRLG_DEPS'
/home/luuk/.cache/speedrun-frlg/deps                      # inherited, fine
$ echo $PATH
/home/agent/.local/bin:/usr/local/share/npm-global/bin:/usr/local/sbin:/usr/local/bin:...
                                                          # no deps/sysroot/usr/bin, no rust
$ bash -lc 'echo $PATH'
/home/agent/.cargo/bin:.../deps/bin:.../deps/rust/bin:.../deps/sysroot/usr/bin:...
```

So `arm-none-eabi-as`, `arm-none-eabi-ld`, `cmake`, `pkg-config` and (once P5 is fixed) `cargo` are
all absent from a plain command. The exported *variables* survive because the harness captured
them once and re-exports them; `PATH` gets rewritten from the harness's own snapshot.

A truly clean shell has neither:

```
$ env -i /bin/bash -c 'echo "[$FRLG_DEPS]"'
[]
$ env -i /bin/bash -lc 'echo "[$FRLG_DEPS]"'
[/home/luuk/.cache/speedrun-frlg/deps]
```

`bin/frlg-doctor` is immune only because it sources `/etc/profile.d/10-frlg.sh` itself on line 9 —
which is why it found `arm-none-eabi-as` while an interactive `command -v arm-none-eabi-as` finds
nothing.

**Impact** — the ROM build will fail to find the ARM assembler from a non-login shell. Every script
this project writes must either source the profile or be run under `bash -l`. Cheap to work around,
expensive to keep rediscovering.

**Where the fix belongs** — `.sbx/kit/spec.yaml`, the startup command that writes the profile.

**The fix** — write the same block to the file the harness sources before *every* command, in
addition to `profile.d`. `CLAUDE_ENV_FILE` and `BASH_ENV` are both already set to
`/etc/sandbox-persistent.sh`, and that file is currently **empty**:

```sh
sudo tee /etc/profile.d/10-frlg.sh >/dev/null <<EOF
...
EOF
# and:
grep -q 10-frlg /etc/sandbox-persistent.sh 2>/dev/null \
  || echo '. /etc/profile.d/10-frlg.sh' | sudo tee -a /etc/sandbox-persistent.sh >/dev/null
```

(Only the plain `.` line — never a completion script; see the project CLAUDE.md.) Keep the
existing `~/.bashrc` append as well. Then every `bash -c`, every `Makefile` recipe and every
subprocess sees the full `PATH`.

---

# Problems that do not block milestone 1

## P7. The `.bk2` Input Log column order is not determinable from the mounted BizHawk

**Button names — authoritative.** `$BIZHAWK_HOME/defctrl.json`, key
`AllTrollers` → `"GBA Controller"`, verbatim:

```json
"GBA Controller": {
    "Up":     "Up, J1 POV1U, X1 DpadUp, X1 LStickUp",
    "Down":   "Down, J1 POV1D, X1 DpadDown, X1 LStickDown",
    "Left":   "Left, J1 POV1L, X1 DpadLeft, X1 LStickLeft",
    "Right":  "Right, J1 POV1R, X1 DpadRight, X1 LStickRight",
    "Start":  "Enter, J1 B10, X1 Start",
    "Select": "Space, J1 B9, X1 Back",
    "B":      "Z, J1 B1, X1 X",
    "A":      "X, J1 B2, X1 A",
    "L":      "W, J1 B5, X1 LeftShoulder",
    "R":      "E, J1 B6, X1 RightShoulder"
}
```

So the exact names are `Up Down Left Right Start Select B A L R`, and the controller is called
`GBA Controller`. (`BizHawk.Emulation.Cores.dll` also contains the string `Subframe GBA Controller`
alongside `GBA Controller`, `Tilt {0}`, `Tilt Z` — the mGBA core exposes tilt/light-sensor analog
axes too, which a full `Input Log` line may carry. Note also `Power`, which BizHawk's GBA
definition normally carries as a boolean button, is **not** bound in `defctrl.json` — absence of a
default binding is not evidence of absence from the column list.)

**Column order — could not be determined, and I am not going to infer it.** `defctrl.json` gives a
*binding* order, not the movie's column order; those coincide often enough to be a trap. The
authority is `ControllerDefinition.OrderedControlsFlat` for the mGBA core, which lives in compiled
CIL inside `$BIZHAWK_HOME/dll/BizHawk.Emulation.Cores.dll`. I extracted the UTF-16 string heaps
from `BizHawk.Emulation.Cores.dll` and `BizHawk.Client.Common.dll`; the button-name strings are
interned once and shared, so their heap positions carry no ordering information, and the mnemonic
tables are metadata blobs, not readable text. There is no `ildasm`/`monodis` in the sandbox and
BizHawk must not be executed here. No sample `.bk2` ships with the release
(`find $BIZHAWK_HOME -iname '*.bk2'` → nothing), and `route/template.bk2` **does not exist in the
repo**, though `docs/sandbox.md` tells the agent to copy `Header`/`SyncSettings` from it verbatim.

**Impact** — blocks emitting a *correct* `.bk2`, which is milestone 1's last step. Does not block
routing, tier-1 verification, or the raw `u16` input log, which stays the canonical artifact.

**Where the fix belongs** — the host.

**The fix** — record a real one-frame GBA movie in the BizHawk 2.11.1 that will replay our work,
and commit it as `route/template.bk2`. It settles `Header`, `SyncSettings`, the column order and
the `LogKey` line all at once, and it is the thing `docs/sandbox.md` already assumes exists.
Second-best: run `monodis --output=- BizHawk.Emulation.Cores.dll | grep -A40 'GBA Controller'` on
the host and paste the `BoolButtons` initialiser order into `docs/`.

## P8. sccache is configured but does not exist

`/etc/profile.d/10-frlg.sh` exports `SCCACHE_DIR` and `SCCACHE_CACHE_SIZE=4G`; the artifacts
layout reserves `cache/sccache` with a 4 GiB cap and `bin/frlg-artifacts-gc` reports on it; the
cargo-config startup command guards on `[ -x "$FRLG_DEPS/bin/sccache" ]`. But `$FRLG_DEPS/bin`
does not exist and `tools/host-prep.sh` has no step that builds or downloads sccache
(`grep -n sccache tools/host-prep.sh` → only the artifacts-layout `mkdir`). The guard means this
fails silently: no wrapper is written, `target/` rebuilds from scratch every sandbox, and 4 GiB of
budget is reserved for an empty directory.

**Where the fix belongs** — `tools/host-prep.sh`. **The fix** — either add a `sccache` step
(`cargo install --root "$DEPS" sccache`, or fetch the release tarball into `$DEPS/bin`) or delete
the three references so the next reader is not misled.

## P9. Two dead `PATH` entries, and one lucky fallback

- `$FRLG_DEPS/bin` — does not exist (P8).
- `$FRLG_DEPS/mgba/prefix/bin` — does not exist; correct, since `do_mgba()` builds with
  `-DBUILD_QT=OFF -DBUILD_SDL=OFF`, so mGBA installs no binaries. Harmless, but it means there is
  no `mgba` CLI to sanity-check the ROM with.
- The startup command computes `TRIPLE=$(gcc -dumpmachine 2>/dev/null || echo x86_64-linux-gnu)`.
  `gcc` does not exist, so it silently took the fallback — which happens to be right on this host.
  On an arm64 host it would produce a `LIBRARY_PATH`/`PKG_CONFIG_PATH` pointing at a directory that
  does not exist, with no error. Use `dpkg-architecture -qDEB_HOST_MULTIARCH` or, simpler,
  glob `"$DEPS"/sysroot/usr/lib/*-linux-gnu` and fail loudly if it does not resolve to exactly one.

## P10. `docs/` and the agent prompt overstate the root filesystem constraint

`docs/sandbox.md` (and the task brief) say the sandbox has a 16 GiB root and that raising it makes
`sbx create` hang. Both are stale. `tools/run-sandbox.sh:43` reads
`FRLG_ROOT_SIZE:-24g`, commit `fa4b726` explicitly retracted the 16g diagnosis ("the hang was a
nested mount, not the root size … 24g is fine and is restored here"), and the running sandbox
confirms it:

```
$ df -h /
Filesystem      Size  Used Avail Use% Mounted on
overlay          24G  385M   22G   2% /
```

`~/decomp` is 81 MB unpacked (the read-only source is 219 MB including `.git`; the copy excludes
it). With 22 GiB free, a pokefirered build tree plus a Rust `target/` is not close to a problem.
**Fix**: correct the paragraph in `docs/sandbox.md`.

## P11. Python has no `pip` in venvs, and `ssl` is broken

`python3 -V` → **Python 3.14.4**. Offline behaviour is exactly as designed:

```
$ python3 -m pip install --break-system-packages requests
Looking in links: /home/luuk/.cache/speedrun-frlg/deps/wheels
ERROR: Could not find a version that satisfies the requirement requests (from versions: none)
ERROR: No matching distribution found for requests
```

(A plain `pip install requests` stops earlier, at PEP 668 "externally-managed-environment", which
masks the offline check — worth knowing when reading a failure.) `$FRLG_DEPS/wheels` is empty as
documented, and `tools/requirements.txt` does not exist, so the `wheels` step is a no-op.

Two notes. `python3 -m venv` succeeds but produces an environment **without pip**
(`No module named pip`) — `ensurepip` cannot reach an index. And `import ssl` fails under the
project's `LD_LIBRARY_PATH` (P3). Neither matters for the plan: **nothing in milestone 1 needs a
non-stdlib Python package.** Python's role here is scripting and file surgery — `struct`, `zipfile`
(`.bk2` is a zip), `hashlib` (SHA1 for the header), `json`, `zlib`. `zipfile` and `hashlib` cover
what the vendored `zip`/`sha1` crates cover on the Rust side. Keep the wheelhouse empty.

---

# What passed, with the evidence

**Check 1 — doctor.** Runs clean, exit 1. Full output at the time of the run:

```
mounts
   ok   deps                       /home/luuk/.cache/speedrun-frlg/deps
   ok   decomp (ro)                /home/luuk/.cache/speedrun-frlg/decompiled
   ok   decomp (rw copy)           /home/agent/decomp/Makefile
   ok   artifacts                  /home/luuk/.cache/speedrun-frlg/artifacts/.frlg-artifacts
   ok   artifacts is writable

image (not ours -- if one of these fails, extract it into the sysroot on the host)
   FAIL gcc                        bin/frlg-doctor: line 17: gcc: command not found
   FAIL g++                        bin/frlg-doctor: line 17: g++: command not found
   ok   make                       GNU Make 4.4.1
   ok   python3                    Python 3.14.4
   ok   perl
   ok   bash                       GNU bash, version 5.3.9(1)-release (x86_64-pc-linux-gnu)
   ok   tar                        tar (GNU tar) 1.35
   ok   git                        git version 2.53.0
   ok   pkg-config                 1.8.1

toolchain (prebuilt on the host -- a FAIL here is usually a glibc/ABI mismatch)
   ok   arm-none-eabi-as           GNU assembler (2.42-1ubuntu1+23) 2.42
   ok   arm-none-eabi-ld           GNU ld (2.42-1ubuntu1+23) 2.42
   ok   agbcc                      agbcc: Invalid option `--version'
   ok   agbcc in decomp            /home/agent/decomp/tools/agbcc/bin/agbcc
   FAIL cargo                      bin/frlg-doctor: line 17: cargo: command not found
   FAIL rustc                      bin/frlg-doctor: line 17: rustc: command not found
   ok   vendored crates            /home/luuk/.cache/speedrun-frlg/deps/cargo-vendor
   ok   libmgba                    /home/luuk/.cache/speedrun-frlg/deps/mgba/prefix/lib/libmgba.so
   ok   mgba headers               .../mgba/prefix/include/mgba/core/core.h
   ok   bizhawk (ref)              /home/luuk/.cache/speedrun-frlg/deps/bizhawk
   ok   libpng                     1.6.43

network (all four should fail -- that is the point)
   ok   crates.io                  blocked
   ok   pypi.org                   blocked
   ok   github.com                 blocked
   ok   tasvideos.org              blocked

artifacts budget

  dir                  used      cap
  rom                   0 M      1 G
  states                0 M      3 G
  scratch               0 M      2 G
  runs                  0 M      4 G
  cache/sccache         0 M      4 G

  total                 0 G  (hard ceiling 20 G)

 something is wrong above
```

Its four FAILs are all real (P1, P5). Two of its passes are not trustworthy: the network block
(P4) and `libmgba` (P2 — the file exists, the library does not load). `agbcc` shows `ok` on
`agbcc: Invalid option '--version'` because agbcc exits 0 on it; cosmetic, but a stricter probe
would be better.

**Check 3 — decomp copy.** `~/decomp` exists, is writable, 81 MB / 9,924 files, and contains all
three agbcc artifacts:

```
-rwxrwxr-x 3793256  tools/agbcc/bin/agbcc
-rwxrwxr-x 4942680  tools/agbcc/bin/agbcc_arm
-rw-rw-r--   46956  tools/agbcc/lib/libgcc.a
-rw-rw-r--  298708  tools/agbcc/lib/libc.a
```

Re-running the startup command's exact `tar | tar` pipeline took **0.36 s** warm (68 MB before
agbcc is layered in). Even cold over virtiofs this is seconds, not minutes — it is not worth
optimising or caching.

Mounts are enforced as documented — `touch` fails with `Read-only file system` on both
`$FRLG_DECOMP_RO` and `$FRLG_DEPS` (including `$BIZHAWK_HOME`), and `$FRLG_ARTIFACTS` is writable:

```
host /home/luuk/.cache/speedrun-frlg/decompiled virtiofs ro,nosuid,nodev,relatime
host /home/luuk/.cache/speedrun-frlg/deps       virtiofs ro,nosuid,nodev,relatime
host /home/luuk/.cache/speedrun-frlg/artifacts  virtiofs rw,nosuid,nodev,relatime
```

**Check 8 — the vendored crate tree is complete.** This is the good news of the session. Built a
scratch crate under `$HOME` with every dependency from `tools/vendor-manifest/Cargo.toml`
(anyhow, thiserror, clap+derive, serde+derive, serde_json, rayon, libc, zip, sha1, crc32fast, png,
hex) and resolved it with the network off:

```
$ cargo generate-lockfile --offline
     Locking 111 packages to latest compatible versions
```

111 packages, matching the 111 directories in `$FRLG_DEPS/cargo-vendor` exactly — every transitive
dependency and every requested feature is present. Nothing to fix host-side. Compilation then gets
through 18 crates before the first build script needs to link:

```
   Compiling libc v0.2.189
   ... 17 more ...
error: linker `cc` not found
error: could not compile `crossbeam-epoch` (build script) due to 1 previous error
```

That is P1, not a vendoring problem.

**Check 12 — artifacts and the budget, verified by actually breaking it.** Layout is exactly as
`do_artifacts()` and the startup command create it: `rom states runs scratch cache/sccache
verify/queue verify/results`, all writable, `scratch/` empty (swept at startup as documented).

To test the trim path I wrote five 1000 MB sparse files into `runs/junk-test-{a..e}` with
ascending mtimes (sparse because `frlg-artifacts-gc` measures with `du --apparent-size`, so 5 GB
of accounting cost 24 KB of real disk):

```
$ bin/frlg-artifacts-gc --check
  runs               5000 M      4 G  OVER
EXIT=1                                          # correct: --check reports and changes nothing

$ bin/frlg-artifacts-gc
  runs               5000 M      4 G  OVER
  rm junk-test-a                                # oldest mtime, deleted first
  total                 3 G  (hard ceiling 20 G)
EXIT=0

$ ls $FRLG_ARTIFACTS/runs
junk-test-b junk-test-c junk-test-d junk-test-e   # 4000 M, under the 4 G cap
```

Oldest-first, whole entries, stops as soon as it is under budget, `--check` is non-destructive,
exit codes right. Junk removed afterwards; `runs/` is empty and the gc reports 0 M across the
board. The budget works.

**Check 11 — the network policy is correct.** Measured with a working curl (`env -u
LD_LIBRARY_PATH`), all four denied hosts return the proxy's structured 403 —
`Blocked by network policy: domain <host>:443 / detail: no matching allow rule — blocked by
default deny policy` — and `api.anthropic.com` returns the API's own 404 banner. No leaks. Also
checked and blocked: `static.crates.io`, `files.pythonhosted.org`, `raw.githubusercontent.com`.

---

# What I would fix first

1. **A native compiler (P1).** Nothing else matters until `cc` exists — it is the single cause of
   four of the five milestone-1 blockers. If sbx can pin an image with `build-essential`, that is
   the clean fix; otherwise extend `SYSROOT_PKGS` with `gcc-13 g++-13 cpp-13 binutils libc6-dev
   libgcc-13-dev libstdc++-13-dev`, stop excluding `libc6-dev`, and add `cc`/`gcc`/`g++` wrappers.
   Everything below is cheap by comparison and can ride the same host-prep re-run.
2. **`-DUSE_ELF=OFF` on mGBA (P2).** One cmake flag. Without it the harness cannot load the
   library even after P1 is fixed, and the "rebuild it in the sandbox" escape hatch does not exist.
3. **Shrink the sysroot and stop poisoning `LD_LIBRARY_PATH` (P3).** Dropping `cmake` from
   `SYSROOT_PKGS` fixes TLS for the whole sandbox and removes ~55 packages. Do it in the same
   host-prep pass as 1 and 2, since all three touch that step.
4. **Fix the doctor's network check and add a `dlopen` check for libmgba (P4, P2).** Two small
   edits to `bin/frlg-doctor`. Everything else in this report was found in spite of doctor saying
   the network was fine and libmgba was fine; that is the gap worth closing before the next
   session trusts it.
5. **Relative `deps/rust` symlink (P5) and the `sandbox-persistent.sh` line (P6).** Two one-liners
   that stop the next session from re-deriving them.
6. **Record `route/template.bk2` on the host (P7).** Not urgent — no route exists yet — but it is
   pure host-side work that unblocks the last step of milestone 1 and settles a format question we
   have so far only assumed. Cheapest possible answer to the most expensive possible mistake.

Not attempted, and still unknown after this session: whether the ROM builds byte-identical to
`firered.sha1`, whether `make syms` produces usable addresses, whether LeafGreen's version switch
works, whether libmgba's frame stepping and EWRAM reads behave, and what a boot-screen framebuffer
looks like. All five are gated on P1. Nothing was copied into `$FRLG_ARTIFACTS/rom/`; it is empty.
