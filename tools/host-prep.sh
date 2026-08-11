#!/usr/bin/env bash
#
# Host-side preparation. Run this on your own machine, where there is a network;
# the sandbox has none, so everything the agent needs has to exist before it starts.
#
#   tools/host-prep.sh            build or download whatever is missing
#   tools/host-prep.sh --check    report what is present and what is not, change nothing
#   tools/host-prep.sh --force X  redo step X even if its stamp is current
#
# Steps are idempotent and stamped, so a re-run after adding one crate does not
# rebuild agbcc.
#
# Everything lands in ~/.cache/speedrun-frlg/deps, next to the artifacts volume,
# and is mounted read-only into the sandbox as a single tree. It lives outside the
# repository on purpose: it is a gigabyte of machine-local build output that can be
# regenerated at any time, which is not something a source tree should carry.
# Override with FRLG_DEPS_DIR and FRLG_ARTIFACTS_DIR if you keep them elsewhere, and
# point .box/mounts.json at wherever they ended up -- that file is what mounts them.
#
# The one exception is the `image` step, which builds the sandbox image and hands it
# to the sandbox runtime's own image store. sbx's stock image ships no compiler, so
# this is what makes the ROM build and the mGBA harness possible at all.

set -euo pipefail

# ---------------------------------------------------------------- pins --------
# Resolved refs are recorded in the deps tree's MANIFEST. Change a pin, re-run, and the
# affected step rebuilds; every sandbox created afterwards gets the new tree.
AGBCC_REF="${AGBCC_REF:-master}"          # pret/agbcc has no releases; the resolved SHA is recorded
# A tag, or a full 40-hex commit. Deliberately not "newest 0.10.x": tier 1 and tier 2 have to be
# pinned to each other by hand, and picking a moving target on our side hid that for a while. See
# the note in do_mgba for why this is not BizHawk's own core yet.
MGBA_REF="${MGBA_REF:-0.10.5}"
BIZHAWK_VER="${BIZHAWK_VER:-2.11.1}"      # must match the BizHawk you replay .bk2 files in
# The mGBA revision BizHawk $BIZHAWK_VER bundles, recorded so the delta between the two tiers is a
# written-down number rather than something a desync has to reveal. 2.11.1's submodules/mgba
# gitlink is 94b1578f8545d8ad17bb4036dba908612d5731e2 (2026-03-03), an untagged master commit that
# calls itself 0.11.0 -- there is no upstream 0.11 tag to pin to. do_bizhawk re-reads the version
# from the shipped assembly rather than trusting this line.
BIZHAWK_MGBA_COMMIT="${BIZHAWK_MGBA_COMMIT:-94b1578f8545d8ad17bb4036dba908612d5731e2}"
RUST_TOOLCHAIN="${RUST_TOOLCHAIN:-stable}" # resolved version is recorded in MANIFEST
# The minimal profile is cargo/rustc/rustdoc and nothing else, and `rustup component add`
# needs the network -- which the sandbox does not have. Anything the agent is expected to
# run on its own code has to be pulled in here or it is simply absent in there.
RUST_COMPONENTS=(clippy rustfmt)
SCCACHE_VER="${SCCACHE_VER:-0.17.0}"      # musl release tarball, so it runs on any image

# The sandbox image. sbx's stock claude image is Ubuntu 26.04 with no compiler at all --
# not gcc, not cpp, not a native as/ld, not even crt1.o -- and there is no network in the
# sandbox to add one. Extracting a toolchain into a mounted sysroot was tried and is a
# bad trade: a relocated gcc needs sysroot/-B/-isystem wrappers to find cc1, its crt
# objects and its headers, and the lib directory it drags along has to go on
# LD_LIBRARY_PATH, where it shadows the image's own OpenSSL and curl. Deriving an image
# instead is one apt-get, matches the image's own glibc exactly, and leaves
# LD_LIBRARY_PATH clean. `sbx template load` puts it in the sandbox runtime's image
# store; the `template` in .box/config.json is what names it at create time.
IMAGE_BASE="${FRLG_IMAGE_BASE:-docker.io/docker/sandbox-templates:claude-code-docker}"
IMAGE_TAG="${FRLG_IMAGE:-frlg-sandbox:1}" # must match .box/config.json's `template`
IMAGE_PKGS=(build-essential binutils-arm-none-eabi libpng-dev zlib1g-dev pkg-config cmake perl)

# .deb packages extracted into a sysroot. This is host-side only now: build.sh in the
# agbcc step needs arm-none-eabi-as and -ar, which a host has no reason to have
# installed, and extracting them beats asking for a sudo apt install. The sandbox gets
# its ARM binutils from the image, so nothing here goes on the sandbox's PATH or
# LD_LIBRARY_PATH -- see the note above for what happens when it does.
SYSROOT_PKGS=(binutils-arm-none-eabi)
SYSROOT_EXCLUDE='^(libc6|libc6-dev|libc-bin|libgcc-s1|libgcc-\d+-dev|libstdc\+\+6|libstdc\+\+-\d+-dev|gcc-\d+-base|libcrypt1|libcrypt-dev)$'

ARTIFACTS_DEFAULT="${FRLG_ARTIFACTS_DIR:-$HOME/.cache/speedrun-frlg/artifacts}"
DEPS_DEFAULT="${FRLG_DEPS_DIR:-$HOME/.cache/speedrun-frlg/deps}"
DECOMP_DEFAULT="${FRLG_DECOMP_DIR:-$HOME/.cache/speedrun-frlg/decompiled}"

# --------------------------------------------------------------- setup --------
REPO="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
DEPS="$DEPS_DEFAULT"
STAMPS="$DEPS/.stamps"
WORK="$DEPS/.work"

MODE=run
FORCE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --check) MODE=check ;;
    --force) FORCE="${2:?--force needs a step name}"; shift ;;
    -h|--help) sed -n '2,22p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 1 ;;
  esac
  shift
done

RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'; DIM=$'\033[2m'; OFF=$'\033[0m'
say()  { printf '%s==>%s %s\n' "$GREEN" "$OFF" "$*"; }
warn() { printf '%s !! %s %s\n' "$YELLOW" "$OFF" "$*" >&2; }
die()  { printf '%s !! %s %s\n' "$RED" "$OFF" "$*" >&2; exit 1; }
note() { printf '%s    %s%s\n' "$DIM" "$*" "$OFF"; }

# A step runs when its stamp does not match the pin it was built from.
stamp_ok() {
  local name="$1" want="$2"
  [ "$FORCE" != "$name" ] || return 1
  [ -f "$STAMPS/$name" ] && [ "$(cat "$STAMPS/$name")" = "$want" ]
}
stamp_set() { mkdir -p "$STAMPS"; printf '%s' "$2" > "$STAMPS/$1"; }

step() {
  local name="$1" want="$2"; shift 2
  if stamp_ok "$name" "$want"; then
    printf '  %-14s %sok%s   %s\n' "$name" "$GREEN" "$OFF" "$want"
  elif [ "$MODE" = check ]; then
    printf '  %-14s %smissing or stale%s (wants %s)\n' "$name" "$YELLOW" "$OFF" "$want"
    return 0
  else
    say "$name: $want"
    "$@"
    stamp_set "$name" "$want"
  fi
  # Every step lands in MANIFEST, whether it ran now or was already current, so the
  # file always describes the whole tree rather than the last run's diff.
  printf '%-16s %s\n' "$name" "$(cat "$DEPS/.resolved/$name" 2>/dev/null || echo "$want")" \
    >> "$DEPS/MANIFEST.new"
}

# A step records what it actually resolved its pin to -- the agbcc commit behind
# "master", the tag behind "auto". Kept per step rather than appended straight to
# MANIFEST so that a run which skips every step still writes a complete one.
manifest() { mkdir -p "$DEPS/.resolved"; printf '%s' "$2" > "$DEPS/.resolved/$1"; }

# ------------------------------------------------------------ preflight -------
need_host() {
  local missing=()
  for c in git curl tar gcc g++ make cmake pkg-config dpkg apt-get python3 docker sbx; do
    command -v "$c" >/dev/null || missing+=("$c")
  done
  [ ${#missing[@]} -eq 0 ] || die "install these on the host first: ${missing[*]}
  on Debian/Ubuntu: sudo apt install git curl build-essential cmake pkg-config python3
  docker and sbx come with Docker Desktop / Docker Sandboxes"
  command -v rustup >/dev/null || die "rustup is not installed on the host.
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  # mGBA's own build needs these headers on the host, not in the sandbox.
  pkg-config --exists libpng zlib || warn "host is missing libpng-dev / zlib1g-dev; the mgba step will fail
  sudo apt install libpng-dev zlib1g-dev"
}

# ----------------------------------------------------------------- steps ------

do_image() {
  # --pull so a forced re-run picks up sbx's current base image; the stamp is the
  # package list, so `--force image` is how you refresh after sbx ships a new one.
  # USER is restored explicitly: the base runs as `agent`, and a derived image that
  # forgets to switch back hands the agent a root sandbox.
  rm -rf "$WORK/image"; mkdir -p "$WORK/image"
  cat > "$WORK/image/Dockerfile" <<EOF
# Generated by tools/host-prep.sh. Edit IMAGE_PKGS there, not this file.
FROM $IMAGE_BASE
USER root
RUN apt-get update \\
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \\
      ${IMAGE_PKGS[*]} \\
 && rm -rf /var/lib/apt/lists/*
USER agent
EOF
  docker build --pull --provenance=false --sbom=false \
    -t "$IMAGE_TAG" "$WORK/image" >/dev/null \
    || die "docker build failed; run it by hand to see why:
  docker build --pull -t $IMAGE_TAG $WORK/image"

  # The sandbox runtime keeps its own image store, so the image has to be handed over
  # as a tar rather than just being present in the host daemon.
  docker save -o "$WORK/image/img.tar" "$IMAGE_TAG"
  sbx template load "$WORK/image/img.tar" >/dev/null \
    || die "sbx template load failed; is the sbx daemon running? (sbx diagnose)"
  rm -rf "$WORK/image"

  local user; user=$(docker image inspect -f '{{.Config.User}}' "$IMAGE_TAG")
  [ "$user" = agent ] || die "$IMAGE_TAG runs as '$user', not agent -- the USER line did not take"
  docker run --rm --entrypoint sh "$IMAGE_TAG" -c '
    for c in cc g++ ld as arm-none-eabi-as arm-none-eabi-ld make perl cmake pkg-config; do
      command -v "$c" >/dev/null || { echo "missing: $c"; exit 1; }
    done' || die "$IMAGE_TAG is missing a tool (above); check the apt output from the build"
  manifest image "$IMAGE_TAG <- $(docker image inspect -f '{{index .RepoDigests 0}}' "$IMAGE_BASE" 2>/dev/null || echo "$IMAGE_BASE")"
}

do_sysroot() {
  rm -rf "$WORK/sysroot" "$DEPS/sysroot"
  mkdir -p "$WORK/sysroot/debs" "$DEPS/sysroot"
  local list
  list=$(apt-cache depends --recurse --no-recommends --no-suggests \
           --no-conflicts --no-breaks --no-replaces --no-enhances -q "${SYSROOT_PKGS[@]}" \
         | grep -E '^\w' | sed 's/:.*//' | sort -u \
         | grep -Pv "$SYSROOT_EXCLUDE" || true)
  [ -n "$list" ] || die "apt-cache returned nothing; is the package index populated? (sudo apt update)"
  note "$(wc -l <<<"$list") packages"
  ( cd "$WORK/sysroot/debs" && apt-get download $list 2>/dev/null ) \
    || warn "some packages failed to download; frlg-doctor in the sandbox will say which tools are missing"
  for deb in "$WORK/sysroot/debs"/*.deb; do dpkg -x "$deb" "$DEPS/sysroot"; done
  rm -rf "$WORK/sysroot"
  [ -x "$DEPS/sysroot/usr/bin/arm-none-eabi-as" ] \
    || die "sysroot has no arm-none-eabi-as; binutils-arm-none-eabi did not extract"
  # The package name alone does not tie this tree to an assembler, and agbcc's build.sh
  # assembles libgcc1.a with it -- so the version belongs in the manifest next to the name.
  local as_ver
  as_ver=$("$DEPS/sysroot/usr/bin/arm-none-eabi-as" --version 2>/dev/null | head -1 | grep -oE '[0-9][0-9.]*$')
  manifest sysroot "$(printf '%s ' "${SYSROOT_PKGS[@]}")${as_ver:-unknown}"
}

do_agbcc() {
  # Depends on the sysroot step: build.sh assembles libgcc1.a and archives libc with
  # arm-none-eabi-as and -ar, which the host has no reason to have installed.
  [ -x "$DEPS/sysroot/usr/bin/arm-none-eabi-as" ] || die "run the sysroot step first"
  export PATH="$DEPS/sysroot/usr/bin:$PATH"

  rm -rf "$WORK/agbcc" "$WORK/agbcc-install" "$DEPS/agbcc"
  git clone --quiet https://github.com/pret/agbcc "$WORK/agbcc"
  git -C "$WORK/agbcc" checkout --quiet "$AGBCC_REF"
  local sha; sha=$(git -C "$WORK/agbcc" rev-parse HEAD)
  ( cd "$WORK/agbcc" && ./build.sh >/dev/null )

  # Use upstream's own installer rather than reimplementing its layout: it expects a
  # pokeemerald-shaped tree and writes <target>/tools/agbcc, which is exactly the
  # directory the sandbox drops into ~/decomp/tools/agbcc at startup.
  mkdir -p "$WORK/agbcc-install"
  ( cd "$WORK/agbcc" && ./install.sh "$WORK/agbcc-install" >/dev/null )
  mv "$WORK/agbcc-install/tools/agbcc" "$DEPS/agbcc"
  rm -rf "$WORK/agbcc" "$WORK/agbcc-install"

  for f in bin/agbcc bin/old_agbcc bin/agbcc_arm lib/libgcc.a lib/libc.a; do
    [ -e "$DEPS/agbcc/$f" ] || die "agbcc install is missing $f; check $DEPS/agbcc"
  done
  manifest agbcc "$sha"
}

do_mgba() {
  # Not pinned to BizHawk's own core, and this is a decision rather than an oversight.
  # BizHawk 2.11.1 bundles an untagged mGBA master commit that reports 0.11.0, and 0.11 removed
  # `getGameTitle`/`getGameCode` from `struct mCore` and moved `VFileOpen`, so
  # crates/mgba-sys/csrc/shim.c does not compile against it. Building 0.11.0 and pointing the
  # workspace at it fails in cc-rs at those three symbols -- measured, not assumed.
  # Until the shim is ported, the two tiers run different cores; do_bizhawk records BizHawk's
  # version and bin/frlg-doctor says the delta out loud on every startup.
  rm -rf "$WORK/mgba" "$DEPS/mgba"
  local ref="$MGBA_REF"
  mkdir -p "$WORK/mgba/src"
  if printf '%s' "$ref" | grep -qE '^[0-9a-f]{40}$'; then
    # A gitlink, which is what a BizHawk submodule gives you: --branch cannot take one, and a
    # shallow fetch of the single commit beats cloning the history to reach it.
    ( cd "$WORK/mgba/src" \
      && git init --quiet . \
      && git remote add origin https://github.com/mgba-emu/mgba \
      && git fetch --quiet --depth 1 origin "$ref" \
      && git checkout --quiet FETCH_HEAD ) \
      || die "could not fetch mGBA commit $ref"
  else
    git clone --quiet --depth 1 --branch "$ref" https://github.com/mgba-emu/mgba "$WORK/mgba/src" \
      || die "could not clone mGBA at tag $ref"
  fi
  cmake -S "$WORK/mgba/src" -B "$WORK/mgba/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$DEPS/mgba/prefix" \
    -DBUILD_QT=OFF -DBUILD_SDL=OFF -DBUILD_SHARED=ON -DBUILD_STATIC=ON \
    -DUSE_FFMPEG=OFF -DUSE_DISCORD_RPC=OFF -DUSE_LUA=OFF -DBUILD_PYTHON=OFF \
    -DUSE_LIBZIP=OFF -DUSE_MINIZIP=OFF -DUSE_SQLITE3=OFF -DUSE_ELF=OFF >/dev/null
    # Zip and the game database are off because the harness hands mGBA a plain .gba
    # path. libzip in particular is worth avoiding: Ubuntu's libzip-dev ships CMake
    # targets pointing at /usr/bin/zipcmp, which lives in the separate libzip-tools
    # package, so a stock host fails configure on a feature we never use.
    # USE_ELF=OFF for the same reason and one more: with it on, libmgba.so needs
    # libelf.so.1, the sandbox image does not ship one, and the versioned ELFUTILS_1.0
    # symbols mean no other library can stand in -- the library simply will not load.
  cmake --build "$WORK/mgba/build" -j"$(nproc)" >/dev/null
  cmake --install "$WORK/mgba/build" >/dev/null
  # Keep the source: if the prebuilt library turns out to be ABI-incompatible with the
  # sandbox image, the agent can rebuild it there against the mounted cmake in sysroot.
  mkdir -p "$DEPS/mgba"
  tar -C "$WORK/mgba" --exclude=src/.git -czf "$DEPS/mgba/src.tar.gz" src
  rm -rf "$WORK/mgba"

  # Built here, but it has to load there, and "the file exists" is not that check.
  # dlopen it inside the sandbox image, which is the only place the answer counts.
  if docker image inspect "$IMAGE_TAG" >/dev/null 2>&1; then
    docker run --rm -v "$DEPS/mgba/prefix:/mgba:ro" --entrypoint python3 "$IMAGE_TAG" \
      -c 'import ctypes; ctypes.CDLL("/mgba/lib/libmgba.so")' \
      || die "libmgba.so does not load in $IMAGE_TAG (see the error above).
  A missing library means a feature is still on: add the -DUSE_*=OFF for it, or the
  package to IMAGE_PKGS. Check what it wants with:
    docker run --rm -v $DEPS/mgba/prefix:/mgba:ro --entrypoint ldd $IMAGE_TAG /mgba/lib/libmgba.so"
  else
    warn "sandbox image $IMAGE_TAG is not built yet; skipping the libmgba load check"
  fi
  manifest mgba "$ref"
}

do_rust() {
  rm -rf "$DEPS/rustup" "$DEPS/rust"
  RUSTUP_HOME="$DEPS/rustup" CARGO_HOME="$WORK/cargo" \
    rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal --no-self-update \
      $(printf -- '--component %s ' "${RUST_COMPONENTS[@]}") >/dev/null
  local resolved
  resolved=$(ls "$DEPS/rustup/toolchains" | head -1)
  # Relative on purpose: an absolute link records the path the tree had when it was
  # built, and this tree gets moved (it used to live in <repo>/.box/deps) and mounted
  # at whatever path the host gives it. A dangling deps/rust reads as "rustc is
  # missing", which is a confusing way to say "the link text is stale".
  ln -sfn "rustup/toolchains/$resolved" "$DEPS/rust"
  [ -x "$DEPS/rust/bin/cargo" ] || die "$DEPS/rust does not resolve to a toolchain"
  # `cargo clippy` and `cargo fmt` are found as cargo-clippy and cargo-fmt on PATH, and
  # each needs its driver next to it. Checked by name because a component that failed to
  # install leaves the toolchain otherwise working, and the sandbox is the wrong place to
  # discover that -- nothing can be added there.
  for b in cargo-clippy clippy-driver cargo-fmt rustfmt; do
    [ -x "$DEPS/rust/bin/$b" ] \
      || die "$DEPS/rust/bin/$b is missing -- rustup did not install the ${RUST_COMPONENTS[*]} components"
  done
  manifest rust "$resolved (${RUST_COMPONENTS[*]})"
}

do_sccache() {
  # Rust's target/ dies with the sandbox, so without this every session pays for a cold
  # build of the whole vendored tree. The musl release is a single static binary, which
  # is what makes it safe to drop into an image we do not control.
  rm -rf "$WORK/sccache"; mkdir -p "$WORK/sccache" "$DEPS/bin"
  local url="https://github.com/mozilla/sccache/releases/download/v$SCCACHE_VER/sccache-v$SCCACHE_VER-x86_64-unknown-linux-musl.tar.gz"
  curl -fsSL "$url" | tar -C "$WORK/sccache" --strip-components=1 -xzf - \
    || die "could not download sccache $SCCACHE_VER from $url"
  install -m755 "$WORK/sccache/sccache" "$DEPS/bin/sccache"
  rm -rf "$WORK/sccache"
  "$DEPS/bin/sccache" --version >/dev/null || die "$DEPS/bin/sccache does not run"
  manifest sccache "$SCCACHE_VER"
}

do_vendor() {
  # The crate set is committed in tools/vendor-manifest/Cargo.toml. Adding a dependency
  # is a host-side act: edit that file, re-run this script, start a fresh sandbox.
  rm -rf "$DEPS/cargo-vendor"
  local m="$REPO/tools/vendor-manifest"
  [ -f "$m/Cargo.toml" ] || die "missing $m/Cargo.toml"
  mkdir -p "$m/src" && : > "$m/src/lib.rs"
  ( cd "$m" && CARGO_HOME="$WORK/cargo" RUSTUP_HOME="$DEPS/rustup" \
      cargo vendor --versioned-dirs "$DEPS/cargo-vendor" >/dev/null )
  manifest vendor "$(sha1sum "$m/Cargo.lock" | cut -c1-12)"
}

# There used to be a `wheels` step here, building $DEPS/wheels from tools/requirements.txt.
# It was removed rather than filled in: the requirements file never existed, so the step only
# ever created an empty directory, while the kit exported PIP_NO_INDEX and PIP_FIND_LINKS at it
# and made the sandbox look like it had an offline install path. It does not -- `pip install X`
# stops at PEP 668 before --no-index is consulted, and `python3 -m venv` yields an environment
# with no pip. Python here is stdlib-only, which covers what the project needs (struct, zipfile,
# hashlib, json, zlib, ctypes; a .bk2 is a zip). If that ever stops being true, add the step back
# together with a real requirements.txt -- not before.

do_bizhawk() {
  # Mounted read-only and never executed in the sandbox: BizHawk needs Mono "complete",
  # OpenAL and Lua 5.4, which is not worth relocating into a no-network container. It is
  # here so the agent can read the real GBA controller definitions and the shipped
  # config schema instead of guessing at the .bk2 format.
  rm -rf "$DEPS/bizhawk"; mkdir -p "$DEPS/bizhawk"
  local url="https://github.com/TASEmulators/BizHawk/releases/download/${BIZHAWK_VER}/BizHawk-${BIZHAWK_VER}-linux-x64.tar.gz"
  # The tarball wraps everything in BizHawk-<ver>-linux-x64/; strip it so $BIZHAWK_HOME
  # is the install root and does not move when the version does.
  curl -fsSL "$url" | tar -C "$DEPS/bizhawk" --strip-components=1 -xzf -
  [ -f "$DEPS/bizhawk/EmuHawkMono.sh" ] || die "BizHawk $BIZHAWK_VER did not extract as expected"

  # dll/libmgba.dll.so is stripped of its version symbols, which is what made this look
  # unknowable for a while. The version is not gone, it is just somewhere else: BizHawk tags the
  # core with [PortedCore("mGBA", "endrift", "<version>")], and a custom-attribute blob stores
  # those as length-prefixed UTF-8. That is what the About dialog reads, so this is the same
  # answer without a GUI or a network.
  local core_ver
  core_ver=$(python3 - "$DEPS/bizhawk/dll/BizHawk.Emulation.Cores.dll" <<'PY'
import re, sys
blob = open(sys.argv[1], "rb").read()
for m in re.finditer(rb"\x04mGBA", blob):
    i, out = m.end(), []
    for _ in range(2):                      # author, then ported version
        n = blob[i]; out.append(blob[i + 1:i + 1 + n].decode("utf8", "replace")); i += 1 + n
    if out[0] == "endrift":
        print(out[1]); break
PY
) || core_ver=""
  [ -n "$core_ver" ] || die "could not read the bundled mGBA version out of BizHawk $BIZHAWK_VER.
  bin/frlg-doctor compares it against our own pin, so an unreadable one is a real failure --
  check whether the [PortedCore] attribute moved in this release."

  # Both halves of the pair on one line, because the number that matters is the delta.
  manifest bizhawk "$BIZHAWK_VER (bundled mGBA $core_ver, submodule $BIZHAWK_MGBA_COMMIT)"
  local ours; ours=$(cat "$DEPS/.resolved/mgba" 2>/dev/null || echo "$MGBA_REF")
  if [ "$core_ver" = "$ours" ]; then
    note "bundled mGBA $core_ver matches our pin"
  else
    warn "tier-1/tier-2 core delta: ours is mGBA $ours, BizHawk $BIZHAWK_VER bundles $core_ver.
  This is the known, recorded gap (see do_mgba); frlg-doctor repeats it every startup. It stops
  being acceptable the moment a .bk2 desyncs -- port crates/mgba-sys/csrc/shim.c to 0.11 and set
  MGBA_REF=$BIZHAWK_MGBA_COMMIT."
  fi
}

do_artifacts() {
  mkdir -p "$ARTIFACTS_DEFAULT"
  for d in rom states runs scratch cache/sccache verify/queue verify/results; do
    mkdir -p "$ARTIFACTS_DEFAULT/$d"
  done
  printf 'speedrun-frlg artifacts volume\n' > "$ARTIFACTS_DEFAULT/.frlg-artifacts"
  manifest artifacts "$ARTIFACTS_DEFAULT"
}

# ------------------------------------------------------------------ run -------
# The decomp must live outside the repository, and this is load-bearing rather than
# tidiness: `sbx create --clone` mounts the workspace and clones it, and a second mount
# that is a subdirectory of that same workspace wedges create. The sandbox is built, its
# startup commands all finish, then the container disappears and the client blocks forever
# having printed nothing. Verified by elimination: with deps or artifacts alone create
# exits 0; with a nested path it hangs every time.
[ -d "$DECOMP_DEFAULT/src" ] || die "$DECOMP_DEFAULT is not a pokefirered checkout.
  It must live outside the repository: a mount nested inside the workspace hangs
  sbx create, silently and forever."
[ "$MODE" = check ] || need_host
mkdir -p "$DEPS" "$WORK"
: > "$DEPS/MANIFEST.new"
printf 'speedrun-frlg toolchain tree, built by tools/host-prep.sh\n' > "$DEPS/.frlg-deps"

printf '\n%sdeps%s  %s\n\n' "$DIM" "$OFF" "$DEPS"
step image     "$IMAGE_TAG:$(printf '%s,' "${IMAGE_PKGS[@]}")" do_image
step sysroot   "$(printf '%s,' "${SYSROOT_PKGS[@]}")" do_sysroot
step agbcc     "$AGBCC_REF"      do_agbcc
step mgba      "$MGBA_REF"       do_mgba
step rust      "$RUST_TOOLCHAIN+$(printf '%s,' "${RUST_COMPONENTS[@]}")" do_rust
step sccache   "$SCCACHE_VER"    do_sccache
step vendor    "$(sha1sum "$REPO/tools/vendor-manifest/Cargo.toml" 2>/dev/null | cut -c1-12)" do_vendor
step bizhawk   "$BIZHAWK_VER"    do_bizhawk
step artifacts "$ARTIFACTS_DEFAULT" do_artifacts

if [ "$MODE" = check ]; then
  rm -f "$DEPS/MANIFEST.new"
  echo
  [ -f "$DEPS/MANIFEST" ] && cat "$DEPS/MANIFEST"
  exit 0
fi

mv "$DEPS/MANIFEST.new" "$DEPS/MANIFEST" 2>/dev/null || rm -f "$DEPS/MANIFEST.new"
# Left by the removed `wheels` step. Always empty, and its only effect was to make the sandbox
# look like it had an offline pip.
rmdir "$DEPS/wheels" 2>/dev/null || true
rmdir "$WORK" 2>/dev/null || true

printf '\n%sdeps size%s  %s\n' "$DIM" "$OFF" "$(du -sh "$DEPS" | cut -f1)"
printf '%simage%s      %s\n' "$DIM" "$OFF" "$IMAGE_TAG (in the sbx image store; sbx template ls)"
cat <<'EOF'

Next, on the host, once:

  1. sudo apt install mono-complete libopenal1 liblua5.4-0 lsb-release
     BizHawk 2.11 needs Mono, not .NET. This is only needed on the host -- the
     sandbox never runs BizHawk.

  2. tools/bk2-template.sh
     Regenerates route/template.bk2 -- the Input Log column order and the mGBA
     SyncSettings blob, taken out of the shipped assemblies and written with
     BizHawk's own movie serialiser. Commit the result. It needs no GUI and no
     BIOS. Re-run it whenever BIZHAWK_VER moves.

  3. Put a GBA BIOS at deps/bizhawk/Firmware/GBA_bios.rom, if you want tier 2 to
     run at all. Loading a movie sets DeterministicEmulationRequested, and MGBAHawk
     then throws MissingFirmwareException("A BIOS is required for deterministic
     recordings!") -- which surfaces as the Firmware Manager dialog, not as an exit
     code, so tools/verify-runner.sh refuses to start without it.
       sha1 300C20DF6731A33952DED8C436F7F186D25D3492, 16384 bytes
     It is copyrighted and is not downloaded by this script. Note that tier 1 runs
     mGBA's HLE BIOS, so the two tiers do not boot identically until frlg is given
     the same file (Emu::load_bios) -- see docs/harness.md.

  4. box config, then box run. The mounts this script filled in are named in
     .box/config.json and given their paths on this machine in the gitignored
     .box/mounts.json; box refuses to start unless the two match.

Re-run `tools/host-prep.sh --force image` when sbx ships a new base image: the
step is stamped on the package list, so it will not notice on its own.

EOF
