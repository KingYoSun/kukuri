# 2026-07-30 — #609 MediaFetcher の本番実装（blob の一時 fetch で media scan を実働化）

参照: [Issue #609](https://github.com/KingYoSun/kukuri/issues/609) /
ADR 0028 §7.2（本実装で追補更新）/ ADR 0025 §2.3 /
`docs/runbooks/openai-compatible-vlm.md`（env 追記）

## 実装範囲

- **ephemeral fetch 経路（`iroh-node` / `blob-service`）**: iroh-blobs 0.103 の公開 API では
  store からの blob 削除ができない（`Blobs::delete` は `pub(crate)`、削除は GC 前提）ため、
  「取得後に破棄」ではなく「**store に書かない**」方式を採用。
  `remote_fetch::fetch_bytes_ephemeral_with_cooldown`（`get::request::get_blob(conn, hash).bytes()`
  による検証付き memory 直接取得。cooldown / peers 走査 / connect 5s / transfer 15s は既存
  ループと共通）を追加し、`BlobService::fetch_blob_ephemeral`（default は `fetch_blob` 委譲、
  `IrohBlobService` は「local 既在なら読む / miss なら store 非経由 remote fetch」で override）
  として公開。remote 取得後もローカル store に blob が残らないことをテストで固定。
- **mime の経路（`cn-safety` / `cn-safety-vlm` / `cn-safety-arachnid`）**: blob store は MIME を
  持たないため、参照元 metadata（`AssetRef.mime` / manifest item）を
  `ProviderScanRequest.media_mime`（serde 後方互換）で運び、`MediaFetcher::fetch` に
  `content_type_hint` 引数を追加（trait 変更）。両 provider は hint を fetcher へ転送する。
- **本番 fetcher（`cn-indexer::media_fetcher::BlobMediaFetcher`）**: hint の 64hex 検証 /
  全体 timeout（`Timeout`）/ 取得不能（`Unavailable`）/ サイズ上限超過（`Protocol`）/
  content type 解決（hint 優先 → magic bytes 判定 jpeg・png・gif・webp・mp4・webm →
  不明は `Protocol`）。すべて fail-closed（hold）で allow に落ちる経路は無い。
  制限は `COMMUNITY_NODE_MEDIA_FETCH_MAX_BYTES`（既定 32 MiB）/
  `COMMUNITY_NODE_MEDIA_FETCH_TIMEOUT_SECS`（既定 30 秒）（`IndexerConfig.media_fetch`）。
- **manifest 展開（`cn-indexer::ingest`）**: `media_manifest_refs` の値は blob hash ではなく
  manifest_id（replica 上の `manifests/media/<id>/{state,envelope}`）であることが判明。
  `media_hints()`（hash / manifest_id 混在）を廃止し、`media_scan_targets` で
  attachments（hash + mime）+ manifest 展開（署名検証 + **post author 本人の署名を要求** →
  items + thumbnail の blob hash + mime、hash dedup）に置換。manifest が解決できない post は
  index しない（fail-closed）。scan request は
  `subject_id = media_hint = blob hash`、`media_mime = 参照元 mime`。
- **注入シーム（`cn-core::resolve_safety_providers`）**: 第 2 引数
  `Option<Arc<dyn MediaFetcher>>` を追加し、vlm / arachnid provider へ
  `with_media_fetcher` を接続（mock は不要）。
- **runtime 結線（`cn-indexer::runtime`）**: safety provider 構成時のみ iroh node
  （persistent、`data_dir` 配下、external relay URLs を `TransportRelayConfig` へ写像）+
  `IrohBlobService` + `BlobMediaFetcher` を構築して注入。peer 接続（seed 適用 / docs
  participant 起動）は ingest loop 起動の後続 Issue の範囲で、それまで remote fetch は
  peer 不在 miss → `Unavailable` → fail-closed hold（挙動後退なし）。

## 設計判断

- **一時性の担保方式**: 「fetch 後に削除」は iroh-blobs の公開 API に削除が無く GC 結線も
  重いため不採用。store 非経由の直接取得により no-permanent-blob-storage を構造的に満たす
  （破棄処理・検証が不要になる）。
- **manifest 展開は ingest 層**: fetcher は replica を知らない（blob hash → bytes のみ）。
  manifest_id → blob hash + mime の解決は docs replica を持つ ingest だけができる。
- **content type 不明は fail-closed**: 誤った MIME での scan は誤分類につながるため、
  hint も magic bytes も無い blob は `Protocol` で hold に倒す。

## テスト

- `blob-service`: `ephemeral_fetch_returns_bytes_without_persisting_them_locally`（2 node、
  remote 取得成功 + local store 非残留）。
- `cn-indexer`（unit）: fetcher の成功 / hash 不正 / miss / サイズ超過 / timeout / sniff
  fallback / 不明 mime、`MediaFetchConfig` の env 上書き・0 拒否。
- `cn-indexer`（contract）: manifest 展開後の blob hash + mime が scan request に載る
  （thumbnail は mime 無し）/ manifest 欠落 post は index されない / 既存 media 系 3 テストを
  blob hash subject へ更新。
- `cn-safety-vlm` / `cn-safety-arachnid`: `media_mime` が fetcher（arachnid は Shield への
  Content-Type header まで）へ伝播すること。
- `cn-core`: 注入シーム（fetcher を渡すと provider の media scan で fetcher が呼ばれる）。
