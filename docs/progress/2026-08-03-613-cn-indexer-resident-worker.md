# 2026-08-03 — #613 cn-indexer の常駐ワーカー化（レプリカ同期 → 安全性スキャン → 検索投影の本番駆動）

参照: [Issue #613](https://github.com/KingYoSun/kukuri/issues/613) /
ADR 0025 §6（Model C）/ ADR 0028（media scan）/
計画: `.claude/plans/2026-08-03-issue-613-cn-indexer-resident-worker.md`

## 実装範囲

- **実行系の組み立て（`runtime.rs`、T1）**: relay validation gate の通過後、persistent iroh node を
  1 つ構築し、media scan の一時取得（`BlobMediaFetcher`）と docs レプリカ同期（`IrohDocsSync`）で
  共有する。`PgIndexEntryStore` / `ArcadeDbProjection`（起動時に `ensure_schema` で接続確認）/
  `SafetyScanService` / `IngestPipeline` / `IndexerParticipant` を本番実装で結線する。
- **常駐ワーカー（`worker.rs`、T2）**: 起動時に `restore_scopes()` で復元 → サポート対象を 1 巡
  取り込み → 以後は「レプリカの変更通知（まとめ待ちつき）+ 一定間隔の冪等な全件見直し」で
  継続更新する。索引解除は「索引の真実源に実在する scope − いま対象であるべき scope」の差分で
  判定する（`IndexEntryStore::list_scopes` を新設。一時的な open 失敗で誤解除せず、ワーカー
  停止中に外れた scope も再起動後に解除される）。scope 単位の連続失敗は指数バックオフ
  （上限つき）で再試行し、1 つの scope / entry の失敗でワーカー全体を止めない。停止シグナル
  （Ctrl+C / SIGTERM）でワーカー → docs 同期 → iroh node の順に停止する。
- **観測状態（`state.rs` / `status.rs`、T3）**: `IndexerRuntimeState` に稼働状態 / 取り込み有効 /
  開いている scope 数 / 最終成功時刻（全件見直し・取り込み）/ 最後のエラーと対象 scope /
  scan・allow・非 allow・scan 失敗・プロバイダ利用不可・索引解除の件数 / メディア取得の
  成功・利用不可・時間切れ・大きさ超過の件数を集約する。ライブラリ API（`snapshot()`）と、
  env 設定時のみの HTTP エンドポイント（`GET /healthz` / `GET /v1/status`）の 2 面で公開する
  （#612 の起動完了判定・E2E がここを読む）。

## fail-closed の分類

- **起動失敗**: relay 不成立 / Postgres 未準備 / チャンネル秘密鍵の鍵 material 不正 / 未知の
  プロバイダ名 / 署名イベント発行が有効なのに署名鍵なし / ArcadeDB に接続不能 /
  シードピア・見直し間隔・状態アドレス env の不正値。
- **取り込みを始めずに常駐**: 安全性プロバイダ未設定（観測状態の `ingest_enabled: false` で
  機械判定できる）。
- 取り込み中の障害（プロバイダ利用不可 / メディア取得失敗 / 投影書き込み失敗）は従来どおり
  entry / scope 単位で「索引しない」側に倒れ、ワーカーは動き続ける。

## private channel の有効化

グローバルな有効化スイッチは持たない。有効化は scope 単位の既存フローそのもの:
利用者の indexing request（`insert_indexing_request`）→ 運営者の承認
（`approve_indexing_request` がサポート対象へ追加）→ チャンネル秘密鍵の登録、が揃った
private channel だけを `restore_scopes()` が開く（鍵が無ければ索引しない）。運用上止めたい
場合は「承認しない / 秘密鍵を登録しない / 失効させる」で行い、失効は次の見直しで
索引解除に反映される。

## 環境変数（新設）

| 変数 | 意味 | 既定 |
|---|---|---|
| `COMMUNITY_NODE_INDEXER_SEED_PEERS` | docs 同期 / blob 取得のシードピア（カンマ区切り、`endpoint_id` または `endpoint_id@host:port`。不正値は起動失敗） | なし |
| `COMMUNITY_NODE_INDEXER_POLL_INTERVAL_SECS` | 全件見直しの間隔（秒。0 は起動失敗） | 300 |
| `COMMUNITY_NODE_INDEXER_STATUS_ADDR` | 観測状態の HTTP 待ち受けアドレス（例 `127.0.0.1:8630`。未設定なら公開しない。認証なしのため内部 network / localhost に割り当てる） | なし |

既存の `COMMUNITY_NODE_DATABASE_URL` / `COMMUNITY_NODE_INDEXER_DATA_DIR` /
`COMMUNITY_NODE_INDEXER_OWN_RELAY` / `COMMUNITY_NODE_INDEXER_EXTERNAL_RELAY_URLS` /
`COMMUNITY_NODE_CHANNEL_SECRET_KEY` / `COMMUNITY_NODE_ARCADEDB_*` /
`COMMUNITY_NODE_SAFETY_*` / `COMMUNITY_NODE_MEDIA_FETCH_*` は変更なし。

`COMMUNITY_NODE_INDEXER_OWN_RELAY=true` の場合、実際の iroh 接続先は
`COMMUNITY_NODE_CONNECTIVITY_URLS` から解決する。フラグだけが設定され URL が空なら起動を
fail-closed する。`COMMUNITY_NODE_INDEXER_EXTERNAL_RELAY_URLS` と併用した場合は両方を重複除去して使う。
また resident worker は固定の `COMMUNITY_NODE_INDEXER_SEED_PEERS` に加え、Valkey の active
bootstrap 登録を各 restore / 全件見直しで再取得する。これにより稼働中 client の endpoint と
`addr_hint` を再起動なしで docs 同期へ反映する。

## テスト

- **ループ契約（`tests/worker_contracts.rs`、`KUKURI_CN_RUN_INTEGRATION_TESTS=1`）**: 起動時
  取り込みと変更通知への反応 / scope 除外の索引解除 / private channel の秘密鍵失効 /
  失敗 scope のバックオフと隔離 / 再起動復元と停止観測。
- **統合（`tests/runtime_integration.rs`）**: 実 iroh ノード 2 台のレプリカ同期 → 取り込み、
  リモートメディアの一時取得（取得後も取得側のローカル blob 保存領域に残らないことを、
  ピアを知らないサービス経由の miss で構造的に確認）。実 ArcadeDB への投影と索引解除は
  `KUKURI_CN_RUN_ARCADEDB_TESTS=1` でゲート（relation テストと同じ流儀）。
- 既存の `ingestion_contracts.rs`（プロバイダ利用不可・スキャン失敗・壊れた内容が索引され
  ないこと、tombstone の索引解除）と `query_contracts.rs`（投影残留 hit を返さない
  fail-closed query gate = ArcadeDB 書き込み失敗時に不許可内容が表に出ないことの根拠）は
  無変更で green。

## 残課題（Issue スコープ外）

- `CommunityIndex` capability の `Available` 昇格判断（#404 / #612 側）。
- cn-user-api readiness との統合と E2E（#612）。
- 本番 image / 配備（非目的として Issue に明記済み）。
