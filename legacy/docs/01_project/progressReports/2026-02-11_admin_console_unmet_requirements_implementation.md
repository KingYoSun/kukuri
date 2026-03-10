# 2026年02月11日 Admin Console 未充足要件実装（Moderation/Trust/Access Control）

## 概要

- 対象: `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` の未実装項目  
  - `admin_console.md` の未充足要件（Moderation のルールテスト実行、Trust のパラメータ/対象検索、Access Control の invite.capability 運用）
- 方針: スコープ縮退は行わず、未充足要件を実装で完了
- 結果: API/UI 実装と API/UI テスト追加を完了し、roadmap 該当チェックを完了へ更新

## 実装内容

### 1. `cn-admin-api`（API 実装）

- Moderation
  - `POST /v1/admin/moderation/rules/test` を追加
  - ルール条件とサンプルイベントを受け取り、判定結果とラベルプレビューを返却
- Trust
  - `GET /v1/admin/trust/targets` を追加
  - pubkey 条件で対象検索し、report/communication スコアを返却
- Access Control
  - `GET /v1/admin/access-control/invites`（一覧）
  - `POST /v1/admin/access-control/invites`（発行）
  - `POST /v1/admin/access-control/invites/{nonce}/revoke`（失効）
  - invite.capability の運用（発行/検索/失効）を API として提供

### 2. `cn-admin-api`（契約/OpenAPI テスト）

- `contract_tests.rs` に以下の成功系契約テストを追加
  - Moderation ルールテスト
  - invite.capability の発行/一覧/失効
  - Trust 対象検索
- `openapi_contract_contains_admin_paths` を拡張し、新規パス収載を検証

### 3. Admin Console（UI 実装）

- `ModerationPage` に Rule Test Runner を追加（サンプル入力 + 実行結果表示）
- `TrustPage` に以下を追加
  - Trust parameters 編集/保存
  - target 検索（pubkey 条件）
- `AccessControlPage` に invite.capability 運用 UI を追加
  - 発行フォーム
  - 一覧/検索
  - revoke 操作

### 4. Admin Console（UI テスト）

- `ModerationPage.test.tsx` にルールテスト実行の API 呼び出し検証を追加
- `TrustPage.test.tsx` にパラメータ保存/対象検索の API 呼び出し検証を追加
- `AccessControlPage.test.tsx` に invite 発行/検索/失効の API 呼び出し検証を追加

## 主要変更ファイル

- `kukuri-community-node/crates/cn-admin-api/src/moderation.rs`
- `kukuri-community-node/crates/cn-admin-api/src/trust.rs`
- `kukuri-community-node/crates/cn-admin-api/src/access_control.rs`
- `kukuri-community-node/crates/cn-admin-api/src/lib.rs`
- `kukuri-community-node/crates/cn-admin-api/src/openapi.rs`
- `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
- `kukuri-community-node/apps/admin-console/src/pages/ModerationPage.tsx`
- `kukuri-community-node/apps/admin-console/src/pages/TrustPage.tsx`
- `kukuri-community-node/apps/admin-console/src/pages/AccessControlPage.tsx`
- `kukuri-community-node/apps/admin-console/src/pages/ModerationPage.test.tsx`
- `kukuri-community-node/apps/admin-console/src/pages/TrustPage.test.tsx`
- `kukuri-community-node/apps/admin-console/src/pages/AccessControlPage.test.tsx`

## 検証結果

- API/UI 追加分の個別検証
  - `cargo test --workspace --all-features`（community-node, Docker 実行）: 成功
  - `cargo clippy --workspace --all-features -- -D warnings`（community-node, Docker 実行）: 成功
  - `apps/admin-console` の Vitest（対象ページ指定）: 成功
- リポジトリ標準検証
  - `./scripts/test-docker.ps1 rust -NoBuild`: 成功
  - `./scripts/test-docker.ps1 ts -NoBuild`: 成功
- 必須 `gh act` ジョブ
  - `format-check`: 初回失敗（community-node の fmt 差分）→ `cargo fmt --all` 後に成功
  - `native-test-linux`: 成功
  - `community-node-tests`: 初回失敗（`kukuri-postgres-age` 名衝突）→再実行で成功

## 付記

- タスク管理更新を実施
  - `community_nodes_roadmap.md` の該当項目を `[x]` 化
  - `in_progress.md` から当該タスクを削除
  - `completed/2026-02-11.md` へ完了内容/検証ログを追記
