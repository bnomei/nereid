#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PROFILE_SECONDS="${PROFILE_SECONDS:-30}"
PROFILE_FREQ="${PROFILE_FREQ:-100}"
PROFILE_DIR="${PROFILE_DIR:-benchmarks/profiles}"

mkdir -p "$PROFILE_DIR"

run_py() {
  if command -v python3 >/dev/null 2>&1; then
    python3 "$@"
    return 0
  fi
  if command -v python >/dev/null 2>&1; then
    python "$@"
    return 0
  fi
  if command -v uv >/dev/null 2>&1; then
    UV_CACHE_DIR="${UV_CACHE_DIR:-/tmp/nereid-uv-cache}" uv run python "$@"
    return 0
  fi
  echo "error: no python interpreter found (python3/python/uv)" >&2
  exit 127
}

profile_case() {
  local bench="$1"
  local group="$2"
  local case_id="$3"

  # Criterion filters are regexes; anchor and escape group dots so we match
  # exactly one case (e.g. `render.sequence/small` would otherwise also match
  # `render.sequence/small_long_text`).
  local escaped_group="${group//./\\.}"
  local escaped_case_id="${case_id//./\\.}"
  local filter="^${escaped_group}/${escaped_case_id}$"

  echo "Profiling: bench=${bench} case=${filter}"

  PROFILE_FREQ="$PROFILE_FREQ" \
    ./scripts/bench-criterion run --bench "$bench" -- --profile-time "$PROFILE_SECONDS" "$filter"

  local src="target/criterion/${group}/${case_id}/profile/flamegraph.svg"
  if [[ ! -f "$src" ]]; then
    echo "error: expected flamegraph not found: $src" >&2
    exit 2
  fi

  local out_svg="${PROFILE_DIR}/${group}__${case_id}.svg"
  cp "$src" "$out_svg"

  local prefix="${out_svg%.svg}"
  run_py benchmarks/flamegraph_to_csv.py "$out_svg" --out-prefix "$prefix"
}

# Hot-path cases (keep in sync with perf specs 30–33).
profile_case "store" "store.save_session" "io_medium"
profile_case "store" "store.save_session" "compute_only_medium"
profile_case "scenario" "scenario.persist_edit" "session_25_touch_1"
profile_case "scenario" "scenario.persist_edit" "session_medium_touch_1"
profile_case "flow" "flow.route" "routing_stress"
profile_case "flow" "flow.route" "large_long_labels"
profile_case "render" "render.flow" "large_long_labels"
