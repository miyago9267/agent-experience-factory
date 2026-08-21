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
- [ ] 建立 runtime adapter manifest，逐一標明 Claude、Codex、Gemini、Grok 的
  真實執行能力與輸出限制。
- [ ] 將第一個真實 runtime session 的結果轉成 verification record。
- [ ] 建立 capability gap 與 deterministic grader 的正式 fixture。
- [ ] 連接 Context Harness 的 task scope 與 Waza task，禁止 benchmark 自行擴張 scope。

# 邊界

- 不自動安裝 Waza，不下載模型，不碰 production。
- 不讀寫 `<non-entry-root>`。
- 不把 benchmark pass 自動升級成 Personal Model 或 confirmed experience。
- 不在 P2 引入向量搜尋、常駐服務或跨 provider 的隱式 session 操作。
