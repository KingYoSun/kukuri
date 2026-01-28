# Community Nodes 実装タスク（M2: bootstrap/relay 統合）

最終更新日: 2026年01月24日

目的: M1 の雛形に対し、コミュニティノードの「取込・配信・広告（capability）」の中核（relay/bootstrap）を実装し、以降の index/moderation/trust の土台を完成させる。

参照（設計）:
- `docs/03_implementation/community_nodes/services_relay.md`
- `docs/03_implementation/community_nodes/services_bootstrap.md`
- `docs/03_implementation/community_nodes/topic_subscription_design.md`
- `docs/03_implementation/community_nodes/event_treatment_policy.md`
- `docs/03_implementation/community_nodes/ingested_record_persistence_policy.md`
- `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
- `docs/03_implementation/community_nodes/access_control_design.md`
- `docs/03_implementation/community_nodes/auth_transition_design.md`
- `docs/03_implementation/community_nodes/policy_consent_management.md`
- `docs/03_implementation/community_nodes/personal_data_handling_policy.md`
- `docs/03_implementation/community_nodes/user_api.md`
- `docs/03_implementation/community_nodes/api_server_stack.md`
- `docs/03_implementation/community_nodes/admin_api.md`
- `docs/03_implementation/community_nodes/admin_console.md`
- `docs/03_implementation/community_nodes/ops_runbook.md`
- `docs/03_implementation/community_nodes/billing_usage_metering.md`
- `docs/03_implementation/community_nodes/rate_limit_design.md`

## M2-1 `kukuri-cli` 統合（bootstrap/relay の基盤）

- [x] `kukuri_cli_migration.md` に沿って `kukuri-cli` を `kukuri-community-node` 側へ統合する
  - [x] `cn-cli`（または `cn`）に `bootstrap` / `relay` の daemon 起動を追加
  - [x] 既存 CLI の有用コマンド（鍵生成/表示等）を維持しつつ、サービス起動に必要な設定I/Fを揃える

## M2-2 Node Key / 秘密鍵管理

- [x] Node Key の読み込み/生成（Docker volume へ保管）を実装する
- [x] `cn-cli` で Node Key の生成/ローテーション手順を用意する（監査ログに残す）

## M2-3 relay: 永続化（dedupe/削除/置換/retention）

- [x] `ingested_record_persistence_policy.md` に沿って `cn_relay` のスキーマと永続化フローを実装する
  - [x] `event_dedupe`（パーティションと独立した dedupe の正）
  - [x] `events`（`ingested_at` パーティション推奨）
  - [x] `deletion_tombstones`（未到着削除の整合）
  - [x] replaceable/addressable の effective view（current テーブル）
- [x] `event_treatment_policy.md` に沿って、`upsert`/`delete` の意味と適用条件を実装する
  - [x] topic タグ必須（`["t","<topic_id>"]`）/ `#t` 必須（REQ）
  - [x] deletion（NIP-09）/ expiration（NIP-40）の適用

## M2-4 relay: outbox/NOTIFY（下流向け配信の正）

- [x] `outbox_notify_semantics.md` に沿って outbox を実装する
  - [x] `cn_relay.events_outbox`（`seq`/`op`/`event_id`/`topic_id`）
  - [x] `cn_relay.consumer_offsets`
  - [x] relay の保存トランザクションと outbox insert を同一TXにする
  - [x] `LISTEN/NOTIFY` は起床通知として発行（例: `cn_relay_outbox`）
- [x] outbox 保持/削除（例: 30日）と遅延時の再構築手順を運用に落とす（実装はジョブでも手動でもよい）

## M2-5 relay: iroh-gossip（取込/再配信）

- [x] iroh-gossip topic の subscribe/unsubscribe を node-level subscription に追従させる
- [x] 受信メッセージを Nostr event（JSON）として検証し、永続化/下流通知へ流す
- [x] ブリッジループ回避（iroh-gossip 由来のイベントを同一 topic に再注入しない）を守る

## M2-6 relay: WS（Nostr互換 最小）

- [x] NIP-01 の最小 publish/subscribe を実装する（`EVENT`/`REQ`/`EOSE`）
- [x] バックフィルは DB を正として返す（保存済み → `EOSE` → リアルタイム）
- [x] 濫用対策（`#t` 必須、フィルタ数/複雑さ上限、バックフィル頻度制限、接続数上限）を実装する

## M2-7 relay/bootstrap 認証OFF→ON（段階的切替）

- [x] `auth_transition_design.md` の `auth_mode=off|required` + `enforce_at` + `grace_seconds` を実装する
  - [x] relay WS: 施行後は NIP-42（AUTH）必須 + timeout + 既存接続の猶予/切断
  - [x] bootstrap HTTP: 施行後は `GET /v1/bootstrap/*` を認証必須化できる
- [x] 同意（ToS/Privacy）の扱いを設計通りに適用する（OFF時は不要、ON時はチェック有効化）

## M2-8 bootstrap: 39000/39001 生成（DB正）+ HTTP配布（User API正）

- [x] `services_bootstrap.md` の SoT/優先順位に沿って 39000/39001 を実装する
  - [x] DB上の広告設定 → bootstrap が署名済み event JSON を生成 → DBへ保存
  - [x] `d` タグの安定キー（39000=`descriptor`、39001=`topic_service:<topic_id>:<role>:<scope>`）
  - [x] `exp`/定期再発行（keep-alive）/失効（exp超過は返さない）
- [x] User API に配布エンドポイントを実装する（生成はしない）
  - [x] `GET /v1/bootstrap/nodes`
  - [x] `GET /v1/bootstrap/topics/:topic/services`
  - [x] ETag/Last-Modified/Cache-Control/`next_refresh_at` の付与
- [x] gossip/DHT は v1 では「ヒント」として実装する（受信=採用ではなく HTTP 再取得で確定）

## M2-9 User API: 認証/同意/購読/Access Control（入口の統合）

- [x] `user_api.md` に沿って認証（署名チャレンジ + JWT（HS256））を実装する
- [x] `policy_consent_management.md` に沿って policies/consents を実装し、保護 API に同意必須化を適用する
- [x] `topic_subscription_design.md` に沿って購読申請/承認/停止（user-level / node-level）を実装する
- [x] `access_control_design.md` に沿って P2P-only を正とし、Access Control の User API 依存を排除する

## M2-10 Billing/Metering（v1最小の“状態”）

- [x] `billing_usage_metering.md` の v1 データモデル（plan/subscription/usage）を実装する
- [x] v1 は決済連携無しでも運用できるよう、Admin から状態を管理できる導線を用意する

## M2-11 Admin API/Console（運用の入口）

- [x] `admin_api.md` の最小 API を実装する（config/audit/health/auth）
- [x] `admin_console.md` のうち、M2に必要な画面を先に実装する
  - Services: relay/bootstrap の `auth_mode`/rate limit 等の設定
  - Subscriptions: 購読申請の approve/reject、node-level の制限
  - Policies: ToS/Privacy の作成/公開/current切替
  - Audit Logs / Health の確認

## M2-12 個人データ（削除/エクスポート） v1

- [x] `personal_data_handling_policy.md` に沿って、export/deletion request のスキーマとジョブ状態管理を実装する（v1最小）
- [x] User API に個人データ API を実装する（`user_api.md` のエンドポイント一覧）
  - `POST /v1/personal-data-export-requests` / `GET ...` / `GET .../download`
  - `POST /v1/personal-data-deletion-requests` / `GET ...`
- [x] 削除要求の受理後は、DB 状態（例: `subscriber_accounts.status=deleting`）で保護 API を即時拒否できるようにする（JWTでも即時反映）
- [x] export 生成物（zip）は短期保持（例: 24h）し、download token/期限切れ自動削除までを用意する（第三者データを含めない）
- [x] deletion job で、DB の削除/匿名化 + 派生データ（Meilisearch/AGE）の削除/再計算の最小パスを用意する

## M2-13 Ops/Observability（v1最小）

- [x] `ops_runbook.md` の必須メトリクスのうち、common/user-api/relay/outbox（backlog）を計測できるようにする
- [x] ログは JSON で `stdout` に統一し、本文/JWT/生 pubkey/IP/UA を出さない（必要ならハッシュ化）

## M2 完了条件（次のM3へ進める状態）

- [x] compose で `postgres`/`relay`/`user-api` が起動し、イベントを ingest→DB保存→outbox insert まで到達する
- [x] WS で `#t` 必須の購読が動作し、バックフィル→`EOSE`→リアルタイム配信が成立する
- [x] 39000/39001 の署名済み event が DB に保存され、User API から HTTP で配布できる
- [x] Admin API/Console からサービス設定（auth_mode/rate limit 等）を更新し、各サービスが反映できる

次: `docs/01_project/activeContext/tasks/priority/community_nodes_m3.md`
