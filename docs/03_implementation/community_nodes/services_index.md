# Index サービス（Meilisearch）実装計画

**作成日**: 2026年01月22日  
**役割**: topic 別の検索・ランキング（発見体験の核）

## 責務

- relay が保存した取込レコード（Postgres）を購読し、正規化/検索用ドキュメントへ変換する
- Meilisearch への同期（検索/ランキング用のドキュメント生成）
- イベント種別（置換/削除/期限切れ等）の反映（正規化結果の整合性を保つ）
- topic 別の検索 API / trending API の提供
- 再索引（reindex）とインデックス設定（synonyms/stopwords 等）の管理

## 外部インタフェース（提案）

- 外部公開は User API に集約する
  - `GET /v1/search?topic=...&q=...`
  - `GET /v1/trending?topic=...`
  - `POST /v1/reindex`（Admin API 経由で実行）

## Meilisearch 設計（最小）

- インデックスは topic 単位（`topic_<id>`）を基本とし、必要に応じて統合インデックスも用意する
- `document` は「検索に必要な最小情報 + 検索結果表示に必要な要約」を保持する
- 生イベントは Postgres に保存し、Meilisearch は検索用の派生ストアとして扱う

## 実装手順（v1）

1. relay の取込レコードを追従（outbox を正として `seq` 追従 + `consumer_offsets` で offset 管理）
   - 詳細: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
2. 正規化（検索対象フィールドの抽出、topic_id 抽出、置換/削除/期限切れの反映）
   - `upsert`: ドキュメントを更新（replaceable/addressable の effective view 更新も含む）
   - `delete`: ドキュメントを削除（deletion request 適用、expiration 到来 等）
   - 詳細: `docs/03_implementation/community_nodes/event_treatment_policy.md`
3. Meilisearch 同期（upsert/delete）
4. trending（簡易: 期間内の反応数/投稿数 + 重み付け）
5. reindex ジョブ（キュー/進捗を Postgres に記録）
