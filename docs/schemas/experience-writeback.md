# Experience Write-back Record

經驗回寫只保存已確認、可重用的做法；一次性的猜測、未驗證偏好與專案機密不得直接升級成全域規則。

每筆候選經驗至少包含：

- `candidate_id`：唯一識別名稱。
- `source_task`：產生經驗的任務。
- `scope`：經驗適用的專案或情境。
- `observation`：實際觀察到的事件。
- `lesson`：可重用的做法或限制。
- `evidence`：支持這個結論的驗證紀錄。
- `confidence`：`candidate`、`confirmed` 或 `retired`。
- `owner`：經驗應該放在以下層級之一：
  `experience_library`、Personal Model、shared contract、project knowledge 或 task record。
- `review_trigger`：何時需要重新審核。

Jev 可在明確啟用後，將高信心的 `keep` 判斷確認到 `experience_library`，
供跨 session/runtime 取用與 portable pack 匯出；低信心、反例或敏感內容仍須
人工確認。這不等於可寫入 Personal Model、shared contract 或 project knowledge。

若 Jev 判斷需要人工確認，候選會記錄當時已評估的 observation 數量；沒有新
observation 時不會重複呼叫，出現新 observation 後才可再次評估。含反例的候選
不送交 Jev，並維持人工確認。

升級到 Personal Model、shared contract 或 project knowledge，仍須使用者
明確確認，或依該 owner 已定義的重複驗證條件處理。Jev 的信心只作為分類證據，
不代表事實正確。
