# Community Nodes 実装タスク（ロードマップ）

最終更新日: 2026年02月10日

目的: `docs/03_implementation/community_nodes/*` の設計に基づき、M0-M5 とクライアント実装のタスクを段階的に進められるように整理する。

## タスクファイル一覧

- M0: KIP-0001 仕様固定 + kip_types 基盤（新規）: `docs/01_project/activeContext/tasks/priority/community_nodes_m0.md`
- M1: リポジトリ雛形 + Compose: `docs/01_project/activeContext/tasks/priority/community_nodes.md`
- M2: bootstrap/relay 統合（iroh-gossip + 永続化 + outbox + 39000/39001 + Access Control）: `docs/01_project/activeContext/tasks/priority/community_nodes_m2.md`
- M3: Index v1（Meilisearch）: `docs/01_project/activeContext/tasks/priority/community_nodes_m3.md`
- M4: Moderation v1（ルール）+ v2準備（LLM）: `docs/01_project/activeContext/tasks/priority/community_nodes_m4.md`
- M5: Trust v1（Apache AGE）: `docs/01_project/activeContext/tasks/priority/community_nodes_m5.md`
- Client: ノード採用/label・attestation適用/鍵管理/暗号化投稿（新規）: `docs/01_project/activeContext/tasks/priority/community_nodes_client.md`

## 欠落タスク（対応済み）

- [x] M0: `docs/kips/` に KIP-0001 v0.1 を追加し、`cn-kip-types` に共通の kind/tag/exp/署名検証を実装する
- [x] クライアント: ノード採用 UI、label/attestation 適用、key.envelope 受理と鍵保管、scope 別暗号投稿/復号を実装する
- [x] 計画更新: `docs/01_project/activeContext/community_node_plan.md` の Node HTTP API パスを現行実装に合わせて更新する（`/v1/bootstrap/*`、`/v1/reports`、`/v1/search`）。Access Control は P2P-only を正とする
- [x] テスト: `cn-relay`/`cn-bootstrap`/`cn-admin-api`/`cn-kip-types` の統合・契約テストと、User API 主要エンドポイントの契約テストを追加する

## 実ノードE2E/統合テスト拡充（計画）

- [x] E2E 用に `kukuri-community-node` を起動する Docker 経路を追加し、実ノードの base URL をテストへ配布する（`docker-compose.test.yml`/`scripts/test-docker.*`/`SCENARIO` を整理）。
- [x] 実ノードの DB/Meilisearch に投入する E2E シード（ユーザー/トピック/投稿/label/trust）を作成し、テスト前に投入/後に掃除できるようにする。
- [x] 実ノード認証フロー（challenge/verify）を通すヘルパーを追加し、`community-node` 設定/認証/同意取得の E2E を実ノードで再実行する。
- [x] （legacy）invite.capability と key.envelope を実ノードで発行できるテストヘルパー（`cn-cli`）を整備し、招待適用/鍵同期/暗号化投稿の E2E を追加した（**P2P-only 移行のため参考扱い**）。
- [x] P2P-only: `access_control_issue_invite`/`access_control_request_join` を使った **invite共有→join.request→key.envelope→暗号化投稿** の E2E を追加する。
- [x] label/attestation/trust を実ノードで発行し、PostCard のラベル/信頼バッジ表示まで検証する E2E を追加する。
- [x] search/index の実データを投入し、検索UI（サジェスト/ページング/0件）と community node search API の連携を E2E で確認する。
- [x] bootstrap/relay 実ノードのエンドポイントを使った P2P 連携確認（list_bootstrap_nodes/services）を E2E に追加する。
- [x] 実ノード E2E のログ/artefact を `test-results/community-node-e2e` と `tmp/logs/community-node-e2e` に集約し、Runbook/CI の artefact 収集に追加する。
- [x] 実ノード E2E を Nightly/CI（`test.yml`/`nightly.yml`）へ組み込み、実行条件と所要時間を明記する。
  - CI: `test.yml` の `desktop-e2e`（push/pull_request: main/develop + workflow_dispatch）。`./scripts/test-docker.ps1 e2e-community-node` を使用。
  - Nightly: `nightly.yml` の `community-node-e2e`（cron: `0 15 * * *` + workflow_dispatch）。`./scripts/test-docker.sh e2e-community-node` を使用。
  - 所要時間目安: キャッシュあり 15〜25分、キャッシュなし 30〜45分（Docker build + seed + E2E 実行）。

## 未実装/不足事項（2026年02月02日 追記）

- [x] クライアントのノード採用設定を **複数ノード/role別** に拡張し、label/trust/search/bootstrap を採用ノード単位で切替できるようにする（UI/Store/DTO/Handler を含む）。
- [x] trust.anchor(39011) の発行/保存/適用/UI を実装し、採用attester切替で表示が変わることを確認する（M5受入条件の未達）。
- [x] Access Control の **epochローテ/追放** フローを実装する（epoch++ と残留者への key.envelope 再配布、ローテ操作の CLI/サービス化）。
- [x] `access_control` は P2P-only を正とし、User API の invite/keys エンドポイントは廃止（実装・UI・ドキュメントを統一）。
- [x] 通報 UI（reason 選択 + submit）を PostCard などに追加し、`community_node_submit_report` を利用した E2E を追加する。
- [x] label/attestation のクライアント側検証（署名/exp/採用ノード）を導入し、未採用ノード由来のデータは無視する。
- [x] テスト不足の補完:
  - [x] `cn-kip-types`: 39001/39005/39010/39011/39020 の検証テスト追加。
  - [x] `cn-user-api`: `/v1/bootstrap/*` `/v1/reports` `/v1/search` の契約テスト追加。
  - [x] `cn-admin-api`: login/logout など主要エンドポイントの契約テスト追加。
  - [x] `kukuri-tauri`: `CommunityNodeHandler` の単体/契約テスト追加。

## 未実装/不足事項（2026年02月03日 追記）

- [x] Gossip/DHT 由来の 39000/39001 収集・キャッシュ・HTTP 再取得の統合フローをクライアントに実装する（plan 4.1 の未対応分）。
- [x] community_node_plan と KIP-0001 の仕様差分（39022/kind一覧、Topic ID 形式、NIP-44 実装方針、P2P-only と Access Control の運用範囲）を整理してドキュメント整合する。
- [x] `cn-admin-api`: access_control rotate/revoke と `POST /v1/reindex` の成功系契約テストを追加する。
- [x] `cn-user-api`: `/v1/labels` `/v1/trust/*` `/v1/trending` の成功系契約テスト（seed/fixture 含む）を追加する。
- [x] `cn-relay`: ingest→outbox→WS/gossip 配信の統合テストを追加する。
- [x] `kukuri-tauri`: `CommunityNodeHandler` の複数ノード集約（search/trust の合算 + cursor 合成）ユニットテストを追加する。

## 未実装/不足事項（2026年02月05日 追記）

- [x] join.request 受信側の rate limit / 手動承認フローを実装し、key.envelope の自動配布を抑止する（P2P-only）
- [x] invite.capability の max_uses を消費管理に反映し、再利用防止のテストを追加する
- [x] クライアントの KIP 検証を強化（k/ver/必須タグ/schema）し、不正イベント拒否のテストを追加する
- [x] friend_plus を v1 で扱う方針を確定（friend=相互フォロー(kind=3)、friend_plus=FoF(2-hop)の pull join.request。Key Steward自動配布なし）
- [x] friend_plus（FoF + pull join.request）の実装:
  - [x] AccessControlService に FoF(2-hop) 判定（kind=3 相互フォロー）を追加し、scope=friend_plus の join.request を判定できるようにする
  - [x] scope=friend_plus の join.request を承認/却下するコマンドと UI を整備し、key.envelope 配布まで通す
- [x] friend_plus（FoF + pull join.request）のテスト:
  - [x] FoF 判定（相互フォロー/2-hop/非該当）の unit テストを追加する
  - [x] join.request(friend_plus) → 承認 → key.envelope → 復号表示の統合/E2E テストを追加する

## 未実装/不足事項（2026年02月06日 追記）

- [x] Access Control: P2P-only 方針と relay/DB 実装の整合を取り、membership/epoch 検証やノード側保持の是非を設計・実装・ドキュメントで統一する
- [x] Access Control: KIP 検証の不足を補完する（join.request の friend_plus 対応、key.envelope/invite の schema 検証、schema 名の統一）+ テスト追加
- [x] 削除要求: Meilisearch/AGE/モデレーションの派生データを削除・再計算するフローを実装し、削除要求が仕様通りに反映されることを確認する
- [x] OpenAPI: `utoipa` で User/Admin API の spec を生成し `/v1/openapi.json` を実データで返す。Admin Console で OpenAPI 由来の型/クライアント生成を導入し契約テストへ反映する
- [x] Admin Console: Privacy/Data・Index・Access Control のページを追加し、Vitest + Testing Library の UI テスト基盤を整備する
- [x] Moderation LLM: OpenAI/Local provider を実装（または無効化の明示）し、LLM ラベリングの統合/E2E テストを追加する

## 未実装/不足事項（2026年02月07日 調査追記）

- [x] Moderation LLM: `max_requests_per_day` / `max_cost_per_day` / `max_concurrency` を実行時に強制する予算・同時実行制御を実装し、上限到達時のスキップ理由を監査可能な形で記録する
- [x] Compose: `docs/03_implementation/community_nodes/docker_compose_profiles.md` と整合するように `llm-local` プロファイル（必要に応じて local LLM サービス定義を含む）を `kukuri-community-node/docker-compose.yml` に追加する
- [x] Admin Console: LLM 連携設定（OpenAI/Local、外部送信ON/OFF、送信範囲、保存/保持、予算上限）とメンバーシップ一覧/検索（topic+scope+pubkey）を専用 UI として実装する
- [x] `cn-user-api` 契約テスト: `/v1/bootstrap/*` `/v1/search` `/v1/reports` の成功系（200/201）とレスポンス shape の互換性を検証するテストを追加する
- [x] `cn-admin-api` 契約テスト: `login -> session cookie -> /v1/admin/auth/me -> logout` の成功系フローを追加し、認証契約の後方互換を担保する
- [x] friend_plus 統合/E2E: `join.request(friend_plus) -> 承認 -> key.envelope -> 復号表示` を実ノード経路で検証するシナリオを追加する

## 未実装/不足事項（2026年02月07日 監査追記）

- [x] `cn-admin-api` 契約テストを拡充し、`services` / `policies` / `moderation` / `subscription-requests` / `node-subscriptions` / `plans` / `subscriptions` / `usage` / `audit-logs` / `trust` の主要エンドポイント成功系とレスポンス互換を検証する
- [x] `cn-user-api` 契約テストを拡充し、`/v1/auth/*` `/v1/policies/*` `/v1/consents*` `/v1/topic-subscription*` `/v1/personal-data-*` の成功系とレスポンス shape の互換を検証する
- [x] Admin Console の UI テストを拡充し、`Dashboard` `Services` `Subscriptions` `Policies` `Trust` `Audit Logs` のページで主要操作と表示崩れ防止を検証する
- [x] `cn-relay` 統合テストを拡充し、認証 OFF→ON 切替（`enforce_at` / `ws_auth_timeout_seconds`）と rate limit 境界（接続/REQ/EVENT）で期待する `NOTICE` / `CLOSED` / reject を検証する
- [x] 管理画面の技術要件（`shadcn/ui`）について、現状実装との整合を確認し、採用する場合は依存/共通UI化を実装、見送る場合は `docs/03_implementation/community_nodes/admin_console.md` と `summary.md` の要件記述を更新する

## 未実装/不足事項（2026年02月08日 監査追記）

- [x] Runbook 要件（`GET /healthz` は依存関係込みの ready 判定）に合わせ、`cn-user-api` / `cn-admin-api` / `cn-index` / `cn-moderation` / `cn-trust` / `cn-bootstrap` の health 判定を DB 単体から拡張する（少なくとも Meilisearch・外部LLM・内部依存サービスの疎通を反映）。
- [x] `cn-admin-api` の health 集約ポーリング（`services::poll_health_once`）を契約/統合テストで検証し、`cn_admin.service_health` の `healthy|degraded|unreachable` と `details_json` 更新の後方互換を担保する。
- [x] `cn-user-api` / `cn-admin-api` の `/healthz` `/metrics` 契約テストを追加し、status code とレスポンス shape（`status`、Prometheus content-type）を固定する。
- [x] `cn-user-api` bootstrap 配布の条件付き GET（`If-None-Match` / `If-Modified-Since`）と `ETag` / `Last-Modified` / `Cache-Control` / `next_refresh_at` を検証するテストを追加する。
- [x] `cn-index` の統合テストを追加し、outbox `upsert/delete`・期限切れ削除・`reindex_jobs` の状態遷移（pending/running/succeeded/failed）までを Meilisearch 反映込みで検証する。
- [x] `cn-trust` の統合テストを追加し、`report/interactions` 取込 -> score 算出 -> `attestation(kind=39010)` 発行 -> `jobs/job_schedules` 更新までの一連フローを検証する。

## 未実装/不足事項（2026年02月09日 監査追記）

- [x] `cn-bootstrap` の 39001 クリーンアップを補完する（`topic_services` が 0 件になったとき stale な `cn_bootstrap.events(kind=39001)` が残り続ける挙動を修正し、設定変更時に即時反映できるようにする）。
- [x] `cn-bootstrap` の統合/契約テストを追加する（`refresh_bootstrap_events` の DB 反映、`topic_services` 0 件時の削除、`/healthz` `/metrics` の shape/依存異常時ステータス遷移）。
- [x] `cn_core::service_config::watch_service_config` に `LISTEN cn_admin_config` を実装し、`poll` フォールバックとのハイブリッド反映へ拡張する（`cn-admin-api` の `pg_notify` を実際に反映経路として使えるようにする）+ テスト追加。
- [x] Admin API パス設計の不整合を解消する（`services_trust.md` の `POST /v1/attestations`、`services_moderation.md` の `POST /v1/labels`、`policy_consent_management.md` の `/v1/policies*` を実装系の `/v1/admin/*` と統一。互換エイリアス実装 + 契約テスト追加済み）。
- [x] Admin Console `Dashboard` を Runbook 要件に追従させる（`outbox backlog` / `reject` 急増 / DB 逼迫の主要指標表示を追加）+ UI テスト追加。
- [x] Admin Console `Privacy/Data` に DSAR 運用ビューを追加する（削除/エクスポート要求ジョブの `queued|running|completed|failed` 一覧、再実行/中止操作、監査ログ連携）+ Admin API/フロント双方のテスト追加。

## 未実装/不足事項（2026年02月10日 監査追記）

- [x] `cn-admin-api`: Axum 0.8 のルーティング規則に合わせ、`/v1/admin/*/:param` と `/v1/policies/:policy_id*` を `{param}` 形式へ統一する（Router 初期化時の panic 回避）。
- [x] `cn-admin-api`: ルータ初期化のスモークテストを追加し、パス定義不正（`:` 形式など）を CI で即検知できるようにする。
- [x] `cn-relay`: 認証必須モードの consent/subscription 強制を統合テストで検証する（AUTH 成功 + 未同意 => `consent-required`、同意済み未購読 => `restricted: subscription required`、同意済み購読済み => 受理）。
- [x] `cn-user-api`: Billing/quota の 402 契約テストを追加する（search/trending/report/topic-subscription の上限超過、`QUOTA_EXCEEDED` details/reset_at、同一 `request_id` 再送時の冪等挙動）。
- [x] `cn-index` / `cn-moderation` / `cn-trust`: `/healthz` `/metrics` 契約テストを追加し、依存障害時の `503` 遷移と Prometheus content-type の互換を固定する。

## 未実装/不足事項（2026年02月10日 再監査追記）

- [ ] `cn-relay`: WS バックフィルの初期取得順序を `created_at` 降順（同値は `event.id` 辞書順）へ修正し、`limit` 適用時の並び順と `EOSE` 遷移を統合テストで固定する（`services_relay.md` の NIP-01 整合要件に合わせる）。
- [ ] `cn-user-api`: bootstrap 認証必須時の `401 + WWW-Authenticate` 契約を実装し、`/v1/bootstrap/nodes` `/v1/bootstrap/topics/{topic_id}/services` の契約テストでヘッダ互換を検証する。
- [ ] Runbook 必須メトリクス（outbox consumer別エラー率/処理レイテンシ/batch size）を `cn-index` / `cn-moderation` / `cn-trust` に追加し、`/metrics` 契約テストでメトリクス名とラベル互換を固定する。
- [ ] `admin_console.md` の未充足要件（Moderation のルールテスト実行、Trust のパラメータ/対象検索、Access Control の invite.capability 運用）について、実装するか要件更新でスコープを縮退するかを確定し、選択した方針に対する API/UI テストを追加する。

## 参照（設計）

- `docs/03_implementation/community_nodes/summary.md`（全体方針とマイルストーン）
- `docs/03_implementation/community_nodes/architecture_overview.md`（責務分割/データフロー）
- `docs/03_implementation/community_nodes/repository_structure.md`（構成案）
