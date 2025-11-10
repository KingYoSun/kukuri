# Trending Metrics Job 実装ガイド
作成日: 2025年11月07日  
最終更新: 2025年11月10日

## 背景
- Phase 5 で `/trending` `/following` フィードを実装したが、現状のトレンドスコアはリクエスト時に `topics` / `posts` テーブルから生計算しており、ピーク時にクエリ負荷が高い。
- Summary Panel や Docker シナリオ `trending-feed` で安定した検証を行うため、時間窓集計済みのメトリクスを提供し、キャッシュ刷新と監視を容易にする必要がある。
- 既存の `topic_metrics` テーブル（`posts_24h` など）は未活用のまま残っており、再利用することで大規模リファクタを避けつつ導線の信頼性を高められる。

## 目的
- 直近 24 時間 / 6 時間の投稿数・参加者数・エンゲージメントをバッチで集計し、`list_trending_topics` / `list_trending_posts` が高速にレスポンスできる状態にする。
- 集計ジョブ失敗時の検知・復旧手順を確立し、Nightly / Docker シナリオからも同値検証できるようにする。
- 将来的なリアルタイム更新（WebSocket 等）へ拡張しやすい設計を採用する。

## 現状サマリー（2025年11月10日更新）
- `app.conf`（`AppConfig.metrics`）に `enabled` / `interval_minutes` / `ttl_hours` / `score_weights.posts|unique_authors|boosts` / `prometheus_port` / `emit_histogram` を追加し、`KUKURI_METRICS_*` 環境変数で上書き可能にした。既定値は「有効」「5 分間隔」「TTL 48h」「0.6/0.3/0.1」「prometheus_port: 未設定」「histogram: false」。
- `AppState::new` で `TrendingMetricsJob` を `Arc` 付き 5 分ループとして `tauri::async_runtime::spawn` し、起動直後から `run_once` → `sleep(interval)` を繰り返す。失敗は `metrics::trending` 名前空間で ERROR ログに出力。
- 2025年11月10日: `infrastructure/jobs/trending_metrics_metrics.rs` に Prometheus レジストリ（`runs_total` `failures_total` `topics_upserted` `expired_records` `last_success_timestamp` `last_failure_timestamp` `duration_seconds`）を実装し、`TrendingMetricsJob` が成功/失敗ごとに更新。`KUKURI_METRICS_PROMETHEUS_PORT` を設定すると 127.0.0.1 バインドの `tiny_http` サーバーが `/metrics` エンドポイントを公開する。
- `TopicMetricsRepository` に `latest_window_end` / `list_recent_metrics(limit)` を追加し、`topic_metrics` から直近ウィンドウを一括取得できるようにした。`TopicService::list_trending_topics` はメトリクスが有効かつ最新レコードが存在する場合に `TrendingDataSource::Metrics` を返却し、フォールバックとして従来のリアルタイム集計を保持する。
- `topic_handler::list_trending_topics` / `post_handler::list_trending_posts` はどちらも `generated_at = topic_metrics.window_end` を返却するため、Summary Panel と Docker `trending-feed` シナリオの比較が 1:1 で行える。
- CI: `pnpm vitest run routes/trending.test.tsx routes/following.test.tsx src/tests/unit/hooks/useTrendingFeeds.test.tsx` と `scripts/test-docker.sh ts --scenario trending-feed --no-build` を Nightly で実行。`trending_metrics_job` のローカル実行は `cargo test trending_metrics_job::*` / `cargo fmt` / `cargo test --package kukuri-tauri` に含まれる。

## 要件

### 機能要件
- 集計対象: トピック別に `posts_count`, `unique_authors`, `boosts`, `replies`, `bookmarks`, `participant_delta` を算出。
- 時間窓: 24h ローリング（主要指標）、6h ローリング（速報用）。必要に応じて 1h / 7d を追加可能な拡張性を確保。
- 書き込み先: `topic_metrics`（既存）に `window_start`, `window_end`, `posts_24h`, `posts_6h`, `score_24h`, `score_6h`, `updated_at` を追加。過去レコードは TTL 48h で削除。
- 点数計算: `score = posts * 0.6 + unique_authors * 0.3 + boosts * 0.1`。将来変更に備えて `metrics::ScoreWeights` 構造体で定義し、設定ファイルで上書き可能にする。
- API 連携: `TopicService::list_trending_topics` / `PostService::list_trending_posts` を `topic_metrics` を参照する実装へリファクタし、メトリクスが欠落している場合のみフォールバック計算を行う。
- 冪等性: ウィンドウ計算は UPSERT で更新し、ジョブ再実行で重複が生じないようにする。

### 非機能要件
- 実行頻度: 5 分間隔（Cron 形式 `0 */5 * * * *` 相当）でキューイング。バックオフ: 3 回連続失敗時に 15 分スキップ。
- 実行時間: 1 回 2 秒以内（対象トピック 5,000 件を上限）。
- 可観測性: Prometheus へ `trending_metrics_job_duration_seconds`・`trending_metrics_job_last_success_timestamp`・`trending_metrics_job_topics_upserted`・`trending_metrics_job_expired_records` をエクスポート。ログは `metrics::trending` 名前空間で JSON 出力。
- コンフィグ: `app.toml` の `metrics` セクションに `enabled`, `interval_minutes`, `score_weights`, `prometheus_port`, `emit_histogram` を追記。
- リソース制限: ジョブ実行時は read-only で `posts` テーブルへアクセスし、不要なロックを避ける。必要に応じて `READ UNCOMMITTED` レベルで統計を取得。

## Prometheus エクスポート手順（2025年11月10日追加）

1. `KUKURI_METRICS_ENABLED=true` のまま、任意のポートを `KUKURI_METRICS_PROMETHEUS_PORT=9898`（例）として設定する。ヒストグラムが必要な場合は `KUKURI_METRICS_EMIT_HISTOGRAM=true` を併用。
2. アプリ起動後に `curl http://localhost:9898/metrics` を実行すると以下のようなメトリクスが返る。`runs_total`／`failures_total`／`topics_upserted`／`expired_records`／`last_success_timestamp`／`duration_seconds_bucket` を監視対象とする。

```text
# HELP kukuri_trending_metrics_job_runs_total Total number of successful trending metrics job executions
# TYPE kukuri_trending_metrics_job_runs_total counter
kukuri_trending_metrics_job_runs_total 3
kukuri_trending_metrics_job_topics_upserted 42
kukuri_trending_metrics_job_last_success_timestamp 1731229205123
```

3. `tiny_http` サーバーは 127.0.0.1 のみにバインドされる。外部公開が必要な場合はリバースプロキシか SSH トンネルを利用する。
4. 生成されたメトリクスは Runbook Chapter7 と `phase5_ci_path_audit.md` で `curl` / `scripts/metrics/export-p2p --job trending`（今後追加予定）から収集する。

## アーキテクチャ案
- モジュール構成
  - `infrastructure/jobs/trending_metrics_job.rs`: 集計ロジックの本体。`TopicMetricsRepository` と `PostRepository` を依存注入。
  - `application/services/trending_metrics_service.rs`: ジョブのライフサイクル制御（起動/停止/再実行要求）。
  - `infrastructure/repositories/topic_metrics_repository.rs`: UPSERT/GC を担当。SQLx クエリは `.sqlx/` に保存。
- フロー
  1. `AppState` 起動時に `JobScheduler`（既存）へ `TrendingMetricsJob` を登録。
  2. ジョブ実行で `collect_raw_metrics(window)` → `calculate_score(weight)` → `persist(window, score)` の順に実行。
  3. 成功時に `metrics::trending_last_success` を更新し、Nightly や Runbook で参照できるようにする。
  4. 失敗時は `errorHandler.log('TrendingMetricsJob.failed', …)` を経由し、Sentry / Slack 連携でアラートを通知。

## 実装ステップ
1. **スキーマ更新**
   - `migrations/` に `topic_metrics` の列追加 migration を作成。
   - `sqlx prepare` を実行し `.sqlx/` を更新。
2. **リポジトリ層実装**
   - `TopicMetricsRepository::upsert(window_start, window_end, metrics)` を追加。
   - GC バッチ `cleanup_expired(now, ttl_hours=48)` を用意。
3. **ジョブ本体**
   - `TrendingMetricsJob::run()` でウィンドウごとに計算し、並列度 4 程度でバッチ処理。
   - 指数バックオフと `JobError` の分類（致命/一過性）を実装。
4. **サービス/DI**
   - `JobScheduler` に `TrendingMetricsJob` の登録を追加。`app.toml` で `metrics.enabled=false` の場合はスキップ。
5. **API 更新**
   - `TopicService::list_trending_topics` で `topic_metrics` を読み込み、スコア・順位計算を集計値ベースに変更。
   - `PostService::list_trending_posts` も `topic_metrics` の結果を利用して topic ごとの上限を判定。
6. **テスト整備**
   - ユニット: スコア計算、ウィンドウ切替、GC。
   - 統合: SQLite を使った end-to-end（`cargo test trending_metrics_job::*`）。
   - Docker: `trending-feed` シナリオでジョブを有効にし、Summary Panel が集計値と一致するかを確認。
7. **ドキュメント更新**
   - 本ドキュメントを v1.0 として確定し、`phase5_ci_path_audit.md` / `p2p_mainline_runbook.md` にリンク。

→ 2025年11月08日: 上記 1〜7 を `shared/config.rs` / `application::services::topic_service.rs` / `presentation/handlers/topic_handler.rs` / `state.rs` / `scripts/test-docker.*` で実装・反映済み。

## 監視・運用
- ダッシュボード:
  - Prometheus クエリ `trending_metrics_job_duration_seconds`（p95, p99）でスパイクを監視。
  - `trending_metrics_job_last_success_timestamp` を用いて 10 分以上未更新でアラート。
- ログ:
  - 成功: `{"event":"trending_metrics_job.completed","window":"24h","topics":1234,"duration_ms":820}`。
  - 失敗: `{"event":"trending_metrics_job.failed","error":"sqlx::Error","retry_in_seconds":900}`。
- アラート:
  - 2 回連続失敗で Slack `#alerts-trending` に通知し、Runbook のトリアージ手順（原因切り分け→再実行→バックフィル）を実行。
  - テーブルサイズが閾値（例: 10 万行）を超えた場合は GC の設定値を確認するよう警告を出す。
- バックフィル:
  - 新ジョブ投入時は `--backfill --minutes 1440` フラグで初期集計を実行し、結果を `topic_metrics_history`（一時テーブル）へ保存して比較可能にする。

## テスト戦略
- SQLx query tests: `#[sqlx::test(migrations = "...")]` で、24h/6h 同時計算パスを検証。
- Property test: QuickCheck で score 計算が重複投稿に対して冪等であることを検証。
- Load test: `cargo bench -p kukuri-tauri --bench trending_metrics_job` を追加し、1, 5, 10 万トピックでの処理時間を計測。
- Nightly: `nightly.yml` の `Trending Feed (Docker)` ジョブで `METRICS_JOB_ENABLED=1` を設定し、実行ログを artefact 化。

## リスクと対応
- **長時間ロック**: 集計クエリが `posts` テーブルにロックを保持しないよう `READ UNCOMMITTED` と LIMIT/OFFSET を駆使。必要なら一時テーブルに抽出してから集計する。
- **スコア振れ幅**: 新指標導入で UI が急変する可能性。`score_weights` の Feature Flag（`metrics.score_profile`) を用意し、段階的にロールアウト。
- **ジョブ停止時のフォールバック**: `TopicService` でメトリクス未更新（`updated_at` > 15 分）を検知した際は従来の生計算で応答し、UI には「一時的に集計結果が旧データ」のバナーを表示する。

## 関連ドキュメント
- `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` 5.7 節 — Docker シナリオと導線ギャップの整理。
- `docs/01_project/activeContext/artefacts/phase5_ci_path_audit.md` — CI 経路と Nightly 構成の更新計画。
- `docs/03_implementation/p2p_mainline_runbook.md` — 監視と障害対応手順（今後追記予定）。
- `docs/03_implementation/docker_test_environment.md` — Docker テスト手順とシナリオ一覧。
