# Agent Workflow Factory 安裝手冊

## 範圍

本 installer 會建立使用者層級入口 `~/.local/bin/agent-workflow`，並在缺少
Context Harness binary 時，從指定的 canonical workspace 自動 build/install。
它也會驗證：

- `agent-workspace` 是否存在且包含 Rust harness；缺少 binary 時自動建置。
- `dotfile/config/ai` 是否為 canonical source。
- generated runtime entry 是否存在。
- `<non-entry-root>` 是否被排除。

預設不修改 shell startup，不使用 sudo，不自動安裝 Waza，不寫入 credential，且
Jev experience retention 關閉。若明確啟用 Jev，執行期間會將通過本機隱私檢查的
摘要送到 TypeSafe API；其他候選資料不會因此外送。
若透過 `AGENT_CONTEXT_HARNESS_BIN` 明確指定 binary，installer 不會覆蓋該路徑；缺少時會直接失敗。
Waza 是 real benchmark 的可選外部執行器；未安裝時仍可使用 mock benchmark，
但 `plugin health --id waza` 會明確顯示 real runner 的能力缺口。

## 執行前檢查

```bash
sed -n '1,260p' INSTALL.md
bash -n install.sh
./install.sh --dry-run \
  --workspace-root /path/to/agent-workspace \
  --dotfile-root /path/to/dotfile \
  --non-entry-root /path/to/non-entry \
  --jev-experience-retention off
```

若要啟用 Jev 判斷，加入 `--jev-experience-retention on`。呼叫 `agent-workflow`
的 process 還必須有 `TYPESAFE_API_KEY`；沒有 key 時仍只使用本機候選，不會阻擋。
Jev 不會收到來源路徑、證據檔、task ID 或原始對話。詳細 payload 與人工閘門見
Context Harness 的經驗系統規格。隱私過濾只阻擋常見敏感資料形式，不是完整
DLP；啟用代表允許合格摘要送到 TypeSafe。

如果 source path 不是預設位置，使用：

```bash
./install.sh --workspace-root /path/to/agent-workspace \
  --dotfile-root /path/to/dotfile \
  --non-entry-root /path/to/non-entry
```

## 實際安裝

```bash
./install.sh \
  --workspace-root /path/to/agent-workspace \
  --dotfile-root /path/to/dotfile \
  --non-entry-root /path/to/non-entry \
  --jev-experience-retention off
```

安裝器會建立或更新：

```text
~/.local/bin/agent-workflow
~/.config/agent-experience/factory.env
```

`factory.env` 只保存 workspace 與 dotfile 的 source root，讓安裝時指定的路徑在
之後不依賴當前 shell 環境仍能生效。若要指定配置目錄，可使用
`--config-dir PATH` 與 `--runtime-config-dir PATH`；環境變數
`AGENT_FACTORY_WORKSPACE_ROOT`、`AGENT_FACTORY_DOTFILE_ROOT` 仍可在執行時覆蓋已安裝設定。

若目標是既有非 symlink 檔案，會先建立帶時間戳的 backup，不會直接覆寫。

## 驗證

```bash
agent-workflow doctor
agent-workflow status
agent-workflow bootstrap --runtime codex --cwd "$PWD"
agent-workflow runtime doctor
agent-workflow resume --cwd "$PWD"
agent-workflow handoff --reason '交接給下一個 session'
agent-workflow experience sync --runtime codex --cwd "$PWD"
agent-workflow plugin health --id waza
```

預期 `doctor` 回報 `status: OK`，`resume` 輸出 Context Pack，`experience sync` 輸出
scope-filtered experience bundle。找不到唯一安全 task 時，入口會停下來要求明確指定
`--task`，不會自行猜測專案。

要重新套用 canonical dotfile 到 runtime，使用 `runtime sync`。它只呼叫
`dotfile/script/common/setup_<runtime>.sh`，不在 Factory 複製 runtime-specific 邏輯：

```bash
agent-workflow runtime sync --runtime codex
agent-workflow runtime sync --all
```

## 移除

目前只需移除 factory 建立的入口：

```bash
rm ~/.local/bin/agent-workflow
```

這不會刪除 canonical source、private experience 或 runtime 設定。
