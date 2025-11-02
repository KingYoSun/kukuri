[title] 作業中タスク（in_progress）

最終更新日: 2025年11月03日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続しない（外部インデックスサーバー等の導入時に検討）。
- まず P2P（iroh + iroh-gossip + DHT）で一通りの体験が完結することを最優先。
- kukuri 内部のイベントは全て NIPs 準拠（内部フォーマットは Nostr Event スキーマを準拠・整合）。
- テスト戦略: Tauri v2 では E2E が困難なため、層別テスト（ユニット/結合/契約）＋スモーク最小限に切替。

## 現在のタスク

### Clippy 警告ゼロ体制の復帰

- [x] `domain/entities/event/validation/nip19.rs` の `format!` 文字列を埋め込み式へ置換し、Clippy (`uninlined_format_args`) を解消
- [x] `infrastructure/p2p/dht_integration.rs` の `format!` 文字列を修正し、`AppError::DeserializationError` 周辺のログ表現を統一
- [x] `cargo clippy --all-features -- -D warnings` を `kukuri-tauri/src-tauri` で再実行し、警告ゼロを確認（ログ取得・`phase5_ci_path_audit.md` へ反映）
- [x] `kukuri-cli` 側でも `cargo clippy --all-features -- -D warnings` を実行し、警告ゼロ継続を確認
- [x] Clippy 対応後に `refactoring_plan_2025-08-08_v3.md` の成功指標欄を更新し、再発防止タスクを記録

### ユーザー導線ドキュメント整備

- [ ] UI から到達可能な機能一覧を棚卸しし、`docs/01_project/activeContext/artefacts/` 配下にサマリードキュメントを作成
  - 2025年11月01日: 主要画面（Welcome/Home/Topics/Search/Settings/Debugパネル）とサイドバー導線を確認。未リンク状態の要素（Sidebar「トレンド」「フォロー中」、UserSearchResultsの`/profile/$userId`リンク）を記録済み。
  - 2025年11月01日: `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` を起票し、画面別導線と改善候補を整理。
  - 2025年11月02日: グローバル要素セクションにサイドバー未読バッジ/最終活動更新ロジック、PostComposer の下書き運用、offlineSyncService の同期フローを追記し、TopicPage の最終更新表示（秒→ミリ秒換算）の対処内容を整理。
  - 2025年11月02日: `RootRoute` / `MainLayout` の認証ガードと設定画面のプライバシートグル未連携を追記して導線ギャップを更新。
  - 2025年11月02日: セクション5を追加し、設定画面のプライバシー/プロフィール導線とサイドバー新規投稿導線の具体的な実装案・テスト計画を整理。
  - 2025年11月02日: `usePrivacySettingsStore` と `ProfileEditDialog` を実装し、設定画面からプロフィール編集モーダルを起動できるようにした。
  - 2025年11月02日: `useComposerStore` / `GlobalComposer` を追加し、Home/Sidebar の「投稿する」「新規投稿」ボタンからグローバルコンポーザーを開閉できるように接続。
  - 2025年11月02日: 次ステップ候補を整理 — (1) プライバシー設定のバックエンド連携/API 仕様検討、(2) グローバルコンポーザーのUI/UX改善（トピック選択・ショートカット）の要件定義。
  - 2025年11月03日: プロフィール画像アップロード導線をリモート同期必須に更新し、iroh-blobs 0.96.0 / iroh-docs 0.94.0 を用いた設計案を `phase5_user_flow_inventory.md` に追記。鍵管理モーダルとの連携方針を整理し、ギャップ欄に画像アップロード実装待ちを登録。
  - 2025年11月03日: 共有スコープの Capability 分岐と `StreamEncryptor` 採用、外部 URL フォールバック廃止を決定し、データ構造・処理フロー・セキュリティ節へ反映。
- [x] Tauri コマンド呼び出し状況（フロントエンド `invoke` 探索結果）と未使用 API の整理結果をドキュメントへ反映
  - 2025年11月01日: `TauriApi`・`SecureStorageApi`・`p2pApi`・`nostr`ユーティリティの `invoke` 使用箇所を洗い出し、未使用コマンド候補（例: `add_relay`, `subscribe_to_user`, `join_topic_by_name` など）を整理中。
  - 2025年11月01日: `offlineApi` 系コマンドと `syncEngine` の直接 `invoke`（`get_post_metadata` など）を棚卸しし、`phase5_user_flow_inventory.md` の 3.1/3.2 に追記。
  - 2025年11月02日: `invokeCommand` / `invoke` 呼び出しをスクリプトで抽出し、統合テスト専用コマンド群（`import_key` ほか）と未使用 API を `phase5_user_flow_inventory.md` 3.2/3.3 に反映、併せて 1.6/3.1 の補足内容を更新。
- [ ] `refactoring_plan_2025-08-08_v3.md` のユーザー導線指標チェックボックスを更新し、未達項目のフォロータスクを連携
  - 2025年11月01日: 「UIから到達可能な全機能の文書化完了」を達成済みに更新し、参照ドキュメントと更新日を記録。
  - 2025年11月02日: 指標欄に最新ドキュメント更新（統合テスト専用コマンド整理）と未導線APIの整理状況を追記、Phase2.5セクションへ `TopicPage` 最終更新バグの改善候補を登録。
  - 2025年11月03日: 2.5節を棚卸し結果サマリーへ差し替え、Relay/P2Pステータスカードとプロフィール編集導線の進捗メモを追記。
- [ ] 作成した資料を `phase5_ci_path_audit.md` / `tauri_app_implementation_plan.md` へリンクし、タスク完了後に in_progress.md を更新予定
  - 2025年11月01日: `phase5_ci_path_audit.md` に関連ドキュメントリンクを追加し、`tauri_app_implementation_plan.md` Phase 5 セクションから参照を追記。
  - 2025年11月02日: 上記 2 ドキュメントを最新内容に合わせて再更新し、最終更新日と追記内容にユーザー導線棚卸しの差分を反映。
  - 2025年11月02日: Phase 5 backlog の優先順位を再定義（投稿導線統一→プロフィール編集→プライバシー設定→トレンド/フォロー中→テスト整備）し、`tauri_app_implementation_plan.md` に反映。
  - 2025年11月03日: `phase5_ci_path_audit.md` に Relay/P2P ステータスカードのユニットテスト計画と `profileAvatarSync` 統合テスト計画を追記。`tauri_app_implementation_plan.md` へステータスカード検証・鍵管理モーダル・プロフィール画像リモート同期を Phase 5 優先度として追加。

### プロフィールアバター UI 連携

- [x] フロントの `ProfileForm` など既存 UI から新 `upload_profile_avatar` / `fetch_profile_avatar` API を呼び出す配線と UX チューニングを実施
  - 2025年11月02日: 現行フォームの画像アップロード導線・プレビュー処理を調査し、新コマンドに合わせたストア更新とエラーハンドリング改善の洗い出しを開始。
  - 2025年11月02日: `ProfileForm` に Tauri ダイアログ経由の画像選択・プレビュー・バリデーションを実装し、`ProfileSetup` / `ProfileEditDialog` から `upload_profile_avatar` / `fetch_profile_avatar` を呼び出すよう接続。アップロード後は `authStore` にメタデータ（`avatar`）とデータURLを反映し、Vitest の関連ユニットテストを更新済み。
  - 2025年11月02日: `authStore` がログイン／初期化／アカウント切替時に `fetch_profile_avatar` を呼び出して `currentUser.avatar` と `picture` を同期し、`AccountSwitcher` / `ReplyForm` / `QuoteForm` でデフォルトアバターにフォールバックするようリファレンスを更新。ユニットテストにリモート取得ケースを追加し、全体テストを実行済み。
  - 2025年11月03日: `ProfileEditDialog` のユニットテストを追加し、アップロード・フェッチ・エラー処理パスを検証。Vitest から `@tauri-apps/api/dialog` / `fs` を解決できるよう専用モックと `vitest.config.ts` のエイリアスを追加した上でテストを実行。
- [x] 全体の UI で新アバターメタデータを参照している箇所を洗い、必要に応じて default_avatar フォールバックを適用しきれているか確認
  - 2025年11月02日: `resolveUserAvatarSrc` ユーティリティを追加し、ReplyForm・QuoteForm・PostCard・AccountSwitcher・UserSearchResults のアバター参照を共通化。UserSearchResults 用のフォールバック検証テストを新設し、既存フォーム/ポスト系ユニットテストと併せて `pnpm vitest run ...` で成功を確認。
