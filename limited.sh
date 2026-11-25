#!/usr/bin/env bash
# Runs a heavy command (build, tests, screenshot rendering) with memory and CPU limits, so it gets
# stopped instead of freezing the desktop when the computer runs low on memory.
#
#   ./limited.sh cargo test
#   ./limited.sh cargo test --test readme_screenshots -- --ignored
#   LIMIT_MEMORY=8G LIMIT_JOBS=6 ./limited.sh cargo build --release
#
# LIMIT_MEMORY  memory for the command and everything it starts (default: 40% of RAM)
# LIMIT_SWAP    swap it may use (default: 512M; heavy swapping is what freezes the desktop)
# LIMIT_JOBS    parallel compile jobs and software-rendering threads (default: half the CPU cores)
#
# Without a systemd user session (CI, containers) it only lowers the priority and job count.
set -uo pipefail

if [[ $# -eq 0 ]]; then
  sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
fi

total_kib=$(awk '/^MemTotal:/ { print $2 }' /proc/meminfo)
memory="${LIMIT_MEMORY:-$(( total_kib * 40 / 100 / 1024 ))M}"
swap="${LIMIT_SWAP:-512M}"
cores=$(nproc)
jobs="${LIMIT_JOBS:-$(( cores > 1 ? cores / 2 : 1 ))}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-${jobs}}"
# Threads used by Mesa's software renderers (llvmpipe, lavapipe) in headless rendering.
export LP_NUM_THREADS="${LP_NUM_THREADS:-${jobs}}"
# Rust test threads, so tests that each render a window don't all run at once.
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-${jobs}}"

if [[ -S "${XDG_RUNTIME_DIR:-/nonexistent}/bus" ]] && command -v systemd-run >/dev/null; then
  echo "limited.sh: up to ${memory} memory, ${swap} swap, ${jobs} jobs" >&2
  systemd-run --user --scope --quiet --collect \
    --property="MemoryMax=${memory}" --property="MemorySwapMax=${swap}" \
    --property=CPUWeight=20 --property=IOWeight=20 \
    -- nice -n 10 "$@"
else
  echo "limited.sh: no systemd user session, so no memory limit; ${jobs} jobs at low priority" >&2
  nice -n 10 "$@"
fi
status=$?

# 137 = killed (SIGKILL), which is what the memory limit does.
if [[ ${status} -eq 137 ]]; then
  echo "limited.sh: stopped at the ${memory} memory limit. Close other programs, or raise LIMIT_MEMORY." >&2
fi
exit "${status}"
