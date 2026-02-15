# Community Nodes 実装タスク（ロードマップ）

最終更新日: 2026年02月14日

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

- [x] `cn-relay`: WS バックフィルの初期取得順序を `created_at` 降順（同値は `event.id` 辞書順）へ修正し、`limit` 適用時の並び順と `EOSE` 遷移を統合テストで固定する（`services_relay.md` の NIP-01 整合要件に合わせる）。
- [x] `cn-user-api`: bootstrap 認証必須時の `401 + WWW-Authenticate` 契約を実装し、`/v1/bootstrap/nodes` `/v1/bootstrap/topics/{topic_id}/services` の契約テストでヘッダ互換を検証する。
- [x] Runbook 必須メトリクス（outbox consumer別エラー率/処理レイテンシ/batch size）を `cn-index` / `cn-moderation` / `cn-trust` に追加し、`/metrics` 契約テストでメトリクス名とラベル互換を固定する。
- [x] `admin_console.md` の未充足要件（Moderation のルールテスト実行、Trust のパラメータ/対象検索、Access Control の invite.capability 運用）について、実装するか要件更新でスコープを縮退するかを確定し、選択した方針に対する API/UI テストを追加する。

## 未実装/不足事項（2026年02月11日 監査追記）

- [x] `cn-relay`: `/healthz` `/metrics` の契約テストを追加し、Runbook 必須メトリクス（`ws_connections` / `ws_req_total` / `ws_event_total` / `ingest_received_total` / `ingest_rejected_total` / `gossip_received_total` / `gossip_sent_total` / `dedupe_hits_total` / `dedupe_misses_total`）の公開互換を固定する。
- [x] `cn-user-api`: 認証/同意/課金メータの回帰テストを追加し、API 実行で `auth_success_total` / `auth_failure_total` / `consent_required_total` / `quota_exceeded_total` の増分を検証する。
- [x] `ops_runbook.md` のバックアップ/リストア要件（`pg_dump` 世代管理・`pg_restore` 復旧手順）を運用スクリプト化し、`scripts/test-docker.ps1` から実行できる復旧ドリルを整備する。
- [x] `cn-cli`: Node Key 生成/ローテーションと Access Control rotate/revoke の統合テストを追加し、監査ログ記録・DB 反映・CLI 出力の後方互換を担保する。

## 未実装/不足事項（2026年02月11日 再監査追記）

- [x] `admin_console.md` の Access Control 要件（epoch ローテ時の再配布「失敗/未配布」検知）に合わせ、`cn-core`/`cn-admin-api`/Admin Console で配布結果（success/failed/pending + reason）を記録・参照できるようにする。
- [x] Access Control 再配布結果のテストを補完する（`cn-core` 統合テスト: 配布失敗記録、`cn-admin-api` 契約テスト: rotate/revoke のレスポンス shape、Admin Console UI テスト: 失敗/未配布の可視化）。
- [x] `ops_runbook.md` の「月1リストア演習」要件に合わせ、`recovery-drill` を定期 CI ジョブへ組み込み（`nightly.yml` など）、`test-results/community-node-recovery/latest-summary.json` を artefact 収集・検証できるようにする。
  - CI: `.github/workflows/nightly.yml` に `community-node-recovery-drill`（UTC 毎月1日実行ガード + `workflow_dispatch` 手動実行）を追加。
  - 検証: `latest-summary.json` の `status=passed`、`baseline_event_count > 0`、`after_corruption_event_count == 0`、`after_restore_event_count == baseline_event_count` を `jq` で検証。
  - artefact: `nightly.community-node-recovery-logs`（`tmp/logs/community-node-recovery`）と `nightly.community-node-recovery-reports`（`test-results/community-node-recovery`）を収集。
- [x] Runbook の共通必須メトリクス（`http_requests_total` / `http_request_duration_seconds_bucket`）を各サービスの `/metrics` 契約テストで固定し、`service,route,method,status` ラベル互換を保証する。
- [x] `access_control_design.md` の「v1 はノード側に専用DBを持たない」記述と現行実装（membership/invite 管理 API + Admin Console）の整合を取り、P2P-only と運用補助データの境界（正とする SoT）を明文化する。

## 未実装/不足事項（2026年02月12日 監査追記）

- [x] `cn-admin-api`: 監査ログ要件（append-only / 必須）に合わせ、管理操作で `cn_core::admin::log_audit(...).await.ok()` を廃止し、監査ログ書き込み失敗時は API を失敗として返す（少なくとも `services` / `policies` / `subscriptions` / `moderation` / `trust` / `access_control` / `dsar` / `reindex` / `auth` を対象に統一）。
- [x] `cn-admin-api`: 管理更新系で `tx.commit().await.ok()` を廃止し、commit 失敗を呼び出し元へ伝播する。あわせて契約/統合テストを追加し、監査ログ書き込み失敗・commit 失敗時に `5xx` とロールバック（副作用なし）を保証する。
- [x] `cn-admin-api` + Admin Console: `service_configs` に secrets を保存しない要件を実装で強制する（`OPENAI_API_KEY` など秘匿キーの reject または redaction）。`PUT /v1/admin/services/{service}/config` の契約テストと UI テストを追加して後方互換を固定する。
- [x] Admin Console: `auth_transition_design.md` の運用要件に合わせ、relay/bootstrap の `auth_mode` / `enforce_at` / `grace_seconds` / `ws_auth_timeout_seconds` を専用フォームで編集できる UI を追加する（現行の生 JSON 編集依存を解消）。未AUTH接続残数・拒否数など施行状態の表示を追加し、Vitest + Testing Library の回帰テストを追加する。

## 未実装/不足事項（2026年02月12日 再監査追記）

- [x] `cn-admin-api` + Admin Console: `services_moderation.md` の「human review / 再判定 / 無効化」要件を満たすため、ラベルのレビュー状態管理（有効/無効・理由・実施者・実施時刻）と再判定トリガ（対象イベント再評価）を実装し、監査ログ（append-only）まで一連で整備する。
- [x] `cn-user-api`: `billing_usage_metering.md` の `trust.requests` クォータ要件に対し、`/v1/trust/report-based` と `/v1/trust/communication-density` の `402 QUOTA_EXCEEDED` + `X-Request-Id` 冪等挙動の契約テストを追加する（既存の search/trending/report と同等粒度）。
- [x] Admin Console: 認証導線の回帰を防ぐため、`LoginPage` / `App` のセッションブートストラップ（`/v1/admin/auth/me`）・ログイン成功/失敗・ログアウト後遷移の UI テストを追加する。

## 未実装/不足事項（2026年02月12日 追加監査追記）

- [x] `cn-bootstrap`: `services_bootstrap.md` の 39000/39001 配布経路要件（gossip/DHT を発見ヒントとして併用）に合わせ、HTTP配布（DB正）に加えて「更新ヒントの publish 経路」を実装する。少なくとも publish 成功/失敗時のメトリクスと統合テスト（通知受信→HTTP再取得）を追加する。
- [x] `cn-relay`: `ops_runbook.md` の `/healthz` ready 要件に合わせ、DB到達性だけでなく relay 依存（gossip 参加状態・topic購読同期状態）の劣化を `degraded/unavailable` として返せるようにする。あわせて `/healthz` 契約テストを拡張する。
- [x] `cn-relay`: `services_relay.md` / `topic_subscription_design.md` の REQ 制約（`#t` 必須・filter上限・limit上限）に対して、`filters.rs` の単体テストを追加し、拒否理由（`missing #t filter` / `too many filters` / `too many filter values`）の互換を固定する。
- [x] `cn-user-api`: `rate_limit_design.md` の 429 契約（`RATE_LIMITED` + `Retry-After`）に対する回帰テストが不足しているため、`/v1/auth/challenge` `/v1/auth/verify` `/v1/bootstrap/*` と protected API の境界テストを追加する。
- [x] OpenAPI運用: `api_server_stack.md` の「OpenAPI 生成物を CI で差分検知」要件に合わせ、`apps/admin-console/openapi/*.json` と `src/generated/admin-api.ts` の更新漏れを検知する CI ジョブ（生成→`git diff --exit-code`）を追加する。

## 未実装/不足事項（2026年02月13日 監査追記）

- [x] `cn-user-api`: `billing_usage_metering.md` の「メータリング/監査の整合」を満たすため、`billing::consume_quota` の `tx.commit().await.ok()` を廃止し、commit 失敗を `5xx(DB_ERROR)` として返却する（成功/超過レスポンスの誤返却を防止）。
- [x] `cn-user-api`: `topic_subscription_design.md` の user-level subscription 停止フローに合わせ、`delete_topic_subscription` の `tx.commit().await.ok()` を廃止し、commit 失敗時は `5xx` を返して `status=ended` を返さないようにする。
- [x] `cn-user-api`: 上記2件の異常系回帰テストを追加する（commit 失敗時に `200`/`402` を返さないこと、`usage_events`/`usage_counters_daily`/`topic_subscriptions`/`node_subscriptions` の副作用が残らないこと）。
- [x] `cn-admin-api`: `auth::logout` のセッション削除失敗を握り潰さないようにし（`DELETE cn_admin.admin_sessions ... .await.ok()` の廃止）、失敗時のレスポンス契約（`5xx`）を明文化して契約テストを追加する。

## 未実装/不足事項（2026年02月13日 追加監査追記）

- [x] `cn-relay`: `auth_transition_design.md` の既存接続要件（`disconnect_unauthenticated_at = enforce_at + grace_seconds`）と実装を一致させる。`ws_auth_timeout_seconds` との責務分離を明確化し、既存接続が猶予期間を満たしてから切断されることを統合テストで固定する。
- [x] `cn-relay`: `event_treatment_policy.md` の P2P-only 要件に合わせ、`39020/39021/39022`（Access Control）を relay の保存/outbox/WS/gossip 配布対象から除外する（拒否理由の互換を含む）。統合テストを追加する。
- [x] `cn-relay`: `event_treatment_policy.md` の回帰テストを補完する（replaceable/addressable の `created_at` 同値タイブレーク、`kind=5` の `e/a` 削除、`expiration` 到来後の `delete` 通知、ephemeral 非永続化）。
- [x] `cn-admin-api` + `cn-relay` + Admin Console: `node_subscriptions.ingest_policy`（保持期間/容量上限/バックフィル可否）を編集・保存・反映できるように実装し、`topic_subscription_design.md` / `ingested_record_persistence_policy.md` の node-level 制御要件に合わせる（契約テスト + 統合テスト）。
- [x] `cn-relay`: `retention` クリーンアップ（`events/event_topics`、`event_dedupe`、`events_outbox`、`deletion_tombstones`）の統合テストを追加し、保持期間ポリシーの後方互換を固定する。

## 未実装/不足事項（2026年02月13日 調査追記）

- [x] `cn-user-api`: `services_bootstrap.md` の HTTP キャッシュ要件（`ETag` は event JSON/レスポンスボディのハッシュ）に合わせ、`bootstrap::respond_with_events` の `ETag` 生成を `updated_at+件数` 方式からハッシュ方式へ変更する。あわせて「件数不変・同秒更新でも `ETag` が変化する」契約テストを追加する。
- [x] `cn-user-api` 契約テスト: `auth_transition_design.md` / `user_api.md` の要件に合わせ、bootstrap 認証必須モードで「認証済みだが未同意」の場合に `428 CONSENT_REQUIRED` を返すことを `GET /v1/bootstrap/nodes` と `GET /v1/bootstrap/topics/{topic_id}/services` で検証する（既存の `401 + WWW-Authenticate` テストとの境界を固定）。
- [x] `cn-cli`: `cn_cli_migration.md` の「有用サブコマンド維持 + daemon 起動対応」に対する回帰防止として、`migrate` / `config seed` / `admin bootstrap|reset-password` / `openapi export` / `p2p` 系の統合スモークテストを追加する。あわせて `cn bootstrap daemon` / `cn relay daemon` 形式をサポートするか、現行コマンド（`cn bootstrap` / `cn relay`）を正とするよう設計ドキュメントを更新して整合を取る。

## 未実装/不足事項（2026年02月13日 監査追記・追加）

- [x] `cn-relay`: `services_relay.md` の REQ 制約にある `since/until` 時間範囲の上限を実装する（`filters.rs` は現状 `#t`/filter数/値数/`limit` のみ制約し、時間範囲は未制約）。`since > until`・過大lookback・過大window の拒否理由を固定し、unit/integration テストを追加する。
- [x] `cn-moderation`: `outbox_notify_semantics.md` の consumer 要件（起動時catch-up、NOTIFY起床、offsetコミット、リプレイ）に対する統合テストを追加する。`load_last_seq`/`commit_last_seq`/`fetch_outbox_batch` 経路の回帰を検知できるようにし、at-least-once 前提の冪等性を検証する。
- [x] `cn-bootstrap` + クライアント経路: `services_bootstrap.md` の「通知受信→HTTP再取得」運用を実装側で閉じる。`pg_notify('cn_bootstrap_hint')` publish のみで止まっているため、受信側（bridge または client）の再取得トリガ実装と E2E テスト（hint受信で `/v1/bootstrap/*` キャッシュ更新）を追加する。

## 未実装/不足事項（2026年02月14日 監査追記）

- [x] （解消）Manager 決定により v1 は **A) relay/gossip bridge のみ** を採用（B: User API push は defer）。
- [x] 受信ブリッジ（A）を実装し、以下を満たす:
  - [x] hint 受信で `/v1/bootstrap/nodes` と `/v1/bootstrap/topics/{topic_id}/services` を再取得し、キャッシュを更新する
  - [x] 取りこぼし前提のため、既存 `next_refresh_at` ポーリングと併用する
  - [x] relay 側で `pg_notify('cn_bootstrap_hint')` を gossip ヒント配信へ橋渡しし、受信で HTTP 再取得が動作する統合テストを追加する

## 未実装/不足事項（2026年02月14日 ドキュメント運用追記）

- [x] Community Node テストコマンドを全OSでコンテナ既定に統一し、`README` / `docker_test_environment.md` / `ops_runbook.md` / `ci_required_checks_policy.md` / `AGENTS.md` / タスク文書の記載差分を解消する（Issue #5 ドキュメント整備）。

## 未実装/不足事項（2026年02月15日 監査追記）

- [x] CI 実装を Runbook 方針へ統一する。`docs/03_implementation/community_nodes/ops_runbook.md` は「community-node 回帰確認は全OSでコンテナ経路を既定」としているが、`.github/workflows/test.yml` の `community-node-tests` ジョブはホスト上で `cd kukuri-community-node && cargo test --workspace --all-features` を実行している。`test-runner` コンテナ経路（`cargo test --workspace --all-features` + `cargo build --release -p cn-cli`）へ置き換え、CI/Runbook/AGENTS の運用を一致させる。
  - **タスク**: `.github/workflows/test.yml` の `community-node-tests` ジョブを `docker run` + `test-runner` コンテナ経路へ変更
  - **PR**: `fix/test-runner-ci` ブランチで実装
- [x] Admin Console の `ServicesPage` で bootstrap 認証遷移フォームの回帰テストを追加する。現行テストは relay の auth transition を中心に検証しており、bootstrap 側（`auth.mode` / `enforce_at` / `grace_seconds` / `ws_auth_timeout_seconds`）の保存・バリデーション・version 衝突時の更新契約が未固定。
  - **タスク**: `apps/admin-console/src/pages/ServicesPage.svelte` に bootstrap 認証遷移フォームを追加
  - **PR**: `feat/admin-console-bootstrap-auth` ブランチで実装
- [x] `kukuri-tauri/src-tauri/src/state.rs` の 39000/39001 受信経路（`refresh_bootstrap_from_hint` + `ingest_bootstrap_event` 呼び出し）の統合テストを追加する。`community_node_handler.rs` 単体では hint 再取得を検証済みだが、P2P受信ハンドラ経由の連携（state 層）の回帰検知が不足しているため、gossip受信を含む導線テストで固定する。
  - **タスク**: `kukuri-tauri/src-tauri/tests/p2p_bootstrap_integration.rs` を追加し、P2P受信ハンドラ経由の連携を検証
  - **PR**: `feat/p2p-bootstrap-integration-test` ブランチで実装

## 未実装/不足事項（2026年02月15日 調査追記）

- [x] `cn-user-api` の bootstrap hint 受信 API（`GET /v1/bootstrap/hints/latest`）を OpenAPI/設計ドキュメントへ正式反映する。
  - `kukuri-community-node/crates/cn-user-api/src/lib.rs` にはルート実装があるが、`kukuri-community-node/crates/cn-user-api/src/openapi.rs` と `apps/admin-console/openapi/user-api.json` には未収載のため、仕様と生成物がずれている。
  - `docs/03_implementation/community_nodes/services_bootstrap.md` / `docs/03_implementation/community_nodes/user_api.md` に endpoint と利用条件（auth/consent/rate-limit）を追記する。
  - `kukuri-community-node/crates/cn-user-api/src/openapi_contract_tests.rs` に `/v1/bootstrap/hints/latest` の path 互換チェックを追加する。
- [x] 実ノード community-node E2E で `skip` が成功扱いのまま通過しないガードを追加する。
  - `kukuri-tauri/tests/e2e/specs/community-node*.spec.ts` は `SCENARIO` / `E2E_COMMUNITY_NODE_URL` 未設定時に `this.skip()` する設計で、`scripts/docker/run-desktop-e2e.sh` は `pnpm e2e:ci` の終了コードのみで成否判定しているため、設定不備時に実質未実行でも緑化し得る。
  - `SCENARIO=community-node-e2e` 実行時は community-node spec の pending/skip を失敗扱いにするか、最低実行件数を検証するチェックを `wdio` または `scripts/docker/run-desktop-e2e.sh` に追加する。
  - 追加ガードが `scripts/test-docker.sh e2e-community-node` / `scripts/test-docker.ps1 e2e-community-node` / `.github/workflows/test.yml` の `desktop-e2e` で有効化されることをテストで固定する。

## 未実装/不足事項（2026年02月15日 再調査追記）

- [ ] `cn-user-api`: `topic_subscription_design.md` の DoS 要件（申請の同時保留数上限 per pubkey）を実装する。現状 `create_subscription_request` は `check_topic_limit`（active 件数）しか見ておらず pending 件数を制御していないため、上限判定と拒否レスポンス契約（status code / error code / details）を定義して反映する。
- [ ] `cn-user-api` 契約テスト: 申請の同時保留数上限を追加検証する（上限未満で受理、上限到達時に拒否、approve/reject 後に再申請可能）。
- [ ] `cn-admin-api` + `cn-user-api` + `cn-relay`: `topic_subscription_design.md` の DoS 要件（node-level の同時取込 topic 数上限）を実装する。現状 `approve_subscription_request` は `cn_admin.node_subscriptions` を無制限で upsert するため、上限設定と承認時の超過拒否フローを追加する。
- [ ] テスト補完: node-level topic 上限の回帰テストを追加する（`cn-admin-api` 契約テストで承認拒否契約を固定、`cn-relay` 統合テストで上限超過時に新規 topic subscribe が増えないことを検証）。

## 参照（設計）

- `docs/03_implementation/community_nodes/summary.md`（全体方針とマイルストーン）
- `docs/03_implementation/community_nodes/architecture_overview.md`（責務分割/データフロー）
- `docs/03_implementation/community_nodes/repository_structure.md`（構成案）
