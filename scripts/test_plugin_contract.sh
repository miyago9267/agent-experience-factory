#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
entrypoint="$factory_root/scripts/agent-workflow"
result_path="$(mktemp -t agent-workflow-plugin-result.XXXXXX.yaml)"

[[ "$("$entrypoint" plugin list)" == 'waza' ]]
"$entrypoint" plugin doctor >/dev/null
plan="$($entrypoint plugin plan --id waza)"
[[ "$plan" == *'trials_per_task: 2'* ]]
"$entrypoint" plugin run --id waza --engine mock --output "$result_path" >/dev/null
grep -q '^kind: plugin_result$' "$result_path"
grep -q '^status: pass$' "$result_path"
grep -q 'no provider execution performed' "$result_path"

if "$entrypoint" plugin run --id waza --engine unsupported >/dev/null 2>&1; then
  printf '%s\n' 'unsupported engine unexpectedly passed' >&2
  exit 1
fi

printf '%s\n' 'plugin contract: OK'
