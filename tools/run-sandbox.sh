#!/usr/bin/env bash
#
# Start the sandbox. This is everything the project needs from a sandbox manager;
# sbx does the rest natively.
#
#   tools/run-sandbox.sh              create if needed, then attach
#   tools/run-sandbox.sh --name X     use a specific sandbox name
#   tools/run-sandbox.sh --fetch      bring committed work back and exit
#   tools/run-sandbox.sh --rm         remove the sandbox and its git remote
#   tools/run-sandbox.sh --print      show the sbx command without running it
#
# The agent works on an in-container clone (`--clone`), so your working tree is
# never touched and only committed work can come back. See --fetch below.

set -euo pipefail

REPO="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
cd "$REPO"

# Two machine-local trees, both regenerable, both outside the repository.
# tools/host-prep.sh fills them and honours the same overrides.
DEPS="${FRLG_DEPS_DIR:-$HOME/.cache/speedrun-frlg/deps}"
ARTIFACTS="${FRLG_ARTIFACTS_DIR:-$HOME/.cache/speedrun-frlg/artifacts}"

# The decomp must live outside the repository, and this is load-bearing rather than
# tidiness: `sbx create --clone` mounts the workspace and clones it, and passing a
# second mount that is a subdirectory of that same workspace wedges create. The
# sandbox is built, its startup commands all finish, then the container disappears
# and the client blocks forever having printed nothing. Verified by elimination:
# with deps or artifacts alone create exits 0; with a nested path it hangs every time.
DECOMP="${FRLG_DECOMP_DIR:-$HOME/.cache/speedrun-frlg/decompiled}"

BASE="${FRLG_SANDBOX_NAME:-frlg}"
NAME=""            # resolved below, once the action is known
MODEL="${FRLG_MODEL:-claude-opus-5}"
# Built by tools/host-prep.sh's image step and loaded into the sandbox runtime's image
# store. Not a nicety: sbx's stock claude image has no C compiler, so on the default
# image the ROM build, the mGBA harness and every Rust link step all fail the same way.
IMAGE="${FRLG_IMAGE:-frlg-sandbox:1}"
CPUS="${FRLG_CPUS:-16}"
MEMORY="${FRLG_MEMORY:-12g}"

# sbx exposes these two only as environment variables, not as flags. The root
# filesystem has to be generous: the decomp copy, its build tree and Rust's
# target/ all live there, because artifacts is the only writable mount and a
# build tree does not belong on it.
export DOCKER_SANDBOXES_ROOT_SIZE="${FRLG_ROOT_SIZE:-24g}"
export DOCKER_SANDBOXES_DOCKER_SIZE="${FRLG_DOCKER_SIZE:-4g}"

ACTION=run
while [ $# -gt 0 ]; do
  case "$1" in
    --name)  NAME="${2:?--name needs a value}"; shift ;;
    --fetch) ACTION=fetch ;;
    --rm)    ACTION=rm ;;
    --print) ACTION=print ;;
    -h|--help) sed -n '2,14p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 1 ;;
  esac
  shift
done

die() { printf '\033[31m !! \033[0m%s\n' "$*" >&2; exit 1; }

# Names in use, from two sources. A sandbox-scoped secret outlives the sandbox it
# was made for -- it cannot be deleted, only parked (docker/sbx-releases#230) -- so
# a name whose secret still exists is spent, even with no sandbox behind it.
# Reusing one means `sbx secret set-custom` refuses and the run cannot authenticate.
existing() {
  {
    sbx ls 2>/dev/null | awk 'NR > 1 {print $1}'
    sbx secret ls 2>/dev/null | awk '{print $1}'
  } | grep -E "^$BASE-[0-9]+$" | sort -u
}

# Each run gets its own <base>-<n>, as box did. Not just for parallel sandboxes: a
# sandbox-scoped secret cannot be updated or removed once set -- `sbx secret
# set-custom` refuses with "already exists" and no rm key matches it -- so reusing
# one name would wedge its credentials permanently on the first bad token.
next_name() {
  local n=1
  while existing | grep -qx "$BASE-$n"; do n=$((n + 1)); done
  echo "$BASE-$n"
}

# For --fetch and --rm, act on the newest sandbox unless told otherwise.
latest_name() { existing | sort -t- -k2 -n | tail -1; }

if [ -z "$NAME" ]; then
  case "$ACTION" in
    fetch|rm) NAME="$(latest_name)"
              [ -n "$NAME" ] || die "no $BASE-* sandbox exists; pass --name" ;;
    *)        NAME="$(next_name)" ;;
  esac
fi

case "$ACTION" in
fetch)
  # --clone wires the sandbox up as a git remote on the host. Committed work is
  # the only thing that crosses; anything uncommitted in there stays in there.
  git remote get-url "sandbox-$NAME" >/dev/null 2>&1 \
    || die "no sandbox-$NAME remote; is the sandbox running?"
  git fetch "sandbox-$NAME"
  echo
  echo "fetched. The sandbox's work is on the remote's branches:"
  git branch -r --list "sandbox-$NAME/*"
  echo
  echo "check one out with:  git switch -c <local-name> sandbox-$NAME/<branch>"
  exit 0
  ;;
rm)
  sbx rm --force "$NAME" || true
  git remote remove "sandbox-$NAME" 2>/dev/null || true
  echo "removed sandbox $NAME"
  exit 0
  ;;
esac

# ------------------------------------------------------------- preflight ------
[ -d "$DECOMP/src" ]        || die "$DECOMP is not a pokefirered checkout"
[ -f "$DEPS/.frlg-deps" ]   || die "no toolchain tree at $DEPS -- run tools/host-prep.sh"
[ -f "$ARTIFACTS/.frlg-artifacts" ] || die "no artifacts volume at $ARTIFACTS -- run tools/host-prep.sh"
[ -f "$REPO/docs/sandbox.md" ] || die "docs/sandbox.md is missing; it is the agent's system prompt"
command -v sbx >/dev/null   || die "sbx is not on PATH"

# The image lives in sbx's image store, not the host daemon's, so `docker images` is
# not the place to look -- and a missing one fails at create time with an image pull
# error that says nothing about this project.
sbx template ls 2>/dev/null | awk -v ref="${IMAGE%:*}" -v tag="${IMAGE##*:}" \
  '$1 == ref || $1 ~ "/" ref "$" { if ($2 == tag) found = 1 } END { exit !found }' \
  || die "no sandbox image $IMAGE -- run tools/host-prep.sh (its image step builds it).
  Present: $(sbx template ls 2>/dev/null | awk 'NR > 1 {print $1 ":" $2}' | tr '\n' ' ')"

# A stale kit is a hard error at container start rather than at create time, which
# is a slow way to find a typo. sbx validates it in a second.
sbx kit validate .sbx/kit >/dev/null || die "the kit at .sbx/kit is not valid; run: sbx kit validate .sbx/kit"

token_file="${CLAUDE_OAUTH_TOKEN_FILE:-}"
if [ -n "$token_file" ]; then
  token_file="${token_file/#\~/$HOME}"
  [ -s "$token_file" ] || die "CLAUDE_OAUTH_TOKEN_FILE points at $token_file, which is missing or empty"
fi

# The three mounts. Only artifacts is writable -- everything else the agent needs
# to change, it changes inside the container. sbx defaults to read-write, so ":ro"
# is what does the work here.
mounts=(
  "$DECOMP:ro"
  "$DEPS:ro"
  "$ARTIFACTS"
)

command=(
  sbx run claude .
  --clone
  --name "$NAME"
  --template "$IMAGE"
  --cpus "$CPUS"
  --memory "$MEMORY"
  --kit .sbx/kit
  "${mounts[@]}"
  -- --model "$MODEL" --append-system-prompt "$(cat "$REPO/docs/sandbox.md")"
)

if [ "$ACTION" = print ]; then
  printf 'DOCKER_SANDBOXES_ROOT_SIZE=%s DOCKER_SANDBOXES_DOCKER_SIZE=%s\n' \
    "$DOCKER_SANDBOXES_ROOT_SIZE" "$DOCKER_SANDBOXES_DOCKER_SIZE"
  printf '%q ' "${command[@]}"; echo
  exit 0
fi

# Past the dry run, so this is the first thing with a side effect. sbx substitutes
# the real token into outbound requests to api.anthropic.com -- the container only
# ever sees a placeholder. It has to exist before create, because that is when the
# placeholder env var is injected. Passed on stdin: --value and --token would both
# land in the shell history.
if [ -n "$token_file" ]; then
  # A fresh name per run means this never collides. If it somehow does, a custom
  # secret cannot be removed (docker/sbx-releases#230), only "parked".
  sbx secret set-custom --sandbox "$NAME" --host api.anthropic.com \
    --env CLAUDE_CODE_OAUTH_TOKEN < "$token_file" >/dev/null \
    || die "could not store the OAuth token for $NAME.
  If it says the env already exists, park the old placeholder rather than removing it:
    sbx secret ls | grep $NAME
    sbx secret set-custom --sandbox $NAME --host unused.invalid --env PARKED \\
      --placeholder <that-placeholder> --value parked"
else
  # Not fatal: sbx may already hold global Anthropic credentials. Worth saying out
  # loud, because the failure mode otherwise is an agent that starts and then
  # cannot authenticate.
  printf '\033[33m !! \033[0mCLAUDE_OAUTH_TOKEN_FILE is not set; relying on sbx global credentials\n' >&2
fi

cat <<EOF

  sandbox   $NAME
  image     $IMAGE
  model     $MODEL
  limits    $CPUS cpus, $MEMORY, ${DOCKER_SANDBOXES_ROOT_SIZE} root
  decomp    $DECOMP (ro)
  deps      $DEPS (ro)
  artifacts $ARTIFACTS (rw)

  Only committed work comes back. When the session ends:
      tools/run-sandbox.sh --fetch

EOF

exec "${command[@]}"
