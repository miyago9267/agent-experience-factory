#!/usr/bin/env bash
set -Eeuo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/test_support.sh"
build_test_binary

"$binary" observe --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --kind explicit_preference --runtime codex \
  --scope "$workspace_root" --source user_message \
  --summary 'Portable packs contain confirmed summaries without transcripts.' >/dev/null
"$binary" organize --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" >/dev/null
"$binary" decide --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --decision confirm --reason 'Portable pack test.' >/dev/null

pack="$test_root/experience-pack.yaml"
output=$("$binary" export --workspace-root "$workspace_root" --output "$pack")
printf '%s\n' "$output" | grep -Fq 'confirmed_count: 1'
grep -Fq 'kind: agent-experience-pack' "$pack"
grep -Fq 'Portable packs contain confirmed summaries' "$pack"
if grep -Fq 'raw transcript' "$pack"; then
  exit 1
fi

mapped_root="$test_root/mapped-project"
mkdir -p "$mapped_root"
AGENT_EXPERIENCE_DATA_ROOT="$test_root/import-data" "$binary" import \
  --workspace-root "$workspace_root" --input "$pack" \
  --map "$workspace_root=$mapped_root" | grep -Fq 'imported_count: 1'
candidate=$(find "$test_root/import-data/candidates" -type f -name '*.yaml' -print)
grep -Fq -- "- $mapped_root" "$candidate"
grep -Fq 'status: confirmed' "$candidate"

legacy_pack="$test_root/legacy-pack.yaml"
cp "$pack" "$legacy_pack"
perl -0pi -e 's/kind: agent-experience-pack/kind: miyago-agent-experience-pack/' "$legacy_pack"
AGENT_EXPERIENCE_DATA_ROOT="$test_root/legacy-import-data" "$binary" import \
  --workspace-root "$workspace_root" --input "$legacy_pack" \
  --map "$workspace_root=$mapped_root" | grep -Fq 'imported_count: 1'

printf '%s\n' 'experience_portable_pack: OK'
