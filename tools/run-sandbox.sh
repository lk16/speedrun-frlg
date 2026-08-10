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
DECOMP="$REPO/decompiled"

NAME="${FRLG_SANDBOX_NAME:-frlg}"
MODEL="${FRLG_MODEL:-claude-opus-5}"
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

# A stale kit is a hard error at container start rather than at create time, which
# is a slow way to find a typo. sbx validates it in a second.
sbx kit validate .sbx/kit >/dev/null || die "the kit at .sbx/kit is not valid; run: sbx kit validate .sbx/kit"

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

cat <<EOF

  sandbox   $NAME
  model     $MODEL
  limits    $CPUS cpus, $MEMORY, ${DOCKER_SANDBOXES_ROOT_SIZE} root
  decomp    $DECOMP (ro)
  deps      $DEPS (ro)
  artifacts $ARTIFACTS (rw)

  Only committed work comes back. When the session ends:
      tools/run-sandbox.sh --fetch

EOF

exec "${command[@]}"
