# Community Nodes Admin Console: LLM連携設定 + Memberships検索 UI 実装（2026年02月07日）

最終更新日: 2026年02月07日

## 概要

`community_nodes_roadmap.md` の未実装項目であった「Admin Console の LLM 連携設定 UI」と「Access Control memberships 一覧/検索 UI」を実装し、Admin API/OpenAPI/Console の整合をとった。

## 実施内容

- Admin API
  - `GET /v1/admin/access-control/memberships` を追加（`topic_id`/`scope`/`pubkey`/`status`/`limit`）
  - `pubkey` は 64hex の場合に正規化して完全一致、非64hexは部分一致検索に対応
  - OpenAPI (`/v1/openapi.json`) へ path/components を追加し、契約テストを更新
- Admin Console Access Control
  - memberships 専用セクションを追加
  - `topic + scope + pubkey + status` で検索し、結果をテーブル表示
  - rotate/revoke 実行後に memberships query も再取得するように更新
- Admin Console Moderation
  - LLM 連携専用フォームを追加
  - 設定項目:
    - Provider（`disabled/openai/local`）
    - 外部送信 ON/OFF（OpenAI 利用時）
    - 送信範囲（`public/invite/friend/friend_plus`）
    - 保存/保持（decision/snapshot persist + retention days）
    - 予算上限（requests/cost/concurrency）
  - `service=moderation` の service config と監査ログ表示を接続
- Seed/型生成
  - `cn-core` の moderation seed を `send_scope`/`storage`/`retention` 対応へ拡張
  - Admin Console の OpenAPI 生成JSONと TS 生成クライアントを更新

## 技術的詳細

- 主要更新ファイル:
  - `kukuri-community-node/crates/cn-admin-api/src/access_control.rs`
  - `kukuri-community-node/crates/cn-admin-api/src/openapi.rs`
  - `kukuri-community-node/apps/admin-console/src/pages/AccessControlPage.tsx`
  - `kukuri-community-node/apps/admin-console/src/pages/ModerationPage.tsx`
  - `kukuri-community-node/crates/cn-core/src/admin.rs`
- 追加テスト:
  - `kukuri-community-node/crates/cn-admin-api/src/contract_tests.rs`
  - `kukuri-community-node/apps/admin-console/src/pages/ModerationPage.test.tsx`
  - `kukuri-community-node/apps/admin-console/src/pages/AccessControlPage.test.tsx`（更新）

## 次のステップ

- `community_nodes_roadmap.md` の残タスク（`cn-user-api` 契約テスト、`cn-admin-api` auth フロー契約テスト、friend_plus 実ノード E2E）を順次着手する。

## 課題・懸念事項

- `gh act` 実行時に `some refs were not updated` と `pnpm approve-builds` 警告が出るが、いずれも既知でジョブ自体は成功している。

## まとめ

未実装項目だった Admin Console の LLM 連携設定 UI と memberships 一覧/検索 UI を実装し、API 契約・生成型・UI テストまで含めて反映した。必要な Docker/CI 検証（Rust/gh act）も完了した。
