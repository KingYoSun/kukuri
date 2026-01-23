# Community Nodes 実装タスク（M3: Index v1）

最終更新日: 2026年01月23日

目的: relay の outbox を入力として Meilisearch を同期し、User API から検索/トレンド（発見体験）を提供できる状態にする。

参照（設計）:
- `docs/03_implementation/community_nodes/services_index.md`
- `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
- `docs/03_implementation/community_nodes/event_treatment_policy.md`
- `docs/03_implementation/community_nodes/user_api.md`
- `docs/03_implementation/community_nodes/billing_usage_metering.md`

## M3-1 Compose/profile

- [ ] `index` profile を追加し、Meilisearch と `cn-index` を起動できるようにする
- [ ] Meilisearch の secrets/初期設定（master key）を secret/env で注入する

## M3-2 outbox consumer（index worker）

- [ ] `cn-index` が `events_outbox` を `seq` で追従し、`consumer_offsets` をコミットできるようにする
- [ ] `upsert`/`delete` を `event_treatment_policy.md` の意味で適用する（削除/期限切れ/置換の反映）

## M3-3 Meilisearch 同期

- [ ] topic 単位インデックス（`topic_<id>`）を作成し、`document_id=event_id` で冪等に upsert/delete する
- [ ] 検索対象フィールドの正規化（タイトル/本文要約/author/created_at/tags 等）を実装する

## M3-4 trending（v1最小）

- [ ] topic 別の trending 指標（投稿数/反応数 等）を最小で算出する
- [ ] `GET /v1/trending?topic=...` を User API に実装する（外部公開は User API に集約）

## M3-5 User API: search/trending + 課金/クォータ

- [ ] `GET /v1/search?topic=...&q=...` を User API に実装する
- [ ] `billing_usage_metering.md` の方針に沿って、search/trending の metering/quota（402）を適用する（v1最小）

## M3-6 reindex（運用）

- [ ] `POST /v1/reindex`（Admin API 経由で実行）を実装する
- [ ] reindex ジョブのキュー/進捗/失敗を Postgres に記録できるようにする

## M3 完了条件

- [ ] relay ingest されたイベントが Meilisearch に反映され、User API の search/trending で取得できる
- [ ] outbox 遅延・再処理（consumer_offsets 巻き戻し or reindex）で復旧できる

