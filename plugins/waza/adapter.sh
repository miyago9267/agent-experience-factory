#!/usr/bin/env bash
set -Eeuo pipefail

plugin_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
manifest="$plugin_root/manifest.yaml"
eval_file="$plugin_root/evals/personal-model-parity/eval.yaml"

usage() {
  printf '%s\n' 'usage: adapter.sh <health|plan|run> [--output PATH] [--engine mock|waza]'
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
    printf 'evidence:\n  - %s\nerrors: []\n' "$eval_file"
  } > "$output"
  printf 'result_path: %s\nstatus: %s\n' "$output" "$status"
}

mode=health
output=''
engine=mock
while (($#)); do
  case "$1" in
    --health) mode=health; shift ;;
    plan) mode=plan; shift ;;
    run) mode=run; shift ;;
    --output) output="${2:?missing value for --output}"; shift 2 ;;
    --engine) engine="${2:?missing value for --engine}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -f "$manifest" && -f "$eval_file" ]] || { printf 'status: error\nmissing plugin files\n' >&2; exit 2; }

case "$mode" in
  health)
    printf 'plugin_id: waza\nplugin_type: benchmark_runner\nmanifest: OK\neval: OK\n'
    command -v waza >/dev/null 2>&1 && printf 'waza_binary: available\n' || printf 'waza_binary: not-installed\n'
    ;;
  plan)
    printf 'plugin_id: waza\nengine: %s\neval: %s\nscenarios: architecture,operations,chitchat\ntrials_per_task: 2\n' "$engine" "$eval_file"
    ;;
  run)
    case "$engine" in
      mock) write_result "$output" pass 'mock plan validated; no provider execution performed' ;;
      waza)
        if ! command -v waza >/dev/null 2>&1; then
          write_result "$output" capability_gap 'waza executable is not installed; no benchmark execution performed'
          exit 1
        fi
        write_result "$output" inconclusive 'Waza is present but provider configuration is not supplied'
        exit 1
        ;;
      *) printf 'unsupported engine: %s\n' "$engine" >&2; exit 2 ;;
    esac
    ;;
esac
