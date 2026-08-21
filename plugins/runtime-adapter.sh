#!/usr/bin/env bash
set -Eeuo pipefail

runtime="${MIYAGO_PLUGIN_ID:-unknown}"

write_result() {
  local output="$1" status="$2" summary="$3"
  [[ -n "$output" ]] || { printf 'status: %s\n' "$status"; return; }
  mkdir -p "$(dirname -- "$output")"
  {
    printf '%s\n' 'schema_version: "1"' 'kind: plugin_result'
    printf 'plugin_id: %s\nplugin_version: 0.1.0\nstatus: %s\n' "$runtime" "$status"
    printf '%s\n' 'task_id: provider-neutral-runtime-task' "runtime: $runtime" 'model: unknown'
    printf '%s\n' 'scenario: external-session' 'attempt: 1' 'exit_status: null' 'duration_ms: null'
    printf '%s\n' 'output_ref: null' 'tool_summary: []' 'graders: []' 'tasks: []'
    printf '%s\n' 'eval:' '  name: runtime-adapter-compatibility' '  version: 0.1.0' '  executor: external' '  trials_per_task: 1'
    printf 'grader_summary: "%s"\nevidence: []\ncapability_gaps:\n  - external session bridge is not configured\nerrors: []\n' "$summary"
  } > "$output"
  printf 'result_path: %s\nstatus: %s\n' "$output" "$status"
}

mode=health
output=''
while (($#)); do
  case "$1" in
    --health) mode=health; shift ;;
    plan) mode=plan; shift ;;
    run) mode=run; shift ;;
    --output) output="${2:?missing value for --output}"; shift 2 ;;
    -h|--help) printf '%s\n' 'usage: adapter.sh <health|plan|run> [--output PATH]'; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; exit 2 ;;
  esac
done

case "$mode" in
  health)
    printf 'plugin_id: %s\nplugin_type: runtime_adapter\nmanifest: OK\nbridge: external-session\n' "$runtime"
    ;;
  plan)
    printf 'plugin_id: %s\nplugin_type: runtime_adapter\nroute: external-session\nresult: plugin_result\n' "$runtime"
    ;;
  run)
    write_result "$output" capability_gap 'runtime manifest is ready; external session bridge is not configured'
    exit 1
    ;;
esac
