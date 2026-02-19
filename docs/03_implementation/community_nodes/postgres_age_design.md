# Postgres 集約 + Apache AGE 設計

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node` の全サービス

## 方針

- RDB は単一の Postgres サービスに集約する
- trust 計算は Postgres 拡張の Apache AGE（Property Graph / Cypher）を利用する
- マイグレーションは `kukuri-community-node/migrations/` に集約し、全サービスで共有する

## スキーマ分割（提案）

サービス間の責務を分けつつ、DB は一つに集約する。

- `cn_admin`: 管理者/監査ログ/共通設定（サービス設定: 認証モード/施行時刻/猶予等）、利用規約/プライバシーポリシー本文（policies）と公開状態（current/effective）
  - relay/bootstrap の認証OFF→ON 切替: `docs/03_implementation/community_nodes/auth_transition_design.md`
  - `Admin API`（control plane）の最小設計: `docs/03_implementation/community_nodes/admin_api.md`
- `cn_user`: ユーザー認証・課金プラン・購読（user-level）・利用量カウンタ・ポリシー同意（consents）
  - 課金/利用量計測（課金単位、クォータ、監査）: `docs/03_implementation/community_nodes/billing_usage_metering.md`
- `cn_bootstrap`: bootstrap 固有（ノード広告設定、配布対象など）
- `cn_relay`: 取込レコード（イベント）保存、node-level subscription、outbox（下流通知）
  - 取込レコードの永続化ポリシー（dedupe/保持期間/容量上限/パーティション）: `docs/03_implementation/community_nodes/ingested_record_persistence_policy.md`
  - outbox/NOTIFY の配信セマンティクス（at-least-once/offset/リプレイ/バックプレッシャ）: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
- `cn_index`: Postgres 検索同期状態（offset）、インデックス設定、reindex ジョブ
- `cn_moderation`: 通報・ラベル・ルール・LLMジョブ
  - LLM moderation の送信/保存/開示ポリシー（外部送信範囲、ログ/保持、コスト上限、Privacy への記載）: `docs/03_implementation/community_nodes/llm_moderation_policy.md`
- `cn_trust`: trust ジョブ・集計結果・attestation 発行履歴

## Apache AGE の導入

マイグレーションで AGE を有効化する（例）。

```sql
CREATE EXTENSION IF NOT EXISTS age;
LOAD 'age';
SET search_path = ag_catalog, "$user", public;
```

推奨: `cn_trust` 側の初期化で `SELECT create_graph('kukuri_cn');` を実行する。

## グラフモデル（提案）

### Vertex
- `User { pubkey }`
- `Event { event_id, kind, created_at }`（必要なら）

### Edge（最小）
- `REPORTED { reason, created_at }`（通報ベース trust 用）
- `INTERACTED { weight, created_at }`（コミュ濃度ベース trust 用）

## バックアップ/運用（補完）

- Postgres は volume 永続化 + 定期バックアップ（`pg_dump`）を前提にする
- 監査ログ/通報/ラベルは削除ポリシー（保持期間）を設定できるようにする
  - 運用手順（監視/バックアップ/リストア/マイグレーション/インシデント対応）: `docs/03_implementation/community_nodes/ops_runbook.md`
