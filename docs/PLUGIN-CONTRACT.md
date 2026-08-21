# Agent Workflow Factory Plugin Contract

## 目的

Factory 的插件是可替換的接點，不是另一套規則來源。每個插件都必須能被
Factory 找到、檢查、執行，並把結果轉成同一種格式。

## Manifest

每個插件在 `plugins/<plugin-id>/manifest.yaml` 提供一份 manifest：

```yaml
schema_version: "1"
kind: agent-workflow-plugin
plugin_id: waza
version: 0.1.0
plugin_type: benchmark_runner
entrypoint: adapter.sh
capabilities:
  - deterministic_grading
healthcheck:
  command: ["--health"]
```

必要欄位是 `schema_version`、`kind`、`plugin_id`、`version`、`plugin_type`、
`entrypoint`、`capabilities` 與 `healthcheck.command`。Factory 只接受解析後仍在
Factory root 內的相對路徑；允許多個 runtime 共用 parent-level adapter，但不接受
把插件指到 Factory 外或 non-entry path。

目前允許的 `plugin_type`：

- `runtime_adapter`：把一個 runtime 的工作交給外部 provider。
- `benchmark_runner`：執行固定情境並產生比較結果。Waza 屬於這一類。
- `grader`：對輸出做固定規則或 judge 評分。
- `experience_provider`：提供已確認的經驗，不直接改寫正式規則。

## 執行介面

Factory 只需要知道三個標準動作：

```text
plugin --id <id> health
plugin --id <id> plan [options]
plugin --id <id> run [options]
```

每個 adapter 可以擁有自己的參數，但必須支援 `--health`，並在不具備外部
引擎時清楚回報 `capability_gap`，不能假裝成功。

## 統一結果

可執行的插件輸出一份 YAML；完整欄位見 `contracts/plugin-result.yaml`，至少包含：

```yaml
schema_version: "1"
kind: plugin_result
plugin_id: waza
plugin_version: 0.1.0
status: pass
task_id: personal-model-behavior-parity
runtime: provider-neutral
model: unknown
scenario: operations
attempt: 1
exit_status: 0
duration_ms: 0
scope: []
output_ref: null
tool_summary: []
grader_summary: deterministic mock plan only
graders: []
tasks: []
eval:
  name: personal-model-behavior-parity
  version: 0.1.0
  executor: mock
  trials_per_task: 2
evidence: []
capability_gaps: []
errors: []
```

`status` 只能是 `pass`、`fail`、`capability_gap`、`inconclusive` 或 `error`。
結果不等於 Personal Model 規則，也不會直接寫回經驗庫；要進入經驗流程仍
需走既有的 observation、candidate 與 human gate。

## Waza 的位置

Waza 是 benchmark runner 插件，不是所有 runtime 的共通執行器。它遵循官方
`eval.yaml`、task、grader 與 result 模型；Factory 再把 Waza 結果轉成上面的
統一結果。Waza 目前實際執行器是 `mock` 或 `copilot-sdk`，因此 Claude、Codex、
Gemini、Grok 仍要透過各自 adapter 或外部 session 接入，不能宣稱原生 parity。

OpenCode 已登記為同一份 runtime adapter contract，並透過
`opencode run --format json --pure --agent quick-explorer` 執行只讀 headless
session。daily、harness、studio 設定由 dotfile 管理；若 provider credential 不可用，
結果會保留為 `capability_gap`，不會被算成 parity pass。

## 安全與範圍

- 預設不安裝 Waza、不下載模型、不碰 production。
- adapter 不修改 Personal Model、task state 或 runtime 設定。
- raw transcript 不進 portable result；只放必要 artifact reference。
- 所有跨專案來源都必須明列 scope；缺 scope 的結果只能是 `inconclusive`。
