# コミュニティノード 全体アーキテクチャ概要

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`

## 設計原則（community_node_plan.md を踏襲）

1. **ノードは“権威”ではなく“提案者”**
   - moderation は `label(kind=39006)`（署名付き提案）
   - trust は `attestation(kind=39010)`（署名付き主張）
   - クライアントは採用ノードを選択できる（複数採用も可能）
2. **役割分割 + 併用可能**
   - `bootstrap` / `relay` / `index` / `moderation` / `trust` を分離可能にし、ロックインを避ける
3. **検証可能性**
   - 署名 + 期限（`exp`）により、第三者が検証できるデータのみを配布する

## コンポーネント（提案）

```
  (Browser: User)                           (Browser: Operator)
 +-------------------+                     +----------------+
 | Client (kukuri等) |                     |  Admin Console |
 +---------+---------+                     +--------+-------+
           |                                          |
           | HTTP                                     | HTTP
           v                                          v
    +------+-------+                           +------+------+
    |   User API   |  (data plane / 外部I/F)   |  Admin API  |  (control plane)
    +------+-------+                           +------+------+
           |                                          |
           |                                          |
           +------------------+-----------------------+
                              |
                              v
                        +-----+------+
                        |  Postgres  |  (+ Apache AGE)
                        +--+------+--+
                           |      |
                           |      v
                           |  +---+--------+
                           |  | Meilisearch|
                           |  +------------+
                           v
                   +-------+------+
                   |    relay     |  (WS / 取込・配信・永続化)
                   +-------+------+
                           |
                           v
      bootstrap / index / moderation / trust（worker: DB/outbox を購読して処理）
```

- **Admin Console**: 管理 UI（要件指定スタック）。`Admin API` のみを呼び出し、サービスが分離されても UI を維持する。
- **Admin API（control plane）**:
  - 管理者認証・認可
  - サービス設定（DBに永続化）と稼働状態の可視化（health）
  - 設定変更の配布（最初は各サービスが DB をポーリング/`LISTEN` で反映）
- **User API（data plane / 外部I/F統合）**:
  - 外部公開する HTTP API の入口を集約し、認証/課金/購読/レート制限を統一する
  - 利用規約/プライバシーポリシー同意を必須化し、未同意ユーザーの利用を拒否する
  - Access Control（invite redeem、key.envelope 配布、epoch ローテの運用I/F）を提供する
  - 利用量計測（メータリング）とクォータ超過時の挙動を統一する
  - `index/moderation/trust` の結果参照や、購読申請・通報などの write を受け付ける
  - 詳細: `docs/03_implementation/community_nodes/user_api.md`
- **relay（必須: 取込・配信・永続化）**:
  - topic購読に基づきネットワークからレコードを取込んで Postgres に保存する（取込経路の一本化）
  - WS（Nostr互換等）の配信口にもなる（公開経路は reverse proxy（v1推奨: Caddy）で統合する）
  - WS 等で受け付けたイベントを iroh-gossip topic へ再配信し、P2P 側へも流す（橋渡し。iroh-gossip 由来は再注入せず、`event.id` で冪等処理する）
  - デフォルトは認証OFFで起動でき、管理画面から後から認証必須化できる（NIP-42 等）
    - 認証OFFの間は同意（ToS/Privacy）も不要として扱う（ユーザー操作の手間を最小化）
- **bootstrap/index/moderation/trust（worker）**:
  - relay が保存した取込レコードを入力として処理し、派生成果（検索/label/attestation）を生成する
  - 入力は outbox を `seq` で追従し、`LISTEN/NOTIFY` は起床通知として利用する（詳細: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`）
  - 外部公開ポートは原則持たず、User API が外部I/Fを集約する

## Docker Compose とサービス分離

- すべてのサービスは Docker Compose で起動する
- 各サービスは `profiles` により起動対象を切り替える
- Postgres は 1 サービスに集約し、必要に応じて Apache AGE を有効化したイメージを使う
- indexer は Meilisearch を別サービスとして起動し、index サービスが同期する
- relay は **必須**とし、レコード取込・永続化の入口を一本化する（購読 topic 設計とセット）

## データの最小単位（KIP-0001 寄りの整理）

- **Node capability**: `node.descriptor(kind=39000)` / `node.topic_service(kind=39001)`
  - 各サービスが「自分が提供する役割・エンドポイント・ポリシー」を署名付きで配布する
- **Moderation**: `report(kind=39005)` → `label(kind=39006)`
  - report は入力（ゲームされやすい）なので、label のみを“提案”として配る
- **Trust**: `attestation(kind=39010)` / `trust.anchor(kind=39011)`
  - trust はスコアの押し付けではなく、根拠付きの主張を配る

## セキュリティ / 運用前提（補完）

- **Node Key 管理**
  - 署名鍵はサービスごとに分離可能（最小は node 共通鍵）
  - 秘密鍵は Docker volume へ配置し、暗号化保管（パスフレーズは secret/env）
- **Admin 認証**
  - 最小は `Admin API` の password login + session cookie（`httpOnly` 推奨）
  - 外部公開する場合は TLS 前提（v1推奨: Caddy で終端）。ただし admin 系は原則インターネット公開しない
- **秘匿情報**
  - `OPENAI_API_KEY` / `MEILI_MASTER_KEY` 等は `.env` と secrets で注入し、DB に平文保存しない
- **監査ログ**
  - 管理画面の設定変更、手動ラベリング、キー更新等は監査ログに残す（Postgres）
- **運用 Runbook**
  - 監視/バックアップ/マイグレーション/違法・通報対応は `docs/03_implementation/community_nodes/ops_runbook.md` を参照

## 実装優先度（最短価値）

1. **Postgres + relay + User API + Compose**（取込/参照の入口が動く）
2. **Admin API + Admin Console**（運用設定と可視化ができる）
3. **index（Meilisearch）**（見つかる体験を作る）
4. **moderation（ルール → LLM）**（荒れにくい体験を作る）
5. **trust（AGE）**（採用ノード選択の根拠を作る）
