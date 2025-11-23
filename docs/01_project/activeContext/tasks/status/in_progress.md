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

### 2025年11月23日 トピック購読仕様変更実装

- 目標: docs/01_project/activeContext/topic_subscription_spec_review.md に沿って、公開/非公開IDスキーム刷新と購読レジストリ化を進める。
- 状況: バックエンド/CLI/フロントでID正規化・visibility対応・デフォルトトピック切替を実装済み。format-checkは修正完了。
- 次のアクション: 購読レジストリ化とUI文言・導線の整理を継続。
