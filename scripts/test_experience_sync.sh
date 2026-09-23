#!/usr/bin/env bash
set -Eeuo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/test_support.sh"
build_test_binary

"$binary" observe --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --kind explicit_preference --runtime codex \
  --scope "$workspace_root" --source user_message \
  --summary 'The workflow should retain an explicit reusable preference.' >/dev/null
"$binary" observe --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --kind correction --runtime codex \
  --scope "$workspace_root" --source user_message \
  --summary 'An inferred correction should remain behind review.' >/dev/null

output=$(JEV_EXPERIENCE_RETENTION=1 TYPESAFE_API_KEY='' "$binary" sync \
  --workspace-root "$workspace_root" --task "$experience_task_id" \
  --experience-task "$experience_task_id" --cwd "$workspace_root" --runtime codex)
printf '%s\n' "$output" | grep -Fq 'auto_confirmed: 1'
printf '%s\n' "$output" | grep -Fq 'jev_status: enabled'
printf '%s\n' "$output" | grep -Fq 'jev_skipped: 1'
bundle=$(printf '%s\n' "$output" | sed -n 's/^experience_bundle_path: //p')
review=$(printf '%s\n' "$output" | sed -n 's/^review_queue_path: //p')
test -f "$bundle"
test -f "$review"
grep -Fq '需要使用者明確確認' "$review"
grep -Fq 'explicit reusable preference' "$bundle"
grep -Fq 'inferred correction should remain behind review' "$test_root/experience-data/candidates"/*.yaml
test "$(grep -l 'status: confirmed' "$test_root/experience-data/candidates"/*.yaml | wc -l | tr -d ' ')" = 1
test "$(grep -l 'status: candidate' "$test_root/experience-data/candidates"/*.yaml | wc -l | tr -d ' ')" = 1

printf '%s\n' 'experience_sync: OK'
