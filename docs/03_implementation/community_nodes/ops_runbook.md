# 運用要件 / Runbook（v1）

**作成日**: 2026年01月23日  
**対象**: `./kukuri-community-node`（Docker Compose 運用）

## 目的

- コミュニティノードの運用に必要な要件（監視/メトリクス/ログ、バックアップ/リストア、マイグレーション、違法/通報対応）を v1 方針として確定する
- 「P2P + gossip を前提とした制約（過去の回収不能、重複配送）」を踏まえ、現実的に事故対応できる手順を用意する

## 参照（ローカル）

- `docs/03_implementation/community_nodes/docker_compose_profiles.md`（observability profile）
- `docs/03_implementation/community_nodes/outbox_notify_semantics.md`（outbox backlog 指標）
- `docs/03_implementation/community_nodes/event_treatment_policy.md`（削除/期限切れの扱い）
- `docs/03_implementation/community_nodes/billing_usage_metering.md`（クォータ超過/監査）
- `docs/03_implementation/community_nodes/llm_moderation_policy.md`（外部送信/停止条件）
- `docs/03_implementation/community_nodes/personal_data_handling_policy.md`（個人データの保持/削除・エクスポート/同意ログ）

## 0. 運用原則（v1）

- **入口は統合**: 外部I/Fは `user-api`（HTTP）+ `relay`（WS）を正とし、他サービスは内部NWに閉じる
- **DBが正**: 取込レコード・監査・購読・同意・ジョブ状態は Postgres を正とする（Meilisearch は派生）
- **最小開示**: ログ/監査は必要最小限（本文・識別子を不用意に残さない）
- **停止できる設計**: LLM/バックフィル/低優先topicは止められる（運用で破綻しない）

### 0.1 Community Node テスト実行の既定コマンド（全OS共通）

- community-node の回帰確認は Linux/macOS/Windows すべてでコンテナ経路を既定とする。`cd kukuri-community-node && cargo test ...` のホスト直実行はデバッグ用途に限定する。
- 依存サービス起動:
  - `docker compose -f docker-compose.test.yml up -d community-node-postgres community-node-meilisearch`
- test-runner イメージビルド:
  - `docker compose -f docker-compose.test.yml build test-runner`
- テスト + ビルド:
  - `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -e MEILI_URL=http://community-node-meilisearch:7700 -e MEILI_MASTER_KEY=change-me -v "$(git rev-parse --show-toplevel):/workspace" -w /workspace/kukuri-community-node kukuri-test-runner bash -lc "set -euo pipefail; source /usr/local/cargo/env; cargo test --workspace --all-features; cargo build --release -p cn-cli"`

## 1. 監視 / メトリクス / ログ

### 1.1 収集方針

- ログ: 全サービス `stdout`（JSON 推奨）→ compose の log driver で集約
- メトリクス: `/metrics`（Prometheus 形式）を推奨
- ヘルス: `GET /healthz`（依存関係込みの “ready” を返す）
- `observability` profile で Prometheus/Grafana/OTel Collector を起動できるようにする（詳細: `docs/03_implementation/community_nodes/docker_compose_profiles.md`）

### 1.2 ログの最小化（PII/本文）

必須フィールド（例）:
- `timestamp`, `level`, `service`, `version`
- `request_id`（HTTPは `X-Request-Id`、内部は生成）
- `event_id`（可能なら）
- `topic_id`（可能なら）

原則ログに出さない:
- `event.content`（本文）
- pubkey 生値、IP、User-Agent、JWT、メール等の識別子

必要な場合の代替:
- `pubkey_hash`（salt付きハッシュ）を利用し、運用上の追跡だけ可能にする

### 1.3 必須メトリクス（v1最小）

#### 共通
- `http_requests_total{service,route,method,status}`
- `http_request_duration_seconds_bucket`
- `process_resident_memory_bytes` 等（ランタイム標準）

#### relay
- WS: `ws_connections`, `ws_req_total`, `ws_event_total`
- ingest: `ingest_received_total{source=iroh|ws}`, `ingest_rejected_total{reason=invalid|ratelimit|auth|consent|quota|other}`
- iroh-gossip: `gossip_received_total`, `gossip_sent_total`
- dedupe: `dedupe_hits_total`, `dedupe_misses_total`

#### outbox（下流追従）
- consumer別 backlog（推奨）:
  - `backlog = max(seq) - consumer_offsets.last_seq`
  - 詳細: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
- consumer別エラー率、処理レイテンシ、batch size
  - `outbox_consumer_batches_total{service,consumer,result}`（`result=success|error`）
  - `outbox_consumer_processing_duration_seconds{service,consumer,result}`
  - `outbox_consumer_batch_size{service,consumer}`

#### user-api（認証/同意/課金）
- `auth_success_total`, `auth_failure_total`
- `consent_required_total`
- `quota_exceeded_total{metric}`（詳細: `docs/03_implementation/community_nodes/billing_usage_metering.md`）

#### index/moderation/trust
- outbox追従遅延（consumer_offsetsの遅れ）
- 依存（Meilisearch/LLM/AGE）の失敗率

#### Postgres
- 接続数/ロック待ち/ディスク使用率（最小でもディスク逼迫は必須アラート）

### 1.4 アラート例（v1）

- outbox backlog が閾値超過（consumer停止/詰まり）
- relay reject 急増（攻撃/設定ミス/認証切替失敗）
- DB ディスク逼迫、接続枯渇、ロック待ち増大
- Meilisearch 同期停止（index consumer の停止）
- LLM 連続失敗/予算上限到達（LLM自動停止に落ちているか）

### 1.5 検索 PG cutover 監視（Issue #27 / PR-07）

- 参照 Runbook: `docs/01_project/activeContext/search_pg_migration/PR-07_cutover_runbook.md`
- cutover 期間（`search_read_backend=pg` の段階適用中）は、以下を Dashboard の必須パネルとして扱う。
  - Search latency: `http_request_duration_seconds{service=\"cn-user-api\",route=\"/v1/search\"}`（p50/p95/p99）
  - Search error rate: `http_requests_total{service=\"cn-user-api\",route=\"/v1/search\",status=~\"5..\"}` / 総リクエスト
  - Suggest latency: `suggest_stage_a_latency_ms{service=\"cn-user-api\"}` / `suggest_stage_b_latency_ms{service=\"cn-user-api\"}`
  - Suggest filter drop: `suggest_block_filter_drop_count{service=\"cn-user-api\",backend=\"pg\"}`
  - Shadow quality: `shadow_overlap_at_10` / `shadow_latency_delta_ms`
  - Index lag: `outbox_backlog{service=\"cn-index\",consumer=\"index-v1\"}`
- zero-result は専用メトリクスが未実装のため、`cn_search.shadow_read_logs` を代理指標として SQL で収集する。
- 閾値は PR-07 Runbook の品質/性能ゲート（`overlap@10 >= 0.70`, `NDCG@10 >= 0.90`, 検索 P95 `<= 180ms`, サジェスト P95 `<= 80ms`）に揃える。

## 2. バックアップ / リストア

### 2.1 何が正か（優先度）

必須:
- Postgres（全スキーマ: `cn_*`）

任意（再構築可能）:
- Meilisearch（壊れたら reindex できる前提）

別管理（DBに入れない）:
- Node鍵（署名鍵）、JWT secret、OpenAI key 等の秘匿情報

### 2.2 Postgres バックアップ（v1推奨）

- 日次 `pg_dump`（圧縮、世代管理）
- リリース前後: schema-only dump を保存
- 月1でリストア演習（Runbook化して “復旧できる” を保証）

v2（必要なら）:
- WAL アーカイブ + PITR（RPO/RTO 要件が出た時点で）

### 2.3 リストア手順（概要）

1. 既存コンテナ停止（書き込み停止）
2. Postgres volume を退避（事故調査用）
3. 新しい DB へリストア（`pg_restore`）
4. マイグレーション適用（必要なら）
5. services 起動（`relay`→`user-api`→各worker）
6. Meilisearch は必要なら reindex（index ジョブ）

### 2.4 運用スクリプト（`pg_dump` 世代管理 + `pg_restore` 復旧ドリル）

- 実行コマンド: `./scripts/test-docker.ps1 recovery-drill`
- 世代管理: `COMMUNITY_NODE_BACKUP_GENERATIONS`（既定: `30`）
- バックアップ出力: `test-results/community-node-recovery/backups/community-node-pgdump-<timestamp>.dump`
- 復旧ドリルログ: `tmp/logs/community-node-recovery/<timestamp>.log`
- 復旧ドリル結果: `test-results/community-node-recovery/<timestamp>-summary.json` と `test-results/community-node-recovery/latest-summary.json`
- 定期CI（2026年02月11日反映）: `.github/workflows/nightly.yml` の `community-node-recovery-drill` が UTC 毎月1日（または `workflow_dispatch`）に `./scripts/test-docker.ps1 recovery-drill` を実行し、`latest-summary.json` の整合を検証した上で artefact を収集する

ドリル内容:
1. `community-node-user-api` を起動し E2E seed を投入
2. `cn_relay.events` 件数を基準値として記録
3. `pg_dump`（custom format + 圧縮）でバックアップ作成し、古い世代を自動削除
4. `user-api/bootstrap` を停止して書き込みを止め、`TRUNCATE` で障害を模擬
5. `pg_restore` で DB を再作成して復旧
6. サービス再起動後に `cn_relay.events` 件数が基準値へ戻ることを検証

## 3. マイグレーション手順（v1）

- マイグレーションは `kukuri-community-node/migrations/` に集約し、**単一の migrate ジョブ**で実行する
- 起動順序:
  1. Postgres 起動
  2. migrate 実行（排他ロックで二重実行防止）
  3. 各サービス起動

### 3.1 変更方式（expand/contract）

ロールバック不能な本番事故を避けるため、原則として “後方互換” にする。

1. expand（列/テーブル追加、旧コードでも動く）
2. 新旧両対応でアプリ展開
3. backfill（必要なら）
4. 切替
5. contract（不要列/旧テーブル削除）

### 3.2 Apache AGE（拡張）の注意

- `CREATE EXTENSION age` と graph 初期化は “再実行可能（IF NOT EXISTS）” に寄せる
- 失敗時の典型原因（権限/イメージ差分）を運用メモに残す

## 4. 違法/通報対応 Runbook（v1最小）

### 4.1 前提（制約）

- iroh-gossip/WS で拡散済みの過去イベントは回収できない
- できることは「このノードが **これ以上配らない/検索させない/推奨しない**」に寄る

### 4.2 初動（triage）

- 重大度分類（違法/緊急/通常）と責任者の確定
- 監査ログ/関連IDの保全
  - `event_id`, `topic_id`, report_id, label_id 等（本文は原則ログに残さない方針を維持）
- 必要なら当該 topic を一時停止（後述）

### 4.3 封じ込め（推奨アクション）

relay:
- topic 単位で受理停止（WS publish 拒否、iroh-gossip subscribe 停止）
- レート制限強化（IP/鍵）

index:
- 対象イベントを検索結果から除外（delete/upsertの反映）
- denylist（event_id/author）を持つ場合は reindex でも復活しないようにする

moderation:
- ルールベースで即時ラベル（`label(39006)`）発行
- LLM はポリシーに従い（外部送信は既定OFF、予算上限/停止条件必須）必要に応じて利用

trust:
- 対象のスコア算出/配布から除外し、必要なら再計算

### 4.4 対外対応（最小）

- `policy_url` の連絡先に沿って受付（テンプレ: 受付→調査→対応→完了）
- 法的要請への対応方針（開示範囲/保持期間）は Privacy と監査ログに整合させる

### 4.5 復旧/再発防止

- 設定の戻し（topic再開/レート調整/denylist解除の判断）
- 事後メモ（原因、検知、対策、今後の運用改善）

## 5. 個人データ（削除/エクスポート）対応 Runbook（v1最小）

前提:
- 原則は User API の self-service（削除/エクスポート要求）で処理する
- P2P で拡散済みのイベントは回収できないため「本ノードがこれ以上保持/検索/再配信しない」までを保証範囲とする

### 5.1 エクスポート要求（DSAR export）

1. ユーザーが User API へ申請（例: `POST /v1/personal-data-export-requests`）
2. export job をキューし、生成物（zip）を短期保持（例: 24時間）
3. ユーザーへ download URL/token を返す
4. 期限切れ後に生成物を自動削除

運用メモ:
- 生成物は “第三者データを含めない” ようにフィルタする（`event.pubkey == subject_pubkey` 等）
- 生成物へのアクセスログにも本文/識別子を残さない（必要ならハッシュ化）

### 5.2 削除要求（DSAR deletion）

1. ユーザーが User API へ申請（例: `POST /v1/personal-data-deletion-requests`）
2. 即時に失効/無効化を反映し、以後の保護 API を拒否（JWT でも DB の状態で即時拒否できる）
3. deletion job を実行し、DB データを削除/匿名化し、派生データ（Meilisearch/AGE）を削除/再計算
4. 完了を監査ログへ記録し、ユーザーへ status を返す（完了までの目安時間を返す）

注意:
- 削除要求は “全消去” ではなく、監査/濫用調査のために識別子を匿名化したログ/監査イベントを残す場合がある（詳細: `docs/03_implementation/community_nodes/personal_data_handling_policy.md`）
- relay/bootstrap が認証OFFの間は pubkey を特定できず、同意/削除要求を relay 側で強制できない（User API 管轄に限定する）

### 5.3 バックアップ/リストアとの整合

- Postgres バックアップ（`pg_dump`）には削除前のデータが含まれるため、削除要求の反映はバックアップの自然消滅まで遅延し得る（Privacy に明記）
- リストア（過去のバックアップへ復旧）を行う場合、復旧後に「削除要求の再適用」が必要になる可能性がある
  - v1 は運用手順で担保（削除要求の受付/完了を監査ログにも残し、復旧後に照合して再実行）
