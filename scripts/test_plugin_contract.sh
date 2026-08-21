#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
entrypoint="$factory_root/scripts/agent-workflow"
result_path="$(mktemp -t agent-workflow-plugin-result.XXXXXX.yaml)"

expected_plugins=$'claude\ncodex\ngemini\ngrok\nwaza'
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
  [[ "$runtime_exit" -eq 1 ]] || exit 1
  grep -q '^status: capability_gap$' "$runtime_result"
done
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
