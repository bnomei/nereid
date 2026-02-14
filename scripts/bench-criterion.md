# Criterion baseline workflow (nereid)

Goal: make it easy to run `cargo bench` with a consistent local baseline workflow so refactors can be compared apples-to-apples.

## Helper script

Use `./scripts/bench-criterion` to standardize:
- `CARGO_HOME` (defaults to `/tmp/nereid-cargo-home`)
- optional bench target selection (`--bench flow`, `--bench ops`, …)
- saving/comparing Criterion baselines (`--save-baseline`, `--baseline`)

## Recommended workflow

1) On `main`, save a baseline (name includes branch, date, and commit SHA):

```sh
./scripts/bench-criterion save
# prints: baseline: <name>
```

Optionally, focus on a single bench target while iterating:

```sh
./scripts/bench-criterion save --bench flow
```

2) On your refactor branch, compare against that saved baseline:

```sh
./scripts/bench-criterion compare --baseline <name-from-step-1>
```

Again, you can limit to a single bench target:

```sh
./scripts/bench-criterion compare --bench flow --baseline <name-from-step-1>
```

## Expectations / caveats

- Run comparisons on the same machine (and ideally similar system load / power settings).
- Benchmarks can take a while (warmup + sampling); heavier cases are expected to be slow.
- Baselines are stored locally under `target/criterion/**/<baseline-name>/` and are not meant to be committed.

## Notes

- You can override the baseline name explicitly:

```sh
./scripts/bench-criterion save --baseline main-20260208-abc123
```

- You can pass additional Criterion CLI args after `--`:

```sh
./scripts/bench-criterion run --bench flow -- <criterion-args-here>
```

## Profiling (pprof flamegraphs)

Benches attach a `pprof` profiler; flamegraphs are generated when Criterion is run in `--profile-time`
mode (which skips analysis/reporting and focuses on collecting samples).

Example: profile a single hot-path case:

```sh
PROFILE_FREQ=100 \
  ./scripts/bench-criterion run --bench store -- --profile-time 30 store.save_session/io_medium
```

Note: Criterion filters are regexes. If a case id is a prefix of another case id (e.g.
`render.sequence/small` vs `render.sequence/small_long_text`), use an anchored/escaped regex:

```sh
./scripts/bench-criterion run --bench render -- --profile-time 30 '^render\\.sequence/small$'
```

This writes `flamegraph.svg` under `target/criterion/<group>/<case>/profile/flamegraph.svg`.

To profile the standard hot-path set and copy results into `benchmarks/profiles/` (plus CSV/JSON
summaries), run:

```sh
./benchmarks/run_profiles.sh
```

To profile all benchmark cases, run:

```sh
./benchmarks/run_profiles_all.sh
```
