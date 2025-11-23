[title] 作業中タスク（in_progress）

最終更新日: 2025年11月23日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続せず、P2P（iroh + iroh-gossip + DHT）で完結した体験を優先。
- 全イベントは NIPs 準拠のスキーマに沿って取り扱う。
- Tauri v2 では E2E が困難なため、層別テスト（単体・結合/契約＋スモーク最小限）でカバレッジを確保。

## 現在のタスク

### 2025年11月21日 E2Eオンボーディング遷移調査

- 目標: `/welcome` → `/profile-setup` の遷移で止まる事象を解消し、authStore.generateNewKeypair/loginWithNsec と RootRoute ガードの挙動を確認する。
- 状況: 着手済み（E2E ブリッジ状態確認と UI ロジックの再点検中）。
- 次のアクション: generateNewKeypair 周辺のログ拡充と `/profile-setup` リダイレクト抑止有無の切り分け。

### 2025年11月20日 MVP動作確認シナリオ整理

- 目標: Phase 5 Exit Criteria 全項目（docs/01_project/design_doc.md / phase5_user_flow_summary.md）が実体験として再現できることを確認。
- 状況: 着手（チェックリスト化のみ完了）。次は `./scripts/test-docker.ps1` / Nightly artefact で実施予定。
- 重点確認: オンボーディングとキー管理、プロフィール/プライバシー同期、ホーム/トピック/投稿操作、トレンド・フォロー導線、DM、検索、SyncStatusIndicator、P2P/RelayStatus/CLI 連携、Nightlyジョブ、CIジョブ成功。

### 2025年11月23日 トピック購読仕様変更フォローアップ（不具合対応）

- 目標: docs/01_project/activeContext/topic_subscription_spec_review.md に沿って購読ベースの導線を反映し、pnpm tauri dev で #public を購読済み表示にして投稿確認まで通す。
- 状況: 実装済みと報告していたが、UI が「トピック作成」表記のままで参加できず、参加トピック一覧に public が出ず、n0 接続設定でも Relay/peer が 0 のまま投稿不可。再調査・修正中。
- 次のアクション: Rust 側の購読状態返却とストア初期化を点検・修正し、ローカルで #public 購読＋投稿テストを通す。
