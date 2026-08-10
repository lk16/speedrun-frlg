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
# tools/run-sandbox.sh reads the same two defaults; override with FRLG_DEPS_DIR
# and FRLG_ARTIFACTS_DIR if you keep them elsewhere.

set -euo pipefail

# ---------------------------------------------------------------- pins --------
# Resolved refs are recorded in the deps tree's MANIFEST. Change a pin, re-run, and the
# affected step rebuilds; every sandbox created afterwards gets the new tree.
AGBCC_REF="${AGBCC_REF:-master}"          # pret/agbcc has no releases; the resolved SHA is recorded
MGBA_REF="${MGBA_REF:-auto}"              # auto = newest 0.10.x tag; see the note in step mgba
BIZHAWK_VER="${BIZHAWK_VER:-2.11.1}"      # must match the BizHawk you replay .bk2 files in
RUST_TOOLCHAIN="${RUST_TOOLCHAIN:-stable}" # resolved version is recorded in MANIFEST

# .deb packages extracted into a sysroot, for what the sandbox image may not ship.
# libc/libstdc++/libgcc are excluded on purpose: mixing a second glibc into
# LD_LIBRARY_PATH breaks everything downstream of it.
SYSROOT_PKGS=(binutils-arm-none-eabi libpng-dev zlib1g-dev pkg-config cmake)
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
    -h|--help) sed -n '2,20p' "${BASH_SOURCE[0]}"; exit 0 ;;
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
  for c in git curl tar gcc g++ make cmake pkg-config dpkg apt-get python3; do
    command -v "$c" >/dev/null || missing+=("$c")
  done
  [ ${#missing[@]} -eq 0 ] || die "install these on the host first: ${missing[*]}
  on Debian/Ubuntu: sudo apt install git curl build-essential cmake pkg-config python3"
  command -v rustup >/dev/null || die "rustup is not installed on the host.
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  # mGBA's own build needs these headers on the host, not in the sandbox.
  pkg-config --exists libpng zlib || warn "host is missing libpng-dev / zlib1g-dev; the mgba step will fail
  sudo apt install libpng-dev zlib1g-dev libzip-dev libelf-dev"
}

# ----------------------------------------------------------------- steps ------

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
  manifest sysroot "$(printf '%s ' "${SYSROOT_PKGS[@]}")"
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
  rm -rf "$WORK/mgba" "$DEPS/mgba"
  local ref="$MGBA_REF"
  if [ "$ref" = auto ]; then
    # Stay on the same minor line BizHawk's mGBA core is on: the closer the two are,
    # the less the in-sandbox tier-1 check can disagree with the .bk2 acceptance run.
    ref=$(git ls-remote --tags --refs https://github.com/mgba-emu/mgba \
          | sed 's#.*/##' | grep -E '^0\.10\.[0-9]+$' | sort -V | tail -1)
    [ -n "$ref" ] || die "could not resolve an mGBA 0.10.x tag; set MGBA_REF explicitly"
  fi
  git clone --quiet --depth 1 --branch "$ref" https://github.com/mgba-emu/mgba "$WORK/mgba/src"
  cmake -S "$WORK/mgba/src" -B "$WORK/mgba/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$DEPS/mgba/prefix" \
    -DBUILD_QT=OFF -DBUILD_SDL=OFF -DBUILD_SHARED=ON -DBUILD_STATIC=ON \
    -DUSE_FFMPEG=OFF -DUSE_DISCORD_RPC=OFF -DUSE_LUA=OFF -DBUILD_PYTHON=OFF \
    -DUSE_LIBZIP=OFF -DUSE_MINIZIP=OFF -DUSE_SQLITE3=OFF >/dev/null
    # Zip and the game database are off because the harness hands mGBA a plain .gba
    # path. libzip in particular is worth avoiding: Ubuntu's libzip-dev ships CMake
    # targets pointing at /usr/bin/zipcmp, which lives in the separate libzip-tools
    # package, so a stock host fails configure on a feature we never use.
  cmake --build "$WORK/mgba/build" -j"$(nproc)" >/dev/null
  cmake --install "$WORK/mgba/build" >/dev/null
  # Keep the source: if the prebuilt library turns out to be ABI-incompatible with the
  # sandbox image, the agent can rebuild it there against the mounted cmake in sysroot.
  mkdir -p "$DEPS/mgba"
  tar -C "$WORK/mgba" --exclude=src/.git -czf "$DEPS/mgba/src.tar.gz" src
  rm -rf "$WORK/mgba"
  manifest mgba "$ref"
}

do_rust() {
  rm -rf "$DEPS/rustup" "$DEPS/rust"
  RUSTUP_HOME="$DEPS/rustup" CARGO_HOME="$WORK/cargo" \
    rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal --no-self-update >/dev/null
  local resolved
  resolved=$(ls "$DEPS/rustup/toolchains" | head -1)
  ln -sfn "$DEPS/rustup/toolchains/$resolved" "$DEPS/rust"
  manifest rust "$resolved"
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

do_wheels() {
  rm -rf "$DEPS/wheels"; mkdir -p "$DEPS/wheels"
  local r="$REPO/tools/requirements.txt"
  if [ -s "$r" ]; then
    python3 -m pip download -r "$r" -d "$DEPS/wheels" >/dev/null
    note "wheels are built for this host's Python; if the sandbox Python differs, prefer stdlib"
  else
    note "no tools/requirements.txt -- empty wheelhouse, stdlib only"
  fi
  manifest wheels "$( [ -s "$r" ] && sha1sum "$r" | cut -c1-12 || echo empty )"
}

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
  manifest bizhawk "$BIZHAWK_VER"

  # Its mGBA core carries no version string, so this cannot be probed -- check the
  # About dialog if the tier-1 and tier-2 checks ever disagree, and set MGBA_REF to match.
  note "bundled core: $DEPS/bizhawk/dll/libmgba.dll.so (version not embedded; ours is $(cat "$DEPS/.resolved/mgba" 2>/dev/null))"
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
[ -d "$DECOMP_DEFAULT/src" ] || die "$DECOMP_DEFAULT is not a pokefirered checkout.
  It must live outside the repository: a mount nested inside the workspace hangs
  sbx create. See the note in tools/run-sandbox.sh."
[ "$MODE" = check ] || need_host
mkdir -p "$DEPS" "$WORK"
: > "$DEPS/MANIFEST.new"
printf 'speedrun-frlg toolchain tree, built by tools/host-prep.sh\n' > "$DEPS/.frlg-deps"

printf '\n%sdeps%s  %s\n\n' "$DIM" "$OFF" "$DEPS"
step sysroot   "$(printf '%s,' "${SYSROOT_PKGS[@]}")" do_sysroot
step agbcc     "$AGBCC_REF"      do_agbcc
step mgba      "$MGBA_REF"       do_mgba
step rust      "$RUST_TOOLCHAIN" do_rust
step vendor    "$(sha1sum "$REPO/tools/vendor-manifest/Cargo.toml" 2>/dev/null | cut -c1-12)" do_vendor
step wheels    "$(sha1sum "$REPO/tools/requirements.txt" 2>/dev/null | cut -c1-12 || echo empty)" do_wheels
step bizhawk   "$BIZHAWK_VER"    do_bizhawk
step artifacts "$ARTIFACTS_DEFAULT" do_artifacts

if [ "$MODE" = check ]; then
  rm -f "$DEPS/MANIFEST.new"
  echo
  [ -f "$DEPS/MANIFEST" ] && cat "$DEPS/MANIFEST"
  exit 0
fi

mv "$DEPS/MANIFEST.new" "$DEPS/MANIFEST" 2>/dev/null || rm -f "$DEPS/MANIFEST.new"
rmdir "$WORK" 2>/dev/null || true

printf '\n%sdeps size%s  %s\n' "$DIM" "$OFF" "$(du -sh "$DEPS" | cut -f1)"
cat <<'EOF'

Next, on the host, once:

  1. sudo apt install mono-complete libopenal1 liblua5.4-0 lsb-release
     BizHawk 2.11 needs Mono, not .NET. This is only needed on the host -- the
     sandbox never runs BizHawk.

  2. Record a SyncSettings template. Open BizHawk, load the built ROM, record a
     one-frame movie, save it as route/template.bk2 and commit it. The .bk2 writer
     copies its Header and SyncSettings verbatim, so the movies the agent produces
     are guaranteed to load in the BizHawk you actually watch them in. Guessing at
     SyncSettings is the single most likely way to end up with a desyncing file.

  3. tools/run-sandbox.sh

EOF
