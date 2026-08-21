# Agent Workflow Factory 安裝手冊

## 範圍

本 installer 只安裝使用者層級入口 `~/.local/bin/agent-workflow`，並驗證：

- `agent-workspace` 是否存在且包含 Rust harness。
- `dotfile/config/ai` 是否為 canonical source。
- generated runtime entry 是否存在。
- `<non-entry-root>` 是否被排除。

預設不修改 shell startup，不使用 sudo，不安裝 Waza，不寫入 credential。

## 執行前檢查

```bash
sed -n '1,260p' INSTALL.md
bash -n install.sh
./install.sh --dry-run
```

如果 source path 不是預設位置，使用：

```bash
./install.sh --workspace-root /path/to/agent-workspace \
  --dotfile-root /path/to/dotfile
```

## 實際安裝

```bash
./install.sh
```

安裝器會建立或更新：

```text
~/.local/bin/agent-workflow
```

若目標是既有非 symlink 檔案，會先建立帶時間戳的 backup，不會直接覆寫。

## 驗證

```bash
agent-workflow doctor
agent-workflow status
agent-workflow bootstrap --runtime codex --cwd "$PWD"
```

預期 `doctor` 回報 `status: OK`，`bootstrap` 回報 `experience_bundle_path`。

## 移除

目前只需移除 factory 建立的入口：

```bash
rm ~/.local/bin/agent-workflow
```

這不會刪除 canonical source、private experience 或 runtime 設定。
