#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Full suite profiling for all Criterion benchmark cases.
#
# Notes:
# - Uses Criterion's built-in profiling mode (`--profile-time`) to emit pprof output.
# - This is intentionally separate from `benchmarks/run_profiles.sh` (hot-path set).

PROFILE_SECONDS="${PROFILE_SECONDS:-10}"
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

# store.save_session
profile_case "store" "store.save_session" "compute_only_small"
profile_case "store" "store.save_session" "io_small"
profile_case "store" "store.save_session" "compute_only_medium"
profile_case "store" "store.save_session" "io_medium"

# scenario.persist_edit
profile_case "scenario" "scenario.persist_edit" "session_25_touch_1"
profile_case "scenario" "scenario.persist_edit" "session_medium_touch_1"

# flow.layout
profile_case "flow" "flow.layout" "small"
profile_case "flow" "flow.layout" "medium_dense"
profile_case "flow" "flow.layout" "large_long_labels"

# flow.route
profile_case "flow" "flow.route" "small"
profile_case "flow" "flow.route" "medium_dense"
profile_case "flow" "flow.route" "large_long_labels"
profile_case "flow" "flow.route" "routing_stress"

# seq.layout
profile_case "seq" "seq.layout" "small"
profile_case "seq" "seq.layout" "medium"
profile_case "seq" "seq.layout" "large_long_text"

# render.sequence
profile_case "render" "render.sequence" "small"
profile_case "render" "render.sequence" "small_long_text"

# render.flow
profile_case "render" "render.flow" "small"
profile_case "render" "render.flow" "large_long_labels"

# ops.apply
profile_case "ops" "ops.apply" "seq_single"
profile_case "ops" "ops.apply" "seq_batch_10"
profile_case "ops" "ops.apply" "seq_batch_200"
profile_case "ops" "ops.apply" "flow_single"
profile_case "ops" "ops.apply" "flow_batch_10"
profile_case "ops" "ops.apply" "flow_batch_200"
