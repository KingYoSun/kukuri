# Community Nodes 進捗レポート（Admin Console 認証遷移フォーム + 施行状態可視化）

作成日: 2026年02月12日

## 概要

`auth_transition_design.md` の運用要件に合わせ、Admin Console `Services` の relay/bootstrap 設定を生 JSON 編集から専用フォームへ移行した。あわせて relay の auth 施行状態を UI で可視化し、バックエンドのメトリクス収集と契約テストを追加した。

## 実装内容

1. Admin Console（`ServicesPage`）
- relay/bootstrap を専用 `Auth Transition` カードで表示し、以下をフォーム編集可能にした。
  - `auth_mode`
  - `enforce_at`
  - `grace_seconds`
  - `ws_auth_timeout_seconds`
- relay の施行状態表示を追加した。
  - phase（`Auth off` / `Scheduled` / `Grace period` / `Required`）
  - `enforce_at`
  - `disconnect_unauthenticated_at`
  - enforce/切断まで残時間
- relay runtime signals 表示を追加した。
  - 未AUTH接続残数
  - auth-required reject 累積
  - timeout/deadline 切断累積
- relay/bootstrap 以外のサービスは既存 JSON editor を維持した。

2. Relay メトリクス拡張（`cn-core` / `cn-relay`）
- 追加メトリクス:
  - `ws_unauthenticated_connections`（gauge）
  - `ws_auth_disconnect_total{reason}`（counter）
- WebSocket 処理で以下を計測するようにした。
  - 接続開始時の未AUTH接続増加
  - AUTH成功時の未AUTH接続減算
  - timeout/deadline による切断カウント
  - auth-required 拒否時の `ingest_rejected_total{reason="auth"}` 増加

3. Admin API health poll 拡張（`cn-admin-api`）
- relay の `healthz` ポーリング時に `/metrics` を追加取得し、`details_json.auth_transition` へ以下を格納するようにした。
  - `ws_connections`
  - `ws_unauthenticated_connections`
  - `ingest_rejected_auth_total`
  - `ws_auth_disconnect_timeout_total`
  - `ws_auth_disconnect_deadline_total`
  - `metrics_status` / `metrics_error`
- Prometheus テキストの合算パーサを追加し、ラベル付き集計を実装した。

4. テスト追加・更新
- `cn-admin-api` 契約テスト:
  - relay auth transition メトリクス収集の後方互換を追加
- `cn-relay` 統合テスト:
  - 新規メトリクス公開の契約確認を追加
- Admin Console Vitest:
  - 専用フォーム保存
  - 施行状態表示
  - 不正入力拒否
  - relay/bootstrap 以外の JSON 編集維持

## 主な変更ファイル

- `kukuri-community-node/apps/admin-console/src/pages/ServicesPage.tsx`
- `kukuri-community-node/apps/admin-console/src/pages/ServicesPage.test.tsx`
- `kukuri-community-node/crates/cn-core/src/metrics.rs`
- `kukuri-community-node/crates/cn-relay/src/ws.rs`
- `kukuri-community-node/crates/cn-relay/src/integration_tests.rs`
- `kukuri-community-node/crates/cn-admin-api/src/services.rs`
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`

## 検証結果

- `./scripts/test-docker.ps1 ts` 成功
- `./scripts/test-docker.ps1 rust` 成功
- Admin Console `ServicesPage.test.tsx` 成功
- `cargo test --workspace --all-features`（`kukuri-community-node`）成功
  - 初回は既存DB残骸で `events_pkey` 重複が発生
  - Postgres テストコンテナを tmpfs 再起動して再実行し成功
- `gh act --job format-check` 成功（最終ログ: `tmp/logs/gh-act-format-check-auth-transition-20260212-final.log`）
- `gh act --job native-test-linux` 成功（最終ログ: `tmp/logs/gh-act-native-test-linux-auth-transition-20260212-final.log`）
- `gh act --job community-node-tests` 成功（最終ログ: `tmp/logs/gh-act-community-node-tests-auth-transition-20260212-rerun.log`）

