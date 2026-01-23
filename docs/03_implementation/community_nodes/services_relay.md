# Relay サービス 実装計画

**作成日**: 2026年01月22日  
**役割**: 取込・配信・永続化（必須）

## 位置づけ

本計画では relay を「外部公開の配信口」兼「取込の入口」として扱い、**取込経路を一本化**する。

- 目的は「中央集権的な公式リレー」ではなく、クライアントが選択可能な“役割の一つ”として提供すること
- ただし `kukuri-community-node` の内部構成としては、`index/moderation/trust` が共通の入力を扱えるよう **relay を必須**にする
  - topic購読により取込む対象を制限できる（後述の購読設計）

## 取込プロトコル（確定）

- relay の取込（P2P）は **iroh-gossip**（現 `kukuri-cli` 準拠）とする
  - node-level subscription（購読 topic）に応じて、iroh-gossip の topic を subscribe/unsubscribe する
  - 受信したメッセージは Nostr event（JSON）として扱い、検証・永続化・下流通知へ流す
- relay の配信（P2P）も **iroh-gossip へ再配信**する（relayの橋渡し）
  - WS（Nostr互換）等で受け付けたイベントを、対応する iroh-gossip topic へ broadcast する
  - iroh-gossip 由来のイベントを **アプリケーションが同一 topic に再注入（broadcast）しない**（ブリッジループ/増幅の回避）。ただし iroh-gossip の仕様として gossip 中継（転送）は発生する
- 関連:
  - iroh-gossip 統合設計: `docs/03_implementation/iroh_gossip_integration_design.md`

## 責務（v1）

- **取込（ingest）**
  - iroh-gossip の topic を subscribe し、イベントを受信する（node-level subscription に追従）
  - node-level の topic購読に基づき、ネットワークからレコードを取得する
  - 受信したイベントの検証（署名/基本スキーマ/レート制限/サイズ制限）
  - Access Control（scope/epoch）の検証（private scope の扱いは `docs/03_implementation/community_nodes/access_control_design.md`）
  - 重複排除（NIP-01 の `event.id` で dedupe）
  - 永続化（Postgres。ephemeral は保存しない）
- **配信（P2P: iroh-gossip）**
  - WS 等で受け付けたイベントを、対応する iroh-gossip topic へ broadcast する
  - broadcast 前に validate + dedupe（`event.id`）を行い、重複/不正イベントは再配信しない
- **配信（WS: Nostr互換）**
  - Nostr 互換の publish + subscribe（最小実装）
  - v1 方針: topic は `t` タグ（`["t","<topic_id>"]`）で表現し、topic タグが無いイベントは受理しない
  - 購読（REQ）では `#t` を必須にする
  - 初期取得（バックフィル）は DB を正として返し、保存済みイベントを送り切ったら `EOSE` を返す（以降はリアルタイム）
  - 課金/購読制御が必要な場合は認証（NIP-42 等）を実装し、購読範囲を制限できるようにする（後述の「認証」）
  - Access Control（v1 方針）
    - `scope!=public` は `["scope", "..."]` と `["epoch","..."]` を必須化し、暗号化ペイロードとして扱う
    - write は `event.pubkey` の membership を DB で検証して拒否できる（署名検証済み pubkey による判定）
    - read/backfill を制御するには購読者 pubkey の特定が必要なので、private scope の WS 購読は NIP-42（AUTH）必須を推奨
    - join/redeem/鍵配布（39020/39021）の正は User API とし、relay では再配信しない
    - 詳細: `docs/03_implementation/community_nodes/access_control_design.md`
- **下流通知**
  - `index/moderation/trust` が新着を追従できる仕組みを提供する（outbox を正とし、`LISTEN/NOTIFY` は起床通知に限定）
  - 詳細: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`

## 外部インタフェース（提案）

- `WS /relay`（Nostr relay 互換。reverse proxy で `/relay` 配下に統合）
- `GET /healthz`

## 認証（デフォルトOFF / 後から必須化）

- デフォルトでは relay は **認証OFF**（anonymous）として起動できるようにする（初期の導入コストを下げる）
- 認証OFFの間は **同意（ToS/Privacy）も不要**とする（ユーザー操作の手間を最小化）
- 管理画面（Services）から `relay` の **認証必須化**を切り替え可能にする
  - OFF: 認証なしで購読/投稿できる（ただし rate limit と topicのホワイトリストは必須）
  - ON: NIP-42（AUTH）等で pubkey を特定し、購読/投稿を **認証済みユーザーに限定**できる
    - 併せて ToS/Privacy 同意のチェックを有効化できる（未同意は拒否）
    - user-level subscription（課金/権限）を relay 側でも適用できるようになる
    - クォータ超過（利用量上限）の扱いを定義し、WS 側でも拒否理由を返せるようにする（v1詳細: `docs/03_implementation/community_nodes/billing_usage_metering.md`）
- 認証OFF→ON 切替の運用（既存接続・猶予・互換性）は `docs/03_implementation/community_nodes/auth_transition_design.md` に従う
  - 新規接続: 接続後 `ws_auth_timeout_seconds` 以内に AUTH が無ければ切断
  - 既存接続: `disconnect_unauthenticated_at` 到来で未AUTHを `NOTICE` → close する（購読継続の穴を残さない）

## データ/運用（補完）

- 永続化は relay が担う（取込経路の一本化）
  - index は「検索用の派生ストア（Meilisearch）」として扱い、生データの責任は relay/DB に寄せる
- topic購読（必須）
  - relay は `node-level subscription` の状態を監視し、subscribe/unsubscribe を切替える
  - 詳細: `docs/03_implementation/community_nodes/topic_subscription_design.md`
- イベント種別の扱い（削除/置換/エフェメラル等）
  - NIP-01 の分類（regular/replaceable/ephemeral/addressable）に従い、保存/配信/下流反映を統一する
  - v1 方針: deletion request（kind=5, NIP-09）/ `expiration`（NIP-40）を扱い、削除・期限切れは soft delete を基本に「配信/検索/計算対象」から除外する
  - 下流（index/moderation/trust）には outbox（正）+ `LISTEN/NOTIFY`（起床通知）で `upsert`/`delete` を通知し、状態の整合を保つ
  - 詳細: `docs/03_implementation/community_nodes/event_treatment_policy.md` / `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
- 取込レコード永続化ポリシー（dedupe/保持/容量/パーティション）
  - dedupe は `event.id` を冪等キーとし、パーティション構成に依存しない dedupe 専用テーブルを持つ
  - retention/capacity は `ingested_at` を基準にし、必要に応じて topic ごとの `ingest_policy` で上書きする
  - 詳細: `docs/03_implementation/community_nodes/ingested_record_persistence_policy.md`
- 購読フィルタと整合性（NIP 参照）
  - WS 側の購読フィルタ（REQ）や初期取得→リアルタイムへの切替（EOSE）などは `docs/nips/01.md` を参考にし、実装時に整合するよう仕様を確定する
    - `since/until/limit` の意味、複数フィルタの OR、複数条件の AND、`#<tag>` フィルタ等
    - 初期取得の並び順は `created_at` 降順（同値は `event.id` の辞書順）とし、`limit` は初期取得にのみ適用する（NIP-01）
    - EOSE を「保存済みイベントの終端」として扱い、再接続・再取得時の整合性を作る
    - v1 では topic フィルタを `#t` に統一し、topic_id は `docs/03_implementation/community_nodes/topic_subscription_design.md` の正規形（推奨: `kukuri:<64hex>`、例外: `kukuri:global` 等）を受理する
    - iroh-gossip 側は履歴保証が弱い前提のため、WS のバックフィルは DB を正として提供する（DB→`EVENT`→`EOSE`→リアルタイム）
    - 再接続は `since = last_seen_created_at - margin` + `event.id` 冪等を基本とし、取りこぼしを避ける（必要なら v2 で ingest 順序カーソルを追加）
- 整合性（重複/ループ）の扱い（必須）
  - iroh-gossip は gossip プロトコルのため、同一イベントが複数回届く（自己エコー/重複配送）前提で設計する
  - 整合性は「NIP-01 の `event.id` をキーにした冪等処理」で担保する（DB の `UNIQUE(event_id)` + インメモリ LRU 等）
  - ブリッジ方向（再配信のルール）
    - `WS -> iroh-gossip`: validate → 永続化（新規のみ）→ broadcast
    - `iroh-gossip -> WS/outbox`: validate → 永続化（新規のみ）→ WS購読者/下流へ配信（※ iroh-gossip へは再注入しない）
- Abuse 対策（v1で必須）
  - IP/鍵単位の rate limit、接続数上限、巨大イベント拒否、購読フィルタの上限
  - `#t` 無し REQ の拒否、フィルタ数/値数/`limit`/時間範囲（`since/until`）の上限、バックフィル要求頻度の制限
- 運用要件（監視/メトリクス/ログ、バックアップ/リストア、マイグレーション、違法/通報対応 Runbook）: `docs/03_implementation/community_nodes/ops_runbook.md`
