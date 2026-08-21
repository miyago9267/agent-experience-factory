#!/usr/bin/env bash
set -Eeuo pipefail

plugin_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
manifest="$plugin_root/manifest.yaml"
eval_file="$plugin_root/evals/personal-model/eval.yaml"

usage() {
  printf '%s\n' 'usage: adapter.sh <health|plan|run> [--output PATH] [--engine mock|waza] [--suite mock|real] [--context-task TASK_ID] [--context-cwd PATH]'
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

write_waza_result() {
  local output="$1" raw_output="$2" eval_path="$3" context_task="$4"
  local succeeded total score status
  succeeded="$(jq -r '.summary.succeeded' "$raw_output")"
  total="$(jq -r '.summary.total_tests' "$raw_output")"
  score="$(jq -r '.summary.aggregate_score' "$raw_output")"
  status=pass
  [[ "$succeeded" == "$total" ]] || status=fail
  jq -r --arg raw "$raw_output" --arg eval_path "$eval_path" --arg status "$status" --arg context_task "$context_task" '
    def yaml_quote: @json;
    [
      "schema_version: \"1\"",
      "kind: plugin_result",
      "plugin_id: waza",
      "plugin_version: 0.1.0",
      ("status: " + $status),
      ("task_id: " + (.eval_name | yaml_quote)),
      ("runtime: " + (.config.engine_type | yaml_quote)),
      ("model: " + (.config.model_id | yaml_quote)),
      "scenario: all",
      ("attempt: " + (.config.runs_per_test | tostring)),
      "exit_status: 0",
      ("duration_ms: " + (.summary.duration_ms | tostring)),
      ("output_ref: " + ($raw | yaml_quote)),
      "tool_summary: []",
      ("grader_summary: " + (("Waza executed " + (.summary.total_tests | tostring) + " task(s), succeeded " + (.summary.succeeded | tostring) + ", aggregate score " + (.summary.aggregate_score | tostring)) | yaml_quote)),
      "graders:",
      "  - name: aggregate",
      ("    passed: " + ((.summary.failed == 0 and .summary.errors == 0) | tostring)),
      ("    score: " + (.summary.aggregate_score | tostring)),
      ("    feedback: " + (("" + (.summary.succeeded | tostring) + "/" + (.summary.total_tests | tostring) + " tasks succeeded") | yaml_quote)),
      "tasks:",
      (.tasks[] |
        "  - id: " + (.test_id | yaml_quote),
        "    passed: " + ((.status == "passed") | tostring),
        "    attempts: " + (.runs | length | tostring),
        "    trials:",
        (.runs[] |
          "      - attempt: " + (.run_number | tostring),
          "        passed: " + ((.status == "passed") | tostring),
          "        score: " + (([.validations[]?.score] | add // 0) / ([.validations[]?.score] | length // 1) | tostring)
        )
      ),
      "eval:",
      ("  name: " + (.eval_name | yaml_quote)),
      "  version: 0.1.0",
      ("  executor: " + (.config.engine_type | yaml_quote)),
      ("  trials_per_task: " + (.config.runs_per_test | tostring)),
      "evidence:",
      ("  - " + ($eval_path | yaml_quote)),
      ("  - " + ($raw | yaml_quote)),
      ("  - context task validated: " + ($context_task | yaml_quote)),
      "capability_gaps: []",
      "errors: []"
    ] | .[]
  ' "$raw_output" > "$output"
  printf 'result_path: %s\nstatus: %s\n' "$output" "$status"
}

mode=health
output=''
engine=mock
suite=mock
context_task=''
context_cwd="$PWD"
while (($#)); do
  case "$1" in
    --health) mode=health; shift ;;
    plan) mode=plan; shift ;;
    run) mode=run; shift ;;
    --output) output="${2:?missing value for --output}"; shift 2 ;;
    --engine) engine="${2:?missing value for --engine}"; shift 2 ;;
    --suite) suite="${2:?missing value for --suite}"; shift 2 ;;
    --context-task) context_task="${2:?missing value for --context-task}"; shift 2 ;;
    --context-cwd) context_cwd="${2:?missing value for --context-cwd}"; shift 2 ;;
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
        [[ -n "$context_task" ]] || { printf 'status: error\n--engine waza requires --context-task for scope binding\n' >&2; exit 2; }
        context_binary="${MIYAGO_CONTEXT_HARNESS_BIN:-${HOME}/.local/bin/miyago-context-harness}"
        context_root="${MIYAGO_AGENT_WORKSPACE_ROOT:-${HOME}/Project/AI/agent-workspace}"
        "$context_binary" plan --workspace-root "$context_root" --task "$context_task" --cwd "$context_cwd" >/dev/null
        raw_output="${output}.waza.json"
        if ! "$waza_bin" run "$eval_file" --no-cache --output "$raw_output"; then
          write_result "$output" error "Waza execution failed; raw result: $raw_output"
          exit 1
        fi
        write_waza_result "$output" "$raw_output" "$eval_file" "$context_task"
        exit 0
        ;;
      *) printf 'unsupported engine: %s\n' "$engine" >&2; exit 2 ;;
    esac
    ;;
esac
