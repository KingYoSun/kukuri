# 2026-09-17 — #1065 cn-indexer 変更通知の key 分類を実クライアントの key 集合に合わせる

参照: [Issue #1065](https://github.com/kukuri-app/kukuri/issues/1065) / ADR 0025 §7.6 /
`docs/runbooks/community-node-production-rollout.md` §5.6 / 前段 `2026-09-15-1050-verdict-reuse.md`

## 範囲と判定

- 区分 B。Scope revision 2026-09-17、基準 commit `b8357c35`（main）。
- 独立監査は不要（区分 B で、親 Issue・再 Open・shared guard のいずれにも該当しない）。
- 本番確認（実投稿 1 件で全体見直しに倒れないこと）は #1068 の AC-9 へ移管した（2026-09-17 承認）。
- 対象外: 処理順・verdict 再利用規則・300 秒全件見直しの変更、media manifest から参照 object への解決。

## 調査で確定した事実

- app-api `persist_post_object` は 1 投稿で `objects/<id>/state`、`objects/<id>/envelope`、
  `indexes/timeline/<sort>/<id>`、`indexes/thread/<root>/<sort>/<id>` の 4 key を書く（本番 log の
  keys=4 と一致）。添付付き投稿は先に `manifests/media/<mid>/{state,envelope}` も書く（計 6 key）。
- Issue 起票時の「`envelopes/` と `timeline/` を同時に書く」は不正確だった。`timeline/` という prefix は
  無く、`envelopes/<id>` は reaction / session 等の envelope id で投稿 object id ではない。
  そのため当初方針 (b)「`envelopes/` を object id に解決」は採らず、無視に変更した。
- cn-indexer の test helper `persist_post` は `objects/` の 2 key しか書かず、既存 contract では再現しなかった。

## 修正前の再現

helper を実クライアントと同じ 4 key に揃え、TR contract を追加して修正前に実行した（2026-09-17）。

- `client_post_change_keys_ingest_only_that_object`（TR-1）: 赤。`(scanned, indexed, fresh, reused)` が
  期待 `(1, 1, 1, 0)` に対し `(3, 3, 1, 2)`（scope 全体の見直し）。
- `non_indexing_change_keys_do_not_ingest`（TR-2）: 赤。`indexes/` だけの batch で `scanned: 1`。
- `withdrawal_with_index_keys_still_deindexes`（TR-4）: 修正前も緑（全体見直しでも撤回は反映される）。
  分類変更後の回帰防止として置く。

## 実装

| 層 | 変更 |
| --- | --- |
| `crates/docs-sync` | `SharedReplicaKeyFamily`（共有 replica の key 種別表。prefix は互いに素）と `parse` |
| `crates/cn-indexer` | `KeyDisposition` / `key_disposition`（ワイルドカードなしの種別ごとの取り込み方）、`INDEXER_READ_FAMILIES`（読み取り prefix もここから作る）、`ChangedKeys::{Objects, Ignored, WholeScope { reason }}`、無視だけの batch は ingest しない、全体見直しの回数と理由を `IndexerStateSnapshot.event_whole_scope_fallbacks` / `last_whole_scope_fallback_reason`（`#[serde(default)]`）と DEBUG log へ |
| `crates/app-api`（test のみ） | 実操作で共有 replica へ書く key が種別表に全て登録済みであることの突き合わせ |
| docs | ADR 0025 §7.6 の分類表、rollout runbook §5.6 の確認手順 6 |

## AC / INVAR と証跡

| ID | 証跡 |
| --- | --- |
| AC-1 / TR-1 | `client_post_change_keys_ingest_only_that_object`、`worker_event_ingest_processes_only_changed_object_and_records_metrics`（helper が索引 key も書き、`objects/` 全走査が増えず fallback 0） |
| AC-2 / TR-2 | `non_indexing_change_keys_do_not_ingest`（索引のみ / reaction のみ / session・channel・metaverse のみ） |
| AC-3 / TR-3 | `unregistered_change_key_falls_back_to_whole_scope_and_is_observed`、`changed_keys_classify_objects_withdrawals_and_fallback`、`snapshot_reflects_flags_and_counters` |
| INVAR-1 / TR-4 | `withdrawal_after_reuse_still_deindexes`、`withdrawal_with_index_keys_still_deindexes` |
| INVAR-2 | worker の全件見直し経路は無変更（`worker_contracts.rs` の既存 contract） |
| INVAR-3 | `ignored_key_families_never_include_what_the_indexer_reads`、`shared_replica_writes_use_only_registered_key_families`、`prefixes_are_pairwise_disjoint`、`key_disposition` の網羅 match |

## 検証

- `cargo test -p kukuri-cn-indexer --test verdict_reuse_contracts`: 12 passed。
- `cargo test -p kukuri-docs-sync --lib keys`: 2 passed。
- `cargo test -p kukuri-app-api --lib shared_replica_keys`: 1 passed。
- `cargo xtask cn-check`: 成功。
- `cargo xtask cn-test`（`KUKURI_CN_RUN_INTEGRATION_TESTS=1`）: 635 passed / 0 failed / 0 ignored。
  1〜2 回目は linker の一時障害（`LNK1104` で `ucrt.lib` / `msvcrt.lib` を開けない）で中断し、
  `CARGO_BUILD_JOBS=4` で再実行して成功した。
- `cargo xtask rust-check`（fmt + workspace clippy `-D warnings`）: 成功。`cargo test -p kukuri-docs-sync`: 14 passed。
- desktop は無変更のため `tauri-check` / `desktop-lint` は CI に委ねた。
