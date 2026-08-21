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
```

安裝器只建立使用者層級的 `~/.local/bin/agent-workflow` 入口，不使用 sudo，
不會覆寫 canonical source，也不會讀取 `<non-entry-root>`。

## P1 範圍

- 統一 core binary、workspace source 與 dotfile runtime source 的發現方式。
- 提供 `doctor`、`bootstrap`、`status` 與版本資訊。
- 保留 portable experience pack 的 manifest 與 import/export contract。
- 將 runtime adapter 與 benchmark adapter 留在可替換的邊界。
