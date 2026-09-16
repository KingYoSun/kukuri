# 2026-09-16 — #1058 operator が確定した risk signal を再 scan の集約更新から保護する

参照: [Issue #1058](https://github.com/KingYoSun/kukuri/issues/1058)（#1050 の New-requirement）/
ADR 0028 §7.3 / ADR 0026 §6.2 / `docs/runbooks/openai-compatible-vlm.md`「appeal / operator レビュー運用」

## 範囲と判定

- 区分 C（2026-09-16 に B から引き上げ。risk_signals への migration と、cross-node 開示本文にも載る
  `TrustBasisEntry` への項目追加を含むため）。Scope revision 2026-09-16、基準 commit `773e52c3`。
- 本記録は実装 PR の作業報告。本番反映は別 Issue でまとめて行う。
- 対象外: #1050 の集約・verdict 再利用そのもの、trust 寄与の変更、appeal 遷移規則、desktop UI の表示、
  印を外す操作。

## 修正前の再現

基準 commit に列の追加と読み取りだけを入れた状態（保護・印付けなし）で、新規 test が次の箇所で失敗した。

- `appeal_review_adjustments_survive_rescan`（Issue の再現 sequence）: 審査で confidence 20 に訂正して
  再発行した行が、再 scan で `Some(84)` に戻った（`left: Some(84) right: Some(20)`）。
- `operator_adjusted_signal_survives_rescan`: `cn-cli moderation edit` で severity Low にした行が、
  再 scan で `High` に戻った（`left: High right: Low`）。

コード上の追加の到達経路（同 test の後続場面で固定）:

- category を変える編集・再発行、または期限を付ける編集の後は、元の鍵に活性行が無いため、再 scan が
  scanner の判定を新しい行として INSERT していた（AC-2 違反）。
- `cn-cli moderation reissue` は scanner と同じ集約経路で訂正版を保存していたため、保護だけを入れると
  訂正済み行の再発行が抑止される。operator 専用の挿入へ分けた。

## 実装

| 層 | 変更 |
| --- | --- |
| migration `202609160002_risk_signal_operator_adjustment.sql` | `operator_adjusted_at TIMESTAMPTZ` / `operator_origin_category TEXT` と CHECK（印には訂正前 category が必須）、保護判定用の部分 index。`cn_admin.operator_actions` の `appeal.edit_detection` / `appeal.reissue_correction` を発生順に適用して過去の審査操作を復元（編集 → 再発行の連鎖で訂正前 category を引き継ぐ）。冪等 |
| `cn-core` `safety_events.rs` | `StoredRiskSignal.operator_adjusted_at` / `operator_origin_category`。読み取り列を `RISK_SIGNAL_COLUMNS` へ統一（`advisory_lookup.rs` を含む）。`persist_risk_signal_deduplicated` の保護: 活性行が印付きなら更新しない / 活性行が無くても同じ issuer・target・basis で category か訂正前 category が一致する印付き行があれば失効・cleared を問わず INSERT しない / 同時挿入の `ON CONFLICT DO UPDATE` も印付き行を更新しない。`insert_operator_corrected_risk_signal`（審査・cn-cli の再発行で共有） |
| `cn-core` `appeal_reviews.rs` | 審査の Edit で印を付け訂正前 category を初回だけ記録。Reissue は共有の operator 挿入へ切替（旧行の訂正前 category を引継ぎ） |
| `cn-core` `safety_appeals.rs` | `cn-cli` の edit で印付け。reissue は旧行の失効と訂正版の挿入を 1 取引にし、集約経路を通さない |
| `cn-trust` / `cn-protocol` / TS | `TrustRiskInput.operator_adjusted_at`、`TrustBasisEntry.operator_adjusted_at`（RFC3339、`#[serde(default)]`、旧 node の応答は `None`）、`types.generated.ts` 再生成（TS では省略可能） |
| `cn-cli` | `moderation show` に `operator_adj:`（確定時刻と訂正前 category）、`list-signals` に `operator_adjusted=` |
| docs | ADR 0028 §7.3、VLM runbook、本記録 |

設計上の判断:

- 保護の一致条件に訂正前 category を含めるのは、category を変えた訂正の後も元の category の再 scan を
  同じ判定として抑止するため。category を無視すると、同じ投稿に後から出た csam などの critical 判定まで
  抑止してしまうため採らない。
- 印は失効・cleared 後も効く。印を外す操作は持たない（Issue の範囲外）。
- 棄却（Reject）と認容（Accept）は値を変えないため印を付けない。認容は既存の cleared 規則で再作成されない。
- `MemorySafetyArtifactStore`（cn-safety-runtime のテスト用）には operator 経路が無いため変更していない。
- `TrustBasisEntry` は cross-node 開示（confirmed の絶対成分のみ）の本文でもあり、印の時刻が載る。
  operator 個人は特定できない。

## 固定 inventory

| ID | 入口・trigger | shared helper | 読み書き・外部副作用 | guard / invariant | transition | test |
| --- | --- | --- | --- | --- | --- | --- |
| INV-1 | scan（cn-indexer ingest → `SafetyScanService` → `SafetyArtifactStore::persist_signal`） | `persist_risk_signal_deduplicated`（`persist_risk_signal` / `_with_author` も同じ） | risk_signals UPDATE / INSERT、subject author INSERT | 印付き行は更新・再作成しない（AC-1 / AC-2）、未訂正行は #1050 のまま（INVAR-1） | TR-1〜TR-6 | `operator_adjusted_signal_survives_rescan`、`appeal_review_adjustments_survive_rescan`、`rescan_with_same_key_updates_signal_instead_of_inserting` |
| INV-2 | 運営画面の審査 Edit（`apply_appeal_review_action`） | なし | risk_signals UPDATE（印付け）、reports、operator_actions | 印と訂正前 category を同一取引で記録 | TR-2 / TR-3 | `appeal_review_adjustments_survive_rescan` |
| INV-3 | 運営画面の審査 Reissue | `insert_operator_corrected_risk_signal` | 旧行 cleared、訂正版 INSERT（印付き） | 訂正前 category を引継ぎ、appeal 遷移不変（INVAR-2） | TR-4 | 同上、`operator_review_revalidates_and_commits_state_reports_and_audit_together` |
| INV-4 | 運営画面の審査 Accept / Reject | なし | appeal_status のみ | 印を付けない | TR-7 | `appeal_review_adjustments_survive_rescan`（Reject 後の集約更新） |
| INV-5 | `cn-cli moderation edit` | `edit_risk_signal_detection_metadata` | risk_signals UPDATE（印付け） | 同 INV-2 | TR-2 / TR-3 / TR-5 | `operator_adjusted_signal_survives_rescan` |
| INV-6 | `cn-cli moderation reissue` | `reissue_corrected_risk_signal` → `insert_operator_corrected_risk_signal` | 旧行 expires_at、訂正版 INSERT（1 取引） | 集約経路を通さない（再発行の再発行が可能） | TR-4 / TR-8 | 同上 |
| INV-7 | `cn-cli moderation dispute / clear / reject`、`POST /v1/report` の appeal 受付（`reports.rs`） | `update_risk_signal_appeal_status` ほか | appeal_status のみ | 印に触れない（変更なし） | TR-7 | 既存 appeal tests |
| INV-8 | 利用者向け trust read / cross-node pull | `trust_risk_inputs_from` → `build_trust_read` | 応答本文の `operator_adjusted_at` | 寄与計算は印で変わらない | TR-9 | `trust_read_basis_marks_operator_adjusted_signals`、`operator_adjustment_is_carried_to_trust_inputs`、`trust_read_wire_contract_keeps_flattened_view_and_explainable_basis` |
| INV-9 | `cn-cli moderation show / list-signals` | `format_signal` / `format_operator_adjustment` | 標準出力 | 印と訂正前 category を表示 | TR-9 | `show_marks_operator_adjusted_signal` |
| INV-10 | migration 適用（起動時 `initialize_database`） | なし | risk_signals の列追加と backfill | 冪等、審査記録のみから復元 | TR-10 | `operator_adjustment_migration_*` |

risk_signals の書き込み元は `rg "risk_signals" crates --glob '!**/tests/**'` で逆引きした。上記以外は
`retention.rs`（保持期限の延長・期限切れ削除）と `rights_requests.rs`（参照のみ）で、値と印に触れない。

## 状態遷移

| ID | 事前状態 | event | 期待状態 | 禁止する副作用 | test |
| --- | --- | --- | --- | --- | --- |
| TR-1 | 未訂正の活性行 | 再 scan | 値を scanner 値へ更新、行数不変（#1050） | 印付け | `operator_adjusted_signal_survives_rescan`（untouched） |
| TR-2 | 同 category で編集済みの活性行 | 再 scan | 値・期限・appeal 不変、同じ id を返す | UPDATE、INSERT | 同上（edit-same）、`appeal_review_adjustments_survive_rescan` |
| TR-3 | category を変えた編集済み行 | 元 / 訂正後 category の再 scan | 不変、`newly_created = false` | INSERT | 同上（edit-category / grace） |
| TR-4 | 訂正版（同 category / category 変更） | 再 scan | 訂正版の値不変、行数不変 | UPDATE、INSERT | 同上（reissue-same / reissue-category / frank） |
| TR-5 | 期限を付けた編集済み行（失効済み） | 再 scan | 期限・値不変、行数不変 | INSERT | 同上（edit-expiry） |
| TR-6 | 印付き行がある target | 訂正されていない別 category の再 scan | 新規行を作る | 抑止 | 同上（csam） |
| TR-7 | 審査 Reject 後の行 | 再 scan | #1050 の集約更新、印なし | 印付け | `appeal_review_adjustments_survive_rescan`（heidi） |
| TR-8 | 訂正版（印付き） | cn-cli で再度再発行 | 新しい訂正版を作る | 抑止による失敗 | `operator_adjusted_signal_survives_rescan` |
| TR-9 | 印付き / 未訂正の signal | trust read / show | 印の有無が判別でき、寄与は同じ | 寄与の変化 | INV-8 / INV-9 の test |
| TR-10 | 旧 schema + 過去の審査記録 | migration / 再実行 | 審査の訂正行だけに印、再実行で不変 | 認容行・未審査行への印付け | `operator_adjustment_migration_backfills_appeal_review_corrections`、`operator_adjustment_migration_is_idempotent` |

同時実行: 活性行は `FOR UPDATE` で取得してから印を判定するため、審査の更新とは行ロックで直列化される。
活性行が無い間に operator の訂正版挿入と競合した場合は、`ON CONFLICT DO UPDATE ... WHERE
operator_adjusted_at IS NULL` が更新せず、訂正版の行を返す。

## AC / INVAR と証跡

| ID | 証跡 |
| --- | --- |
| AC-1 | TR-2〜TR-5、INV-2 / INV-3 / INV-5 / INV-6 の印付け（`operator_adjusted_at.is_some()`） |
| AC-2 | TR-2〜TR-5 の `newly_created = false` と行数 |
| AC-3 | `show_marks_operator_adjusted_signal`、`trust_read_basis_marks_operator_adjusted_signals`、`operator_adjustment_is_carried_to_trust_inputs`、wire contract |
| INVAR-1 | TR-1、`rescan_with_same_key_updates_signal_instead_of_inserting`（無変更で成功） |
| INVAR-2 | 既存 appeal tests（`safety_appeals.rs` / `appeal_reviews.rs`）無変更で成功、TR-7 |

## 検証

- `cargo test -p kukuri-cn-core -p kukuri-cn-cli -p kukuri-cn-trust -p kukuri-cn-protocol`
  （`KUKURI_CN_RUN_INTEGRATION_TESTS=1`、ローカル Postgres 17）: 成功。
- `cargo xtask cn-check`: 成功。
- `cargo xtask ipc-types`: `types.generated.ts` に `operator_adjusted_at?: string | null` を追加。
- `cargo clippy -p kukuri-cn-protocol -p kukuri-harness --all-targets -- -D warnings`: 成功。
- `cargo xtask cn-test` は CI で確認する。
