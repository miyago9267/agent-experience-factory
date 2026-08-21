#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
workspace_root="${MIYAGO_AGENT_WORKSPACE_ROOT:-$HOME/Project/AI/agent-workspace}"
entrypoint="$factory_root/scripts/agent-workflow"
result_path="$(mktemp -t agent-workflow-plugin-result.XXXXXX.yaml)"

expected_plugins=$'claude\ncodex\ngemini\ngrok\nopencode\nwaza'
actual_plugins="$("$entrypoint" plugin list)"
if [[ "$actual_plugins" != "$expected_plugins" ]]; then
  printf 'unexpected plugin list:\n%s\n' "$actual_plugins" >&2
  exit 1
fi
"$entrypoint" plugin doctor >/dev/null
for runtime in claude codex gemini grok; do
  runtime_plan="$($entrypoint plugin plan --id "$runtime")"
  [[ "$runtime_plan" == *'plugin_type: runtime_adapter'* ]] || exit 1
  [[ "$runtime_plan" == *'result: plugin_result'* ]] || exit 1
  runtime_result="$(mktemp -t agent-workflow-runtime-result.XXXXXX.yaml)"
  set +e
  "$entrypoint" plugin run --id "$runtime" --output "$runtime_result" >/dev/null 2>&1
  runtime_exit=$?
  set -e
  [[ "$runtime_exit" -eq 2 ]] || exit 1
  grep -q '^status: error$' "$runtime_result"
  grep -q 'prompt file is required' "$runtime_result"
done

opencode_plan="$($entrypoint plugin plan --id opencode)"
printf '%s\n' "$opencode_plan" | grep -Fq 'plugin_id: opencode'
opencode_result="$(mktemp -t agent-workflow-opencode-result.XXXXXX.yaml)"
trap 'rm -f "$result_path" "$opencode_result"' EXIT
if "$entrypoint" plugin run --id opencode --context-task agent-benchmark-waza \
  --context-cwd "$workspace_root" --prompt-file "$factory_root/fixtures/runtime-adapter-smoke/prompt.txt" \
  --output "$opencode_result" >/dev/null 2>&1; then
  printf '%s\n' 'OpenCode capability gap unexpectedly passed' >&2
  exit 1
fi
grep -q '^status: capability_gap$' "$opencode_result"
plan="$($entrypoint plugin plan --id waza)"
[[ "$plan" == *'trials_per_task: 2'* ]]
"$entrypoint" plugin run --id waza --engine mock --output "$result_path" >/dev/null
grep -q '^kind: plugin_result$' "$result_path"
grep -q '^status: pass$' "$result_path"
grep -q '^eval:$' "$result_path"
grep -q '^  trials_per_task: 2$' "$result_path"
grep -q 'no provider execution performed' "$result_path"

if "$entrypoint" plugin run --id waza --engine unsupported >/dev/null 2>&1; then
  printf '%s\n' 'unsupported engine unexpectedly passed' >&2
  exit 1
fi

printf '%s\n' 'plugin contract: OK'
