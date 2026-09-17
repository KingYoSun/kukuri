# #1109 再 scan 後に残る scanner 由来 advisory signal の失効

## 状態と固定範囲

- 区分C、Scope revision `2026-09-17-initial`、基準commit `ae606145`。
- [Issue #1109](https://github.com/kukuri-app/kukuri/issues/1109) の AC-1〜5、INVAR-1 を固定する。
  監査・CI・merge照合と現在判定は PR と Issue の記録に集約する。
- 非対象: operator 確定済み signal（#1058）と appeal 中・認容済み・棄却済み signal の扱い、critical 系
  signal、spam / malware / phishing signal、post 行に同梱した参照 blob advisory の再計算時期
  （post の再 ingest で再計算される既存挙動）。

## 決定（ADR 0028 §8.14 に記録）

- D1: provider を呼んだ再 scan（別 subject の内容 cache 再利用を含む）が index 可能な verdict を返したら、
  それを subject の現在の判定とする。同じ issuer・target・target_id の scanner 由来 nsfw / objectionable
  signal のうち、現在の判定の advisory category に無いものへ `expires_at` を刻んで失効させる。
- D2: 構成変更による再 scan と同一構成の再 scan は区別しない。同一構成・同一内容は保存済み verdict の
  再利用で provider を呼ばないため、ここに来る同一構成の再 scan は内容変更か失敗後の再試行に限られる。
- D3: 非 allow（hold / exclude / 失敗）の再 scan では失効させない（安全側。subject は index されず、
  非 allow 経路の signal は従来どおり集約される）。
- D4: 失効させない signal: `operator_adjusted_at` あり（#1058）、`appeal_status` が `disputed` / `cleared`、
  appeal 通報（`cn_admin.reports.appeal_risk_signal_id`）から参照される行（棄却 = 判定維持を含む）、
  critical・spam 系、`classifier_score` 以外の basis。
- D5: 新しい signed moderation event は発行しない。event は不変で是正は risk signal 側（ADR 0028 §7.3）。
  配布済み advisory は既存の `expires_at` 失効契約（配布クエリ・trust 供給から除外）で伝わる。
- D6: advisory 照会は verdict を join しない。signal を真実源とする読み口（`Cleared` / 失効の除外）を保ち、
  書き込み側で signal を現在の判定へ揃える。これにより配布・trust read も同じ状態になる。
- D7: 本番の既存行は migration `202609170003_expire_superseded_advisory_signals.sql` で失効させる。
  条件は D4 を満たし、subject の verdict 行が `allow` で、その行の自 subject advisory に同じ category が
  無く、signal の `persisted_at` が verdict 行の `updated_at` 以前であること。

## 作業

| ID | AC / INVAR | 作業・対象path | 受入条件 | 検証・証跡 |
| --- | --- | --- | --- | --- |
| T1 | AC-1 | `crates/cn-core/tests/advisory_rescan_consistency.rs` | 修正前に旧 signal が照会に残って失敗 | 下記「修正前の再現」 |
| T2 | AC-2 / AC-3 / AC-4 | `crates/cn-safety-runtime/src/service.rs`、`crates/cn-core/src/safety_events.rs` / `safety_runtime.rs` | D1〜D5 の規則で照会と index read が一致し、保護対象は不変 | memory / Postgres の contract |
| T3 | AC-5 | `crates/cn-core/migrations/202609170003_*.sql` | D7 の行だけ失効し、再実行で差分なし | `expire_superseded_advisory_signals_migration` |
| T4 | 全体 | ADR 0028 §8.14、ADR 0026 §7.3、ADR 0046 §6.3、本書 | 規則と例外を正本へ記録 | 文書差分 |
| T5 | 全体 | 検証 | `cn-check` / `cn-test` / `cn-e2e` 成功 | 検証記録 |
| T6 | 全体 | 独立監査・CI・merge照合 | PR head の監査 PASS | PR 記録 |

## Surface と sensitive sink

| ID | 入口・trigger | shared helper | 読み書き・外部副作用 | guard / invariant | transition | test |
| --- | --- | --- | --- | --- | --- | --- |
| INV-1 | cn-indexer の post 本文 scan（worker full pass / key 変更 / restart） | `scan_or_reuse_guarded` → `scan_and_record_inner` → `expire_superseded_advisory_signals` | `cn_safety.risk_signals.expires_at` 更新 | 参照 guard 通過後、index 可能 verdict のみ。D4 | TR-1〜6 | PG / memory contract |
| INV-2 | cn-indexer の参照 blob scan | 同上 | 同上 | 同上（subject = blob） | TR-1, TR-3 | memory contract |
| INV-3 | `scan_and_record` / `scan_and_record_for_author` / `scan_or_reuse`（test・運用入口） | 同上 | 同上 | 同上 | TR-1 | memory contract |
| INV-4 | 保存済み verdict 再利用（`ScanDisposition::Reused`、subject 同一） | 早期 return | 書き込みなし | 失効を行わない | TR-5 | memory contract |
| INV-5 | 起動時 migration | `202609170003` | `expires_at` 更新（一度だけ・冪等） | D4 + D7 | TR-7 | migration test |
| INV-6 | `POST /v1/advisories/lookup` / index read / trust read / 配布クエリ | `list_content_advisories_for_subjects` / `filter_surfaceable_objects` / `list_trust_risk_inputs` / `list_distributable_risk_signals` | SELECT のみ（変更なし） | 失効行の除外は既存 | TR-1 | 既存 contract + PG contract |

sink 逆引き: `expire_superseded_advisory_signals` の caller は `SafetyScanService::scan_and_record_inner` の 1 箇所。
`scan_and_record_inner` の caller は上記 INV-1〜4 の公開 4 関数で、production の caller は `crates/cn-indexer/src/ingest.rs`
の post 本文と参照 blob の 2 箇所（`rg "scan_or_reuse|scan_and_record" crates --glob '!**/tests/**'`）。

## 状態遷移

| ID | 事前状態 | event | 期待状態 | 禁止する副作用 | test |
| --- | --- | --- | --- | --- | --- |
| TR-1 | 旧構成の exclude で High nsfw signal | 新構成の再 scan が allow・label なし | signal 失効、照会・index read とも advisory なし | event 追加、signal 行の削除・追加 | `rescan_allow_without_labels_expires_stale_advisory_signal` |
| TR-2 | nsfw + objectionable の labeled allow | 再 scan が objectionable のみ | nsfw だけ失効、objectionable は集約更新 | objectionable の新規行 | `rescan_keeps_only_current_advisory_categories` |
| TR-3 | 旧 signal が disputed / operator 確定 / 棄却済み / cleared、または spam・critical | 再 scan が allow・label なし | 値・状態とも不変 | 失効 | `rescan_does_not_expire_protected_signals` |
| TR-4 | 旧 signal あり | 再 scan が hold / exclude / 失敗 | 不変 | 失効 | `non_indexable_rescan_keeps_advisory_signals` |
| TR-5 | 保存済み verdict と同じ内容・構成 | 再 ingest（再利用） | 不変 | provider 呼出、失効 | `reused_verdict_does_not_touch_signals` |
| TR-6 | 失効済みの旧 signal | 次の再 scan が同じ category を再検知 | 新しい活性行を作る | 失効行の復活 | `rescan_allow_without_labels_expires_stale_advisory_signal` |
| TR-7 | 本番相当: allow・advisory なしの verdict と旧 High nsfw signal | migration 適用・再実行 | 対象だけ失効、再実行で差分なし | 保護対象・verdict より新しい signal の失効 | `expire_superseded_advisory_signals_migration` |

追加の memory contract: `fresh_allow_rescan_expires_superseded_blob_advisory_signal`（TR-1 の blob subject）、
`fresh_allow_rescan_keeps_appealed_advisory_signals`（TR-3）、`runtime_advisory_expiry_failure_is_returned`
（失効失敗時は verdict を書かずに失敗を返し、次の ingest で再 scan される）。

## 修正前の再現

- 実装前の `advisory_rescan_consistency`（実 Postgres）: 旧構成の exclude で High nsfw signal を作り、新構成の
  再 scan（allow・label なし）後に index read は advisory なし、advisory 照会は旧 signal を 1 件返して
  `rescan_allow_without_labels_expires_stale_advisory_signal` が失敗。`rescan_keeps_only_current_advisory_categories`
  も照会だけが消えた nsfw を返して失敗（index read は objectionable のみ）。保護対象と非 allow の 2 件は成功。
- migration 未適用の状態では `expire_superseded_advisory_signals_migration` の本番相当行が失効せず失敗。
- 実装後は同じ条件で全件成功。

## 検証記録

- `cargo test -p kukuri-cn-safety-runtime`: 成功（reuse 15 件を含む）。
- `KUKURI_CN_RUN_INTEGRATION_TESTS=1` で `advisory_rescan_consistency`（4）、
  `expire_superseded_advisory_signals_migration`（1）、`safety_runtime`（19）: 成功。
- `cargo xtask cn-check`: 成功。
