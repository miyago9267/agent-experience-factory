# Agent Workflow Factory

Agent Workflow Factory 是一個通用的 Harness 層：它把 Context Harness、任務狀態、經驗流水線、runtime adapter、benchmark 與安裝流程組成一個可以獨立版控、安裝、診斷與封存的專案。

它不是某一家模型的設定檔，也不是另一份全域規則。它負責「怎麼把工作系統組起來、怎麼選資料、怎麼交給 runtime 執行、怎麼留下可驗證結果」。

## 它和其他 repository 的關係

Factory 與來源資料刻意分離，避免把設定、經驗與執行器混成一包：

| 元件 | 所屬 | 責任 |
| --- | --- | --- |
| Agent Workflow Factory | 本 repository | Rust Context Harness、安裝入口、工作流程 facade、plugin contract、benchmark、diagnostic |
| Context workspace | 本機資料來源 | routing、task/state、Personal Model、project maps 與目前工作上下文 |
| dotfile | 外部 source | Codex、Claude、Grok、OpenCode 等 runtime 的設定與 bootstrap adapter |
| private experience data | 使用者資料目錄 | 已確認經驗、候選經驗、觀察與 consumption bundle |

Factory 只版控通用工具，不複製或接管任何 runtime 設定，也不把 dotfile 內容當成自己的 canonical source。routing、task、Personal Model、知識與經驗資料由使用者自己的目錄管理；安裝時透過 source-root 設定把它們接起來，因此可換機、換路徑或換 runtime，而不必重寫 Factory 本身。

使用者可以指定任何 non-entry path；它永遠不是本工廠的資料來源、fallback 或測試來源。

## 核心工作流

```text
使用者問題
    ↓
agent-workflow route / plan
    ↓
Context Harness 判斷 task、scope、資料來源與 RoutePlan
    ↓
bootstrap / resume / checkpoint / handoff
    ↓
runtime adapter 或 plugin 執行
    ↓
驗證、benchmark、experience observation
    ↓
candidate →（Jev 關閉：human gate）
         →（Jev 開啟：keep → 可攜經驗庫；discard → 本機留痕；review → human gate）
```

Factory 的入口是 `agent-workflow`。日常工作不需要直接操作 Context Harness 的底層參數，也不需要自己找 candidate ID。

```bash
agent-workflow doctor
agent-workflow route --cwd "$PWD" --query '我要盤點跨專案的 routing 與 symlink'
agent-workflow session-start --runtime codex --cwd "$PWD"
agent-workflow session-start --runtime codex --cwd "$PWD" --experience-task TASK_ID
agent-workflow bootstrap --runtime codex --cwd "$PWD"
agent-workflow plan --cwd "$PWD"
agent-workflow resume --task TASK_ID --cwd "$PWD"
agent-workflow checkpoint --task TASK_ID --summary '目前狀態' --next '下一步'
agent-workflow handoff --task TASK_ID --reason '換 session 或需要交接'
agent-workflow runtime doctor
agent-workflow runtime sync --all
```

經驗流水線也由同一個入口提供：

```bash
agent-workflow experience observe ...
agent-workflow experience organize ...
agent-workflow experience review ...
agent-workflow experience decide ...
agent-workflow experience consume ...
agent-workflow experience export --output ./experience-pack
agent-workflow experience import --input ./experience-pack --map OLD=NEW
```

低風險的 observation、整理與 bundle 產生可以自動化；要把候選經驗寫成使用者自己的 Personal Model 或 shared rule，仍保留 human gate。

可選的 Jev 經驗判斷預設關閉。安裝時明確設定 `--jev-experience-retention on`
後，Context Harness 才會把通過本機隱私檢查的摘要送到 TypeSafe，由 Jev 判斷
保留、略過或交給人工確認。保留的內容只進可攜經驗庫，不會自動升級成 Personal
Model 或 shared rule。沒有 API key、遇到低信心或服務錯誤時，流程維持本機候選
與人工閘門，不阻擋 session。隱私檢查會攔截常見路徑、credential 與程式碼形狀，
但不是完整 DLP；啟用代表允許合格摘要送到 TypeSafe。

## Benchmark 與 Waza

Benchmark 在這裡不是單純測模型分數，而是用固定情境確認整條工作鏈是否仍然可靠：

- Agent 是否讀到正確的 context。
- task scope 是否被遵守，是否碰到排除範圍。
- runtime adapter 是否能回報成功、失敗或 capability gap。
- 輸出是否符合統一的 plugin result 與 verification record。
- 經驗是否能被取用，而不是只被寫進資料夾後長灰塵。

Factory 使用 Waza-compatible 的資料結構：

| 概念 | 在工廠中的用途 |
| --- | --- |
| eval | 一組固定的評估目標、情境與通過條件 |
| task | 一個明確的工作案例與 scope |
| trial | 同一 task 的一次實際執行 |
| grader | 對輸出做固定判定的規則或檢查器 |
| result | 統一格式的成功、失敗、能力缺口與 evidence |

目前 benchmark 分兩種：

1. `mock`：不需要安裝 Waza，用來驗證 eval 設定、scope binding、plugin contract 與結果格式。
2. `waza`：使用實際 Waza runner 執行 benchmark；若本機沒有 Waza 或 provider 不可用，必須回報 `capability_gap`，不能假裝通過。

常用指令：

```bash
agent-workflow plugin list
agent-workflow plugin doctor
agent-workflow plugin run --id waza --engine mock --output /tmp/waza-result.yaml
agent-workflow plugin health --id waza
```

Waza 是可替換的 benchmark runner，不是 Factory 的核心依賴。官方 runner 的安裝方式見 [Microsoft Waza 官方文件](https://github.com/microsoft/waza/blob/main/docs/GUIDE.md)。

固定情境 fixture 位於 `fixtures/`；plugin schema 位於 `contracts/`，Waza 接法位於 `plugins/waza/`。Benchmark output 會在本機生成到 `results/` 與 `plugins/waza/results/`，這些目錄刻意不進 public repository，避免 provider 輸出、本機路徑或環境資訊被公開。結果不會自動升級成 Personal Model，也不會因 benchmark 通過就改寫正式規則。

## Plugin contract

每個 plugin 都有 manifest，並回報統一的 `plugin_result`。目前包含：

- Codex、Claude、Gemini、Grok、OpenCode runtime adapter
- Waza benchmark runner

Factory 只負責發現、health check、scope/context 驗證與結果轉發；runtime 的實際設定仍由 dotfile 管理，benchmark runner 的細節仍由 plugin 管理。規格見 [`docs/PLUGIN-CONTRACT.md`](docs/PLUGIN-CONTRACT.md)。

## 安裝與搬遷

Factory installer 只建立使用者層級入口，不使用 sudo，也不偷偷修改 shell startup 或遠端服務：

```bash
./install.sh --dry-run \
  --workspace-root /path/to/context-workspace \
  --dotfile-root /path/to/dotfile \
  --non-entry-root /path/to/non-entry

./install.sh \
  --workspace-root /path/to/context-workspace \
  --dotfile-root /path/to/dotfile \
  --non-entry-root /path/to/non-entry \
  --experience-task-id YOUR_LOCAL_EXPERIENCE_TASK
```

安裝結果包含：

- `~/.local/bin/agent-workflow`
- `~/.local/bin/context-harness`
- `~/.config/agent-experience/factory.env`
- Factory 自己建置的 `context-harness` binary link（缺少時才建立）

`factory.env` 只保存本機 source roots、experience task ID 與工具位置，不保存任務或經驗內容；執行時仍可用 `AGENT_FACTORY_WORKSPACE_ROOT`、`AGENT_FACTORY_DOTFILE_ROOT`、`AGENT_CONTEXT_HARNESS_BIN` 與 `AGENT_FACTORY_RUNTIME_CONFIG_DIR` 覆蓋。這使 Factory 不依賴任何特定使用者的固定目錄。

安裝後先執行：

```bash
agent-workflow doctor
agent-workflow session-start --runtime codex --cwd "$PWD"
agent-workflow runtime sync --all
```

`doctor` 會檢查 source registry、RoutePlan、Context Harness、generated runtime entry、OpenCode stable link 與 non-entry boundary；缺少來源時停止，不建立第二份 fallback 設定。

`session-start` 是各 runtime 的共同進入點。它先依目前工作目錄尋找唯一 task；找不到時，只有本機安裝明確設定 `--default-task-id` 才會使用該 fallback。public Factory 不內建任何個人 task，避免把使用者的工作狀態寫進通用 repo。

## 測試

```bash
bash scripts/test_all.sh
bash scripts/test_factory.sh
bash scripts/test_factory_workflow.sh
bash scripts/test_plugin_contract.sh
bash scripts/run_cross_project_smoke.sh
```

測試重點不是只看 shell 指令能否執行，而是確認：

- 乾淨 HOME 可以安裝並建立設定入口。
- Factory facade 能呼叫 Context Harness。
- experience observe → organize/review/decide/consume 的資料鏈可運作。
- plugin manifest、health 與結果 schema 一致。
- benchmark 不會越過 task scope，也不會讀取 non-entry。

## 版本與責任邊界

`v0.2.0` 將通用 Context Harness 納入 Factory，並將個人設定、任務狀態與經驗資料維持在本機來源；installer 與回歸測試不再依賴某一台裝置的路徑或 task ID。

以下是後續觀察或擴充，不是安裝必要條件：provider parity 的長期結果、VSCode 使用負擔、向量搜尋、常駐服務、MCP search backend 與自動學習模型。
