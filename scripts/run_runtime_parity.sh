#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
runtime_list=(claude codex gemini grok)
scenario_list=(architecture operations chitchat)
trials=2
output_root="${factory_root}/results/runtime-parity"
context_task=agent-benchmark-waza
context_cwd="$factory_root"

usage() {
  printf '%s\n' 'usage: run_runtime_parity.sh [--runtime RUNTIME] [--trials N] [--output-dir PATH] [--context-task TASK_ID] [--context-cwd PATH]'
}

while (($#)); do
  case "$1" in
    --runtime) runtime_list=("${2:?missing value for --runtime}"); shift 2 ;;
    --trials) trials="${2:?missing value for --trials}"; shift 2 ;;
    --output-dir) output_root="${2:?missing value for --output-dir}"; shift 2 ;;
    --context-task) context_task="${2:?missing value for --context-task}"; shift 2 ;;
    --context-cwd) context_cwd="${2:?missing value for --context-cwd}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ "$trials" =~ ^[1-9][0-9]*$ ]] || { printf 'trials must be positive\n' >&2; exit 2; }
mkdir -p "$output_root"
summary="$output_root/summary.yaml"
{
  printf '%s\n' 'version: 1' 'kind: runtime_parity_summary' "context_task: $context_task" "trials: $trials" 'results:'
} > "$summary"

for runtime in "${runtime_list[@]}"; do
  for scenario in "${scenario_list[@]}"; do
    prompt_file="$factory_root/fixtures/runtime-parity/${scenario}.txt"
    case "$scenario" in
      architecture) expected_terms=(既有 成本 風險 方案) ;;
      operations) expected_terms=(調查 事實 停止) ;;
      chitchat) expected_terms=(實習生) ;;
    esac
    expect_args=()
    for term in "${expected_terms[@]}"; do
      expect_args+=(--expect "$term")
    done
    for ((trial = 1; trial <= trials; trial++)); do
      result="$output_root/${runtime}-${scenario}-${trial}.yaml"
      log="$result.log"
      set +e
      MIYAGO_PLUGIN_ID="$runtime" "$factory_root/plugins/runtime-adapter.sh" run \
        --prompt-file "$prompt_file" \
        --cwd "$context_cwd" \
        --context-task "$context_task" \
        --context-cwd "$context_cwd" \
        --scope <factory-root> \
        --scope <agent-workspace-root> \
        --scope <dotfile-root> \
        "${expect_args[@]}" \
        --output "$result" >"$log" 2>&1
      rc=$?
      set -e
      result_status="$(awk '$1 == "status:" { print $2; exit }' "$result" 2>/dev/null || true)"
      [[ -n "$result_status" ]] || result_status=error
      {
        printf '  - runtime: %s\n' "$runtime"
        printf '    scenario: %s\n' "$scenario"
        printf '    trial: %s\n' "$trial"
        printf '    status: %s\n' "$result_status"
        printf '    exit_status: %s\n' "$rc"
        printf '    result_path: %s\n' "$result"
        printf '    final_output_path: %s\n' "$result.final"
      } >> "$summary"
      printf '%s/%s trial %s: status=%s exit=%s\n' "$runtime" "$scenario" "$trial" "$result_status" "$rc"
    done
  done
done

printf 'summary_path: %s\n' "$summary"
