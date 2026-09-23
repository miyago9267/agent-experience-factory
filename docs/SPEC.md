---
title: Agent Workflow Factory P1
status: active
updated: 2026-08-21
---

# 目的

將通用 Context Harness 與工作流程零件整合成可安裝、可診斷、可搬遷的工具包；
每台裝置的 routing、任務、個人模型與資料仍保留在本機來源。

# 邊界

- 不複製或接管 `<non-entry-root>`。
- 不把 factory manifest 當成 Agent 規則來源。
- 不在安裝時偷偷修改 shell startup、credential、production 或遠端服務。
- 不把 runtime-specific adapter 混進 core binary。
- 不在 P1 引入向量資料庫或常駐服務。

# 統一入口

```text
agent-workflow install
agent-workflow doctor
agent-workflow bootstrap --runtime <runtime> --cwd <path>
agent-workflow plan --cwd <path>
agent-workflow status
agent-workflow export --output <pack>
agent-workflow import --input <pack> --map <old>=<new>
agent-workflow plugin list
agent-workflow plugin doctor
agent-workflow plugin run --id waza --engine mock --output <result>
```

# Source ownership

| 層 | 唯一來源 | Factory 責任 |
|---|---|---|
| Core Harness | Factory repository | 發現、建置、呼叫、驗證 binary |
| Routing / tasks / personal model | 使用者本機 workspace | 供 Harness 載入，不進入公開 Factory |
| Runtime config | 使用者 dotfile repository | 接入、檢查、啟動 adapter |
| Private experience | `~/.local/share/agent-experience` | 匯出、匯入、權限與 scope 檢查 |
| Jev decision | TypeSafe | Per-device opt-in; classifies filtered summaries |
| Benchmark | Waza 或替代 runner | 只提供標準化結果，不寫回規則 |

插件的 manifest、生命週期與結果格式見 [`PLUGIN-CONTRACT.md`](PLUGIN-CONTRACT.md)。
Factory 只負責發現、健康檢查與轉發；插件仍保有自己的執行細節。

Jev experience retention 在 installer 中預設關閉。啟用後，Context Harness 只送
通過本機隱私檢查的摘要、類型與重複次數；缺少 API key 或遇到錯誤時不阻擋
bootstrap。Jev 的 `keep` 只寫入可攜經驗庫，不自動改寫其他規則層。
本機篩檢不是完整 DLP；啟用設定代表使用者接受合格摘要送至 TypeSafe。

# P1 完成條件

- 一個 installer 能在 dry-run 中顯示所有來源、目標與將要改變的檔案，並能從
  Factory repository 自動 build/install Context Harness。
- `doctor` 能辨識缺少的 source、binary、generated entry、runtime target 與
  non-entry boundary。
- `bootstrap` 能呼叫 core sync，回傳可供 runtime 讀取的 bundle 路徑。
- Factory 有統一的 runtime `list`、`doctor`、`sync` facade，實際 runtime 變更仍由
  dotfile canonical setup script 負責。
- confirmed experience 可匯出成不含 raw transcript 的 portable pack，並能在
  明確 path mapping 後匯入。
- installer、doctor、bootstrap、pack import/export 都有 targeted verification。
- Waza 以 `benchmark_runner` 插件接入，能在未安裝 Waza 時用 mock 方式驗證 eval
  配置與標準結果格式；缺少真正引擎時回報 `capability_gap`。

# 後續接點

- runtime adapter contract 統一由 factory 驗證，實作仍留在各 runtime source。
- Waza adapter 透過 provider-neutral benchmark record 接入。
- Phase 4 execution policy 由 Context Harness `plan` 輸出，再由 Factory/runtime plugin
  轉成 Waza-compatible eval/task/trial 設定。
- search provider 先保留 filesystem/index seam，服務化另行評估。
