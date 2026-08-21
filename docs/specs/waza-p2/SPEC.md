---
title: Waza-compatible plugin factory P2
status: active
updated: 2026-08-21
---

# 目的

把 Factory、Context Harness、runtime adapter、benchmark runner、grader 與
verification 的資料格式收斂到同一條 Waza-compatible contract，讓插件可以替換
實作，但不替換結果語意。

# 定義

Waza-compatible 代表沿用 Waza 的可審核基本單位：

`eval -> task -> trial -> grader -> result -> verification evidence`

這不是要求所有插件都執行 Waza。runtime adapter 仍可呼叫 Claude、Codex、Gemini、
Grok 或其他外部工具；它只需要把執行結果正規化成同一份 result。

# 共用規格

- plugin manifest：身份、版本、類型、能力、入口與 healthcheck。
- eval：一次評估的名稱、版本、執行器、試跑次數與 grader。
- task：單一可重現情境、scope、prompt 或輸入引用、停止條件。
- trial：同一 task 的一次隔離執行，包含 runtime、model、attempt 與輸出引用。
- grader：固定規則或 judge，輸出 `score`、`passed`、`feedback`。
- result：整合 trial 與 grader 的狀態、證據、能力缺口與錯誤。
- verification record：把 result 映射成工作成果驗證，不改寫原始 result。

Canonical examples：

- `../../../contracts/plugin-result.yaml`
- `../../../contracts/verification-record.yaml`
- `../../PLUGIN-CONTRACT.md`

# 狀態

所有插件與 verification 使用：

- `pass`：證據足以支持通過。
- `fail`：執行成功但不符合 grader 或目標。
- `capability_gap`：執行器或 provider 缺少必要能力，不能歸咎於規則失敗。
- `inconclusive`：有執行但證據不足以判定。
- `error`：配置、輸入或插件本身錯誤。

# P2 Tasks

- [x] 建立 Waza-compatible plugin result 與 verification contract。
- [x] 將 Waza mock adapter 輸出補齊 eval、grader、task 與 trial 欄位。
- [x] 讓 Factory 能列出、檢查與執行符合 manifest 的插件。
- [x] 建立 runtime adapter manifest，逐一標明 Claude、Codex、Gemini、Grok 的
  真實執行能力與輸出限制。
- [x] 將第一個真實 runtime session 的結果轉成 verification record。
- [x] 建立 capability gap 與 deterministic grader 的正式 fixture。
- [x] 連接 Context Harness 的 task scope 與 Waza task，禁止 benchmark 自行擴張 scope。

## Verification status

- Local verification：Factory tests、plugin contract tests、shellcheck、Rust tests
  均通過。
- Sol verification：runtime manifest 第一輪為 `INCONCLUSIVE`，原因是 verifier 無法
  證明被排除專案的 dirty baseline 沒有變化；該專案不得被讀取，因此不採用這項旁證。
  第一輪提出的 Bash 3.2 fail-closed finding 已修正，但新的 Sol adjudication 因
  verifier thread limit 尚未執行。
- Phase 4 execution policy 的 Sol 初次 gate 為 `REVISE`；findings 與 dispositions
  記錄在 `records/verification/phase4-execution-policy-security-review.md`，修正後
  已由 Sol security-reviewer 判定 `SECURITY_READY`，0 個 P0/P1/P2 findings。
- Waza `0.38.7` 已依官方 installer 安裝並通過 checksum；官方 `waza check` 顯示
  eval schema 有效。Waza mock 與 real `copilot-sdk` 各執行三個情境、每情境兩次。
- Mock 與 real benchmark 第二輪皆為 3/3 task、6/6 trial 通過，aggregate score
  `1.00`。第一輪 real run 暴露出共用 grader 把 `scope` 套用到所有情境的假陰性，
  已改成各 task 自己的 deterministic `output_contains` 條件並重跑通過。
- Factory 的 `waza --suite real` 現在必須帶 `--context-task`，先以 Context Harness
  `plan` 驗證 task、profile、scope 與 human gate，再允許 Waza 執行；normalized result
  會保留每個 task/trial/grader 與 scope binding evidence。
- Claude、Codex、Gemini、Grok 的 CLI 雖然已存在，但目前 adapter 仍是 capability-gap
  stub；manifest 已校正為 `result_contract` 與 `capability_gap_reporting`，不再宣稱
  真實 external session 或 Waza result 已接通。

# 邊界

- Waza 安裝與 benchmark 只限本機；不下載模型，不碰 production。
- 不讀寫 `<non-entry-root>`。
- 不把 benchmark pass 自動升級成 Personal Model 或 confirmed experience。
- 不在 P2 引入向量搜尋、常駐服務或跨 provider 的隱式 session 操作。
