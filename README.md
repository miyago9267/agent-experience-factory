# Agent Workflow Factory

把 the user 的 Agent 工作流組裝成一個可安裝、可診斷、可搬遷的入口。

這個專案是 distribution layer，不取代兩個 canonical source：

- `<agent-workspace-root>`：Context Harness、task、experience schema 與 global rule base。
- `<dotfile-root>`：runtime 設定來源、generated entry、symlink/deploy setup。

## 使用方式

```bash
./install.sh --dry-run
./install.sh
agent-workflow doctor
agent-workflow bootstrap --runtime codex --cwd "$PWD"
agent-workflow plan --cwd "$PWD"
agent-workflow resume --task TASK_ID --cwd "$PWD"
agent-workflow checkpoint --task TASK_ID --summary '目前狀態' --next '下一步'
agent-workflow handoff --task TASK_ID --reason '換 session 或需要交接'
agent-workflow experience sync --runtime codex --task TASK_ID --cwd "$PWD"
agent-workflow plugin list
agent-workflow plugin doctor
agent-workflow plugin run --id waza --engine mock --output /tmp/waza-result.yaml
```

安裝器只建立使用者層級的 `~/.local/bin/agent-workflow` 入口，不使用 sudo，
不會覆寫 canonical source，也不會讀取 `<non-entry-root>`。

Waza 是實際 benchmark runner，不只是結果格式。`mock` 不需要安裝 Waza；要執行
真實 Waza runner 時，依 [Waza 官方安裝說明](https://github.com/microsoft/waza/blob/main/docs/GUIDE.md)
安裝 `waza`，再執行 `agent-workflow plugin health --id waza` 確認版本與能力。

## P1 範圍

- 統一 core binary、workspace source 與 dotfile runtime source 的發現方式。
- 提供 `doctor`、`bootstrap`、`resume`、`checkpoint`、`handoff`、`experience`、`status` 與版本資訊。
- 保留 portable experience pack 的 manifest 與 import/export contract。
- 將 runtime adapter 與 benchmark adapter 留在可替換的邊界。
- 所有插件遵循共用 manifest 與 `plugin_result` 格式；Waza 是第一個
  `benchmark_runner`，目前可先用 mock 驗證規格。
