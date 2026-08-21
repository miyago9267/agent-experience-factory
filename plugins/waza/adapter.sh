#!/usr/bin/env bash
set -Eeuo pipefail

plugin_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
manifest="$plugin_root/manifest.yaml"
eval_file="$plugin_root/evals/personal-model/eval.yaml"

usage() {
  printf '%s\n' 'usage: adapter.sh <health|plan|run> [--output PATH] [--engine mock|waza] [--suite mock|real]'
}

waza_binary() {
  local found
  found="$(command -v waza || true)"
  if [[ -n "$found" ]]; then
    printf '%s\n' "$found"
  elif [[ -x "${HOME}/bin/waza" ]]; then
    printf '%s\n' "${HOME}/bin/waza"
  fi
}

write_result() {
  local output="$1" status="$2" summary="$3"
  [[ -n "$output" ]] || { printf 'status: %s\n' "$status"; return; }
  mkdir -p "$(dirname -- "$output")"
  {
    printf '%s\n' 'schema_version: "1"' 'kind: plugin_result' 'plugin_id: waza' 'plugin_version: 0.1.0'
    printf 'status: %s\n' "$status"
    printf '%s\n' 'task_id: personal-model-behavior-parity' 'runtime: provider-neutral' 'model: unknown'
    printf '%s\n' 'scenario: architecture,operations,chitchat' 'attempt: 2' 'exit_status: 0' 'duration_ms: 0'
    printf '%s\n' 'output_ref: null' 'tool_summary: []'
    printf 'grader_summary: "%s"\n' "$summary"
    printf '%s\n' 'graders: []' 'tasks: []'
    printf '%s\n' 'eval:' '  name: personal-model-behavior-parity' '  version: 0.1.0' "  executor: $engine" '  trials_per_task: 2'
    printf 'evidence:\n  - %s\ncapability_gaps: []\nerrors: []\n' "$eval_file"
  } > "$output"
  printf 'result_path: %s\nstatus: %s\n' "$output" "$status"
}

mode=health
output=''
engine=mock
suite=mock
while (($#)); do
  case "$1" in
    --health) mode=health; shift ;;
    plan) mode=plan; shift ;;
    run) mode=run; shift ;;
    --output) output="${2:?missing value for --output}"; shift 2 ;;
    --engine) engine="${2:?missing value for --engine}"; shift 2 ;;
    --suite) suite="${2:?missing value for --suite}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

case "$suite" in
  mock) eval_file="$plugin_root/evals/personal-model/eval.yaml" ;;
  real) eval_file="$plugin_root/evals/personal-model/eval-real.yaml" ;;
  *) printf 'unsupported suite: %s\n' "$suite" >&2; exit 2 ;;
esac

[[ -f "$manifest" && -f "$eval_file" ]] || { printf 'status: error\nmissing plugin files\n' >&2; exit 2; }

case "$mode" in
  health)
    printf 'plugin_id: waza\nplugin_type: benchmark_runner\nmanifest: OK\neval: OK\n'
    [[ -n "$(waza_binary)" ]] && printf 'waza_binary: available\n' || printf 'waza_binary: not-installed\n'
    ;;
  plan)
    printf 'plugin_id: waza\nengine: %s\nsuite: %s\neval: %s\nscenarios: architecture,operations,chitchat\ntrials_per_task: 2\n' "$engine" "$suite" "$eval_file"
    ;;
  run)
    case "$engine" in
      mock) write_result "$output" pass 'mock plan validated; no provider execution performed' ;;
      waza)
        waza_bin="$(waza_binary)"
        if [[ -z "$waza_bin" ]]; then
          write_result "$output" capability_gap 'waza executable is not installed; no benchmark execution performed'
          exit 1
        fi
        [[ -n "$output" ]] || { printf 'status: error\n--engine waza requires --output for the raw and normalized result\n' >&2; exit 2; }
        raw_output="${output}.waza.json"
        if ! "$waza_bin" run "$eval_file" --no-cache --output "$raw_output"; then
          write_result "$output" error "Waza execution failed; raw result: $raw_output"
          exit 1
        fi
        summary="$(jq -r '[.summary.succeeded, .summary.total_tests, .summary.aggregate_score] | @tsv' "$raw_output")"
        read -r succeeded total score <<< "$summary"
        status=pass
        [[ "$succeeded" == "$total" ]] || status=fail
        write_result "$output" "$status" "Waza executed $total task(s), succeeded $succeeded, aggregate score $score; raw result: $raw_output"
        exit 0
        ;;
      *) printf 'unsupported engine: %s\n' "$engine" >&2; exit 2 ;;
    esac
    ;;
esac
