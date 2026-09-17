# 2026-09-17 — #1090 cn-indexer 取り込みの一時的な失敗で索引済み投稿を消さない

参照: [Issue #1090](https://github.com/kukuri-app/kukuri/issues/1090) / ADR 0025 §7.7 /
前段 `2026-09-15-1050-verdict-reuse.md`（#1080 で参照再確認を追加）

## 範囲と判定

- 区分 C（公開索引の真実源からの削除条件、fail-closed 境界）。基準 commit `503bead6`（main）。
- 独立監査: 必要（PR head に対して別コンテキストで実施）。
- 対象外: 確定した理由による de-index の変更、fail-closed verdict の保存・再利用規則（ADR 0025 §7.6）、
  本番確認（#1068 へまとめる）。

## 調査で確定した事実

- `ingest_records` の `Err` 分岐は、失敗の種類を区別せずに `deindex_object` を呼んでいた。
  #1080 で追加した `ReferenceGuard::check_current` の `LocalOnly` 照会（state / envelope / 撤回）が
  走査途中で失敗すると、索引済み投稿が真実源と投影から消える。cn-e2e の間欠失敗
  （`index_entries_total: 0`）は、baseline 投稿の再走査と `set_replica_query_failure(true)` が重なった場合に起きる。
- 同じ分岐に入る一時的な失敗は、ほかにも次があった: 送信防止・scope 対応の読み取り失敗、scan service の
  判定記録の読み書き失敗、advisory・真実源・投影の書き込み失敗。
- 本文取得（`resolve_body_text`）と media manifest 解決（`media_scan_targets`）は `Err` 分岐を通らず、
  失敗すると自身で de-index していた。本文 blob が一時的に取得できない場合（`Ok(None)` / 転送失敗）も
  削除していた。これは #745 の contract `blob_text_fetch_failure_deindexes_an_existing_entry` と
  #925 の `rejected_blob_bytes_use_only_ephemeral_fetch_and_never_reach_a_provider`（`missing` /
  `unavailable`）で固定されていた挙動で、今回承認された AC-2 に基づいて変更した。取得できた本文が検証に失敗した場合は
  引き続き削除する。取得できない状態が続く entry は保持され続けるが、その本文は索引時に検証済みである。
- scan service 内の参照再確認の失敗は `ScanError::Unavailable` に変換され、分類できる情報が失われる。
- provider 利用不可などで新たに記録された fail-closed verdict は、query 境界の join でも表面化しない
  非 allow の verdict のため、従来どおり de-index する（AC-3 の「非 allow 判定」）。

## 修正前の再現

`crates/cn-indexer/tests/transient_failure_contracts.rs` は、障害なしの再走査で対象の照会・読み取りの
回数を数え、n 回目（0 ≦ n < 回数）以降だけを失敗させる。これで参照再確認の全呼び出し位置
（直接の確認と、scan service 内の確認）を網羅する。修正前の実行結果（2026-09-17）:

| test | 修正前 | 失敗内容 |
| --- | --- | --- |
| `transient_guard_query_failure_keeps_indexed_post_and_holds_new_post`（AC-1） | 赤 | n=0 で索引済み投稿が消えた |
| `transient_index_store_read_failure_keeps_indexed_post` | 赤 | n=1 で真実源の entry が消えた |
| `transient_manifest_query_failure_keeps_indexed_media_post` | 赤 | n=0 で media 投稿が消えた |
| `blob_text_fetch_failure_keeps_an_existing_entry_until_validation_fails` | 赤 | 未取得で entry が消えた |
| `state_change_detected_by_any_recheck_deindexes_indexed_post`（AC-3） | 緑 | 回帰防止用 |
| `missing_manifest_still_deindexes_indexed_media_post`（AC-3） | 緑 | 回帰防止用 |

修正後に分類を壊す変異を 2 種類入れ、contract が検出することを確認した（確認後に元へ戻した）。

- scan service から返る失敗を常に確定した理由とする変異: guard 系が n=3 で赤になった
  （scan service 内の位置も網羅できている）。
- 確定した理由の記録を外す変異: state 変化系が n=1 で赤になった。

## 実装

| 箇所 | 変更 |
| --- | --- |
| `ingest/failure.rs`（新規） | 一時的な失敗の印 `Transient`（anyhow の context）と `transient` / `is_transient`。印の無い失敗は確定した理由 |
| `ingest/reference_guard.rs` | 真実源の読み取りと `LocalOnly` 照会の失敗に印を付ける。`verify` は確定した理由の失敗を `definitive_failure` に記録し、`classify_scan_error` は記録が無ければ scan service の失敗を一時的な失敗とする |
| `ingest/source.rs` | manifest 照会の失敗、本文 blob の取得失敗（`BlobTooLarge` を除く）と未取得に印を付ける |
| `ingest.rs` | `Err` 分岐で一時的な失敗は de-index せず `skipped_non_allow` に数える。本文・manifest 解決の一時的な失敗は自前の de-index を通らず上へ返す。送信防止の読み取り、scan、advisory・真実源・投影の書き込み失敗を分類する。warn log は原因の連鎖を出す |
| ADR 0025 §7.7 | 分類表と、保持してよい根拠 |

## Surface inventory（de-index の sink からの逆引き）

sink は `IndexEntryStore::remove_entry` / `remove_scope` と `IndexProjection::remove_object` / `remove_scope`。

| ID | 入口 | 経路 | 分類 | 確認 |
| --- | --- | --- | --- | --- |
| INV-1 | `ingest_scope` / `ingest_changed_keys` → `ingest_records` | `Err` 分岐 → `deindex_object` | 一時的な失敗は保持、それ以外は削除 | transient 系 contract |
| INV-2 | 同上 → `ingest_object_record` | 撤回・削除・送信防止（`Ok(Deindexed)`） | 確定（不変） | 既存 `withdrawal_after_reuse_still_deindexes` ほか |
| INV-3 | 同上 | 本文解決の失敗 | 検証失敗は削除、取得失敗は保持 | blob text contract |
| INV-4 | 同上 | manifest 解決の失敗 | 欠落・検証失敗は削除、照会失敗は保持 | manifest contract 2 件 |
| INV-5 | 同上 | 非 allow の verdict（本文・media） | 確定（不変） | 既存 ingestion contract |
| INV-6 | `ingest_scope` / `ingest_changed_keys` → `retain_supported_scope` | scope 非対応で `remove_scope` | 確定（不変）。読み取り失敗は走査全体の失敗で削除しない | 既存 `unsupported_scope_cannot_reuse_or_scan_content` |
| INV-7 | `IndexerParticipant::apply_transmission_prevention` | 法的判断の適用 | 確定（対象外・不変） | 既存 |
| INV-8 | `IndexerParticipant::stop_and_deindex_scope` | supported 除外 | 確定（対象外・不変） | 既存 |

未分類 0。

## 状態遷移

| ID | 事前状態 | event | 期待 | 確認 |
| --- | --- | --- | --- | --- |
| TR-1 | 索引済み | 再走査中の参照再確認の照会失敗（全位置） | 保持、索引数 0、削除数 0 | guard contract |
| TR-2 | 未索引の新規投稿 | 同上 | 索引しない | guard contract |
| TR-3 | TR-1 / TR-2 の後 | 障害解除 | 両方索引 | guard contract、cn-e2e |
| TR-4 | 索引済み | 再確認で state 変化（全位置） | 削除 | state 変化 contract |
| TR-5 | 索引済み | 真実源の読み取り失敗（全位置。0 回目は走査全体の失敗） | 保持 | store contract |
| TR-6 | 索引済み media 投稿 | manifest 照会失敗 / manifest 欠落 | 保持 / 削除 | manifest contract 2 件 |
| TR-7 | 索引済み BlobText 投稿 | 本文未取得・取得エラー / hash 不一致 | 保持 / 削除 | blob text contract |

## #1068（本番 general の索引減少）との関係

本番の状態は参照していない。コード上、次の経路は索引済み投稿を消しうる。本番確認は #1068 で行う。

1. **本文 blob の一時取得失敗（今回修正）**: 添付つき投稿は本文も常に `BlobText` で保存される
   （app-api `create_post_with_attachments_in_channel`）。indexer は本文を恒久保存しないため、全件見直しの
   たびに投稿者側のピアから本文を取り直す。投稿者がオフラインで取得できないと、修正前は de-index していた。
   **索引が少数まで減る現象の最有力候補**。indexer の warn log `failed to resolve post body` で確認できる。
2. **参照再確認の照会失敗（今回修正）**: `LocalOnly` 照会の失敗はまれで、大量減少の主因である可能性は低い。
3. **真実源・投影の書き込み失敗（今回修正）**: DB / ArcadeDB の障害時に削除していた。
4. **scan 構成の変更後の fail-closed verdict（今回の対象外・不変）**: #1080（OpenAI Moderation）や
   policy v3 の反映で scan 構成 fingerprint が変わると全件を再 scan する。provider 利用不可や media 本体の
   取得不可で fail-closed verdict が記録されると de-index し、次の pass で再試行する。provider と投稿者の
   ピアが揃えば復帰する。media 本体の取得不可が続くと、索引されないままになる。

#1068 で本番を確認するときの観点: indexer log の `failed to resolve post body` と
`temporarily failed to ingest object record` の件数、`cn_safety.scan_verdicts` の post / blob の
`reason_code`（`provider_unavailable` の件数）、`/v1/status` の `media_fetch_unavailable`。

## AC / INVAR と証跡

| ID | 証跡 |
| --- | --- |
| AC-1 | `transient_guard_query_failure_keeps_indexed_post_and_holds_new_post`（修正前は赤） |
| AC-2 | 上記、`transient_index_store_read_failure_keeps_indexed_post`、`transient_manifest_query_failure_keeps_indexed_media_post`、`blob_text_fetch_failure_keeps_an_existing_entry_until_validation_fails`、`rejected_blob_bytes_use_only_ephemeral_fetch_and_never_reach_a_provider`（`missing` / `unavailable` は保持。一時取得のみ・provider 不到達・upsert なしは全ケースで不変） |
| AC-3 | `state_change_detected_by_any_recheck_deindexes_indexed_post`、`missing_manifest_still_deindexes_indexed_media_post`、blob text の hash 不一致、既存の撤回・削除・送信防止・scope 非対応・署名不一致・非 allow の contract（`cn-test` で実行） |
| AC-4 | cn-e2e `replica_query_failure_keeps_new_posts_out_of_surfaces_until_recovery`（下記「検証」）と CI の `linux-cn-e2e` |
| INVAR-1 | migration と `MemoryIndexEntryStore` の CHECK 相当は無変更。真実源の upsert までの一時的な失敗は upsert に到達しない（TR-2）。投影の書き込みだけが失敗した場合は、全確認を通過した allow 投稿の真実源 entry が残るが、検索・列挙は投影の hit を真実源で絞り込むため、次の走査で投影に書くまで表面化しない（ADR 0025 §6.7 の既存方針。修正前はこの場合も真実源から削除していた） |

## 検証

- `cargo test -p kukuri-cn-indexer --test transient_failure_contracts --test blob_text_ingestion_contracts --lib failure`: 成功。
- `cargo xtask cn-check`（fmt + clippy `-D warnings`）: 成功。
- `cargo xtask cn-e2e`: 13 passed / 0 failed。`replica_query_failure_keeps_new_posts_out_of_surfaces_until_recovery` を含む。
- `cargo xtask cn-test` 1 回目: `rejected_blob_bytes_use_only_ephemeral_fetch_and_never_reach_a_provider` が赤。
  旧挙動（未取得で削除）を固定していたため、AC-2 に合わせて `missing` / `unavailable` の期待を保持へ変更した
  （他の条件は弱めていない）。変更後の単独実行は 6 passed。
- `cargo xtask cn-test` 2 回目: 676 passed / 0 failed / 0 ignored。変更後に `cn-check` も再実行して成功。
