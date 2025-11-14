# Tauriアプリケーション実装計画

**作成日**: 2025年07月28日  
**最終更新**: 2025年11月13日  
**目的**: 体験設計に基づいた具体的な実装タスクとスケジュール（オフラインファースト対応）

## MVP残タスクサマリー（2025年11月10日更新）

| 領域 | 目的 | 必須タスク | 状態/担当 | 参照 |
| --- | --- | --- | --- | --- |
| トレンド/フォロー導線 | `/trending` `/following` Summary Panel を安定化し、Docker/Nightly と整合 | `trending_metrics_job` の24h集計 + `list_trending_*` の `generated_at` ミリ秒保証、`scripts/test-docker.{sh,ps1}` `--scenario trending-feed` の fixture 固定、`TrendingSummaryPanel` / `FollowingSummaryPanel` のテレメトリ表示更新 | ✅ 2025年11月15日: Summary Panel の更新時刻・ラグ・DM 会話ラベルを UI に追加し、`scripts/test-docker.{sh,ps1}` `ts --scenario trending-feed` で `test-results/trending-feed/{reports,prometheus,metrics}` と `tmp/logs/trending_metrics_job_stage4_<timestamp>.log` を自動採取。`nightly.yml` に `trending-metrics-json` artefact を追加し、Runbook/CI から参照可能になった。 | `phase5_user_flow_summary.md` (MVP Exit UX行), `phase5_user_flow_inventory.md` 5.7 |
| DM & 通知 | `DirectMessageInbox` の可搬性と未読表示を完成 | 会話リストの仮想スクロール、候補補完・検索、未読共有（`mark_direct_message_conversation_read` multi-device）、SR-only 告知のUIテストを追加 | ⏳ UI/IPC は実装済。既読共有と `Header.test.tsx` / `DirectMessageDialog.test.tsx` を `pnpm vitest` で再確認できていない。`phase5_user_flow_inventory.md` 5.4 に多端末要件を追記予定。 | `phase5_user_flow_inventory.md` 5.4, `phase5_user_flow_summary.md` |
| プロフィール/設定 | ProfileSetup/Settings モーダルを共通化しプライバシー設定をバックエンドへ伝播 | `ProfileForm` 抽出、プライバシーstore (`usePrivacySettingsStore`) 永続化、`update_nostr_metadata` の権限拡張、設定モーダルの保存フローとテスト | ✅ Stage4（2025年11月12日）: Stage3 での Doc/Blob + privacy 連携に続き、Service Worker (`profileAvatarSyncSW.ts`) / BroadcastChannel / `cache_metadata` TTL 30 分 / `offlineApi.addToSyncQueue` ログを実装し、`scripts/test-docker.{sh,ps1} ts --scenario profile-avatar-sync --service-worker` / `./scripts/test-docker.ps1 rust -Test profile_avatar_sync` / `pnpm vitest run ...useProfileAvatarSync.test.tsx workers/profileAvatarSyncWorker.test.ts` を再実行。`tmp/logs/profile_avatar_sync_stage4_<timestamp>.log` を `nightly.profile-avatar-sync` artefact へ保存し、Runbook Chapter4 / `phase5_ci_path_audit.md` / `phase5_dependency_inventory_template.md` に Service Worker 手順を反映。 | `phase5_user_flow_inventory.md` 5.1 |
| ユーザー検索 | レートリミット/ページネーション/状態遷移をUIで表現 | `search_users` API 拡張（cursor/sort/allow_incomplete）、`UserSearchResults` 状態マシン、レートリミットUI、`useUserSearchQuery` テスト | ⏳ 2025年11月10日に `allow_incomplete` フォールバック・補助検索検知・SearchBar 警告スタイルを実装し、`npx pnpm vitest run src/tests/unit/hooks/useUserSearchQuery.test.tsx src/tests/unit/components/search/UserSearchResults.test.tsx`（`tmp/logs/vitest_user_search_allow_incomplete_20251110132951.log`）と Docker `./scripts/test-docker.sh ts --scenario user-search-pagination --no-build`（`tmp/logs/user_search_pagination_20251110-142854.log`）で回帰を取得。残タスクは Nightly ジョブへのシナリオ組み込みと `test-results/user-search-pagination/*.json` の成果物化。 | `phase5_user_flow_inventory.md` 5.4 |
| Offline sync_queue | オフライン操作の蓄積と競合解決UIを提供 | `sync_queue`/`offline_actions`/`cache_metadata` テーブル、`sync_offline_actions` Tauriコマンド、`useSyncManager` conflict banner/Retry、Service Worker のバックグラウンド同期 | ✅ Stage4（2025年11月11日）: `cache_metadata` に Doc/Blob 情報（`doc_version`/`blob_hash`/`payload_bytes`）を永続化し、`SyncStatusIndicator` に Doc/Blob 競合バナーとサマリーを追加。`serviceWorker/offlineSyncWorker.ts` + BroadcastChannel で自動再送ジョブを整備し、`./scripts/test-docker.{sh,ps1} ts --scenario offline-sync --no-build` と `npx vitest run src/tests/unit/hooks/useSyncManager.test.tsx src/tests/unit/components/SyncStatusIndicator.test.tsx src/tests/unit/components/OfflineIndicator.test.tsx` を Runbook/CI に登録。 | 本書 Phase4, `phase5_user_flow_inventory.md` 5.5/5.11 |

> **クロスウォーク**: 上表は `phase5_user_flow_summary.md` の「MVP Exit Checklist（UX/体験行）」と連動。CI / Docker / Rust テストの実行計画は `phase5_ci_path_audit.md` に記録し、テストが未実行の場合はログファイル（`tmp/logs/*.log`）へのリンクを本書に記載する。

## Phase 1: 認証フローの修正 ✓ 完了

### 1.1 ウェルカム画面の実装 ✓ 完了

#### 完了したタスク
1. `src/routes/welcome.tsx` の作成 ✓
2. `src/components/auth/WelcomeScreen.tsx` の実装 ✓
   - アプリケーションの説明
   - 新規アカウント作成ボタン
   - 既存アカウントでログインボタン
   - テスト作成（5件）
3. `src/components/auth/LoginForm.tsx` の作成 ✓
   - nsec入力フォーム
   - バリデーション（nsec形式の秘密鍵検証）
   - エラーハンドリング
   - セキュア保存オプション
   - テスト作成（8件）
4. `src/components/auth/ProfileSetup.tsx` の作成 ✓
   - 名前、自己紹介の入力
   - アバター設定（イニシャル生成）
   - スキップ機能
   - テスト作成（9件）

#### 実装詳細
```typescript
// WelcomeScreen.tsx
export function WelcomeScreen() {
  const navigate = useNavigate();
  const { generateNewKeypair } = useAuthStore();
  
  const handleCreateAccount = async () => {
    try {
      await generateNewKeypair();
      navigate({ to: '/profile-setup' });
    } catch (error) {
      toast.error('アカウントの作成に失敗しました');
    }
  };
  
  return (
    <div className="flex flex-col items-center justify-center min-h-screen">
      <h1>kukuriへようこそ</h1>
      <p>分散型トピック中心ソーシャルアプリ</p>
      <Button onClick={handleCreateAccount}>新規アカウント作成</Button>
      <Button variant="outline" onClick={() => navigate({ to: '/login' })}>
        既存アカウントでログイン
      </Button>
    </div>
  );
}
```

### 1.2 認証状態の適切な管理 ✓ 完了

#### 完了したタスク
1. `authStore.ts` の修正 ✓
   - 初期状態を `isAuthenticated: false` に固定
   - 起動時に鍵の有効性を確認するロジック追加
   - initializeメソッドの実装（自動ログイン）
   - 複数アカウント管理機能の追加
   - テスト作成（initializeメソッド4件、統合テスト5件）
2. `src/hooks/useAuth.ts` の改善 ✓
   - 初期化ロジックの実装
   - 認証ガードの実装
3. `src/routes/__root.tsx` の修正 ✓
   - 認証状態によるリダイレクト
   - 認証ガードテストの作成

#### 実装の特徴
- **セキュアな鍵管理**: プラットフォーム固有のセキュアストレージを使用し、秘密鍵をメモリに保持しない
- **複数アカウント対応**: 複数のアカウントを安全に管理し、簡単に切り替え可能
- **自動ログイン**: 起動時に前回使用したアカウントで自動的にログイン
- **包括的なテスト**: 全37件のテストによる品質保証

### 1.3 ログアウト機能の修正 ✓ 完了

#### 完了したタスク
1. Headerコンポーネントにユーザーメニュー追加 ✓
   - プロフィール表示
   - 設定メニュー
   - ログアウトボタン
2. ログアウト処理の実装 ✓
   - 確認ダイアログ
   - 状態のクリア
   - ウェルカム画面へのリダイレクト
3. AccountSwitcherコンポーネントの実装 ✓
   - 複数アカウント切り替えUI
   - 現在のアカウント表示
   - アカウント追加・削除機能

### 1.4 セキュアストレージ実装 ✓ 完了

#### 完了したタスク
1. Rustバックエンドのセキュアストレージ実装 ✓
   - keyring crateによるプラットフォーム固有セキュアストレージアクセス
   - macOS Keychain、Windows Credential Manager、Linux Secret Service対応
   - 秘密鍵の個別暗号化保存（npubごと）
   - アカウントメタデータ管理（公開情報のみ）
2. Tauriコマンドの実装 ✓
   - add_account - アカウント追加とセキュア保存
   - list_accounts - 保存済みアカウント一覧
   - switch_account - アカウント切り替え
   - remove_account - アカウント削除
   - get_current_account - 現在のアカウント取得（自動ログイン用）
   - secure_login - セキュアストレージからのログイン
3. フロントエンドの複数アカウント対応 ✓
   - SecureStorageApi TypeScriptラッパー実装
   - authStoreの拡張（複数アカウント管理機能）
   - 自動ログイン機能（起動時の自動認証）
4. 包括的なテストの作成 ✓
   - Rustバックエンドテスト（8件）
   - フロントエンドAPIテスト（6テストスイート）
   - 統合テスト（3テストスイート）

## Phase 2: データ連携の確立 ✓ 完了

### 2.1 ホームページの実データ表示 ✓ 完了

#### 完了したタスク
1. 投稿の実データ表示 ✓
   - `src/pages/Home.tsx` の修正
   - `src/hooks/usePosts.ts` の改善（タイムライン用投稿取得、30秒ごとの自動更新）
   - `src/components/posts/PostCard.tsx` の作成（いいね機能、日本語相対時刻表示）
   - PostCardコンポーネントのテスト作成（9件）
2. トピック一覧の実データ表示 ✓
   - `src/routes/topics.tsx` の作成（トピック探索ページ、リアルタイム検索機能）
   - `src/components/topics/TopicCard.tsx` の作成（参加/退出ボタン、統計情報表示）
   - `src/hooks/useTopics.ts` の実装（TauriAPI連携、CRUD操作ミューテーション）
   - TopicCardコンポーネントのテスト作成（9件）
   - Topics.tsxページのテスト作成（12件）
   - useTopicsフックのテスト作成（7件）
3. 既存テストの修正（QueryClientProvider対応）

#### 実装詳細
```typescript
// usePosts.ts（実装済み）
export function useTimelinePosts() {
  return useQuery({
    queryKey: ['timeline'],
    queryFn: async () => {
      const posts = await TauriApi.getPosts({ limit: 50 });
      return posts;
    },
    refetchInterval: 30000, // 30秒ごとに更新
  });
}
```

### 2.2 トピック機能の実装 ✓ 完了

#### 完了したタスク
1. 投稿作成機能 ✓
   - `src/components/PostComposer.tsx` の実装（投稿作成フォーム）
   - `src/components/TopicSelector.tsx` の実装（トピック選択コンポーネント）
   - Home画面とトピック詳細画面への統合
   - PostComposerのテスト作成（11件）
   - TopicSelectorのテスト作成（12件）
2. トピック管理機能 ✓
   - `src/components/topics/TopicFormModal.tsx` の実装（作成/編集フォーム）
   - `src/components/topics/TopicDeleteDialog.tsx` の実装（削除確認ダイアログ）
   - トピック一覧・詳細ページへの統合
   - react-hook-formを使用したフォームバリデーション
3. P2P連携実装 ✓
   - トピック参加時のP2Pトピック自動参加
   - トピック離脱時のP2Pトピック自動離脱
   - TauriAPIとP2P APIの完全統合

### 2.3 リアルタイム更新の実装 ✓ 完了

#### 完了したタスク
1. Nostrイベントのリアルタイム処理 ✓
   - `src/hooks/useNostrEvents.ts` の作成
   - Tauriイベントリスナーの設定（nostr://event）
   - イベント受信時の自動ストア更新
   - 新規投稿、トピック更新、いいねの即座反映
2. P2Pイベントのリアルタイム処理 ✓
   - `src/hooks/useP2PEventListener.ts` の改善
   - P2Pメッセージの即座反映
   - トピック参加/離脱の自動更新
3. データ同期機能 ✓
   - `src/hooks/useDataSync.ts` の実装
   - 定期的なデータ更新（30秒間隔）
   - イベント駆動とポーリングのハイブリッド方式
4. UI表示機能 ✓
   - `src/components/RealtimeIndicator.tsx` の実装
   - Nostr/P2P接続状態の可視化
   - 最新データ受信時刻の表示
5. 包括的なテスト ✓
   - useNostrEventsのテスト（10件）
   - useDataSyncのテスト（8件）
   - RealtimeIndicatorのテスト（6件）
   - 合計24件の新規テスト追加

### 2.4 追加機能 ✓ 完了

#### 完了したタスク
1. 手動P2P接続機能 ✓
   - `src/components/p2p/PeerConnectionPanel.tsx` の作成 ✓
     - 自分のピアアドレス表示とコピー機能
     - 手動ピアアドレス入力フォーム
     - 接続処理（バリデーション、エラーハンドリング）
     - 接続履歴管理（LocalStorage使用）
   - 設定ページへの統合 ✓
   - 包括的なテストの作成 ✓
2. リアクション機能の実装 ✓
   - 返信機能（ReplyForm） ✓
     - NIP-10準拠の返信タグ実装
     - リアルタイム更新対応
   - 引用機能（QuoteForm） ✓
     - NIP-10準拠の引用タグ実装
     - nostr:プロトコルリンク生成
   - PostCardへの統合 ✓
3. 検索機能の基本実装 ✓
   - SearchBarコンポーネント（デバウンス付き） ✓
   - PostSearchResults（投稿検索） ✓
   - TopicSearchResults（トピック検索） ✓
   - UserSearchResults（ユーザー検索） ✓
   - 検索ページ（/search）の実装 ✓
   - ヘッダーへの検索バー統合 ✓

#### 実装詳細
```typescript
// PeerConnectionPanel.tsx
export function PeerConnectionPanel() {
  const { nodeAddress, connectToPeer } = useP2PStore();
  const [peerAddress, setPeerAddress] = useState('');
  const [isConnecting, setIsConnecting] = useState(false);
  
  const handleConnect = async () => {
    if (!peerAddress.trim()) return;
    
    setIsConnecting(true);
    try {
      await p2pApi.connectToPeer(peerAddress);
      toast.success('ピアに接続しました');
      setPeerAddress('');
    } catch (error) {
      toast.error('接続に失敗しました');
    } finally {
      setIsConnecting(false);
    }
  };
  
  return (
    <Card>
      <CardHeader>
        <CardTitle>P2P接続設定</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          <div>
            <Label>あなたのピアアドレス</Label>
            <div className="flex gap-2">
              <Input value={nodeAddress} readOnly />
              <Button onClick={() => copyToClipboard(nodeAddress)}>
                コピー
              </Button>
            </div>
          </div>
          
          <div>
            <Label>ピアに接続</Label>
            <div className="flex gap-2">
              <Input
                value={peerAddress}
                onChange={(e) => setPeerAddress(e.target.value)}
                placeholder="/ip4/192.168.1.100/tcp/4001/p2p/QmXXX..."
              />
              <Button 
                onClick={handleConnect}
                disabled={isConnecting || !peerAddress.trim()}
              >
                {接続}
              </Button>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
```

## Phase 3: 主要機能の実装

> **ステータス（2025年11月09日更新）**: Phase 3.1〜3.2 は UI/ストア/テストまで完了し、残タスクは Phase5 artefacts（Inventory 5.9〜5.10 / Sec.6）に沿って絞り込んだ。以下では「完了済み」「MVP残」「MVP後」の3分類で管理する。

### ✅ 完了済み（2025年11月09日時点）

#### 3.1 トピック参加・離脱機能の改善 ✓ 完了

#### 完了したタスク
1. P2P接続の自動化 ✓
   - トピック参加時のP2Pトピック自動参加の最適化
   - topicStoreのjoinTopic/leaveTopicメソッドを非同期化
   - P2P接続とNostrサブスクリプションの統合
   - Nostrサブスクリプション開始タイミングの調整（P2P接続後500ms遅延）
2. UIの状態管理改善 ✓
   - 参加中トピックの一覧表示強化（サイドバー）
   - 最終活動時刻でソートされた一覧表示
   - P2Pメッセージの最終活動時刻を考慮
   - ボタンの状態変更のリアルタイム反映
3. 包括的なテストの追加 ✓
   - topicStore.test.ts（8テストケース）
   - Sidebar.test.tsx（7テストケース）
   - TopicCard.test.tsxの更新（5テストケース追加）

#### 3.2 新規投稿機能の拡張 ✓ 部分完了

#### 完了したタスク
1. リッチテキストエディタの実装 ✓
   - マークダウンサポート（@uiw/react-md-editor@4.0.8）
   - MarkdownEditorコンポーネントの作成
   - 画像アップロード機能（ドラッグ&ドロップ対応）
   - メディア埋め込み（画像、動画、YouTube、Vimeo、Twitter/X）
   - プレビュー機能（MarkdownPreviewコンポーネント）
2. 投稿オプションの追加 ✓
   - 予約投稿機能のUI実装（PostScheduler、react-day-picker）
   - 下書き保存機能の実装（draftStore、DraftManager）
   - 自動保存機能（2秒デバウンス）
3. PostComposerコンポーネントの更新 ✓
   - シンプル/Markdownモードのタブ切り替え
   - 全新機能の統合
4. 包括的なテストの追加 ✓
   - 各コンポーネントのテスト作成
   - 17個のテストエラーを全て修正
   - テスト総数: 517個全て成功

#### MVP完成後の改善として保留
- 予約投稿のバックエンド実装
  - 予約投稿の保存機能（SQLite）
  - 予約投稿の実行スケジューラー
  - Tauriコマンドの実装

### 🟡 MVP残タスク（Phase3）
- **トピック作成ショートカット**（Inventory 5.9）: ✅ 2025年11月10日 — `TopicFormModal` に `create-from-composer` モードを追加し、`PostComposer` / `Sidebar` / `TopicSelector` が新規トピックの作成 → 自動参加 → コンポーザー再開を一貫導線で実装。単体テストは `TopicSelector.test.tsx` / `PostComposer.test.tsx` / `Sidebar.test.tsx` を追加。残課題: オフライン作成キューと Runbook 整備。
- **投稿削除後のキャッシュ整合性**（Inventory 5.10）: ✅ 2025年11月10日 — `useDeletePost` + `cacheUtils.invalidatePostCaches` でタイムライン/トレンド/フォロー中の React Query キャッシュと `topicStore.updateTopicPostCount` を即時更新。`PostCard` は新フックへ移行し、`postStore.deletePostRemote` もトピック件数を更新する。Rust `post_delete_flow` は次スプリントで追加予定。
- **プロフィール/設定 Stage3**（Inventory 5.1 + Sec.6）: ✅ 2025年11月10日 — `ProfileEditDialog` / `ProfileSetup` が `update_privacy_settings` → `upload_profile_avatar` をシリアライズし、成功後に `authStore.updateUser` と `useProfileAvatarSync.syncNow({ force: true })` で Doc バージョンを即時反映。`profile_avatar_sync` コマンドは `known_doc_version` で差分転送し、`__root.tsx` の常駐フックが 5 分間隔で同期。`pnpm vitest run src/tests/unit/components/settings/ProfileEditDialog.test.tsx src/tests/unit/components/auth/ProfileSetup.test.tsx src/tests/unit/hooks/useProfileAvatarSync.test.tsx`、`./scripts/test-docker.ps1 ts -Scenario profile-avatar-sync`、`./scripts/test-docker.ps1 rust -Test profile_avatar_sync` を実行し、`docs/03_implementation/p2p_mainline_runbook.md` Chapter4 と `phase5_ci_path_audit.md` にログ/手順 (`tmp/logs/profile_avatar_sync_*.log`) を追加した。

### ⏭️ MVP後に回す項目（Phase3）

#### 3.3 その他のリアクション機能

> **メモ（2025年11月08日）**: ブースト/ブックマーク/カスタムリアクションは MVP 後に実装する方針。仕様とテスト計画は維持しつつ、Phase 5 完了までは優先度を下げる。

#### タスク
1. ブースト機能（リポスト）の実装
   - NostrのNIP-18準拠のリポストイベント
   - UI: ブーストボタンとカウント表示
   - ブースト済み状態の管理
2. ブックマーク機能の実装
   - ローカルストレージでのブックマーク管理
   - ブックマーク一覧ページ
   - UI: ブックマークボタンと状態表示
3. カスタムリアクション絵文字の対応
   - NIP-25準拠のリアクションイベント
   - 絵文字ピッカーの実装
   - リアクション一覧の表示

## Phase 4: オフラインファースト機能の実装

> **ステータス（2025年11月09日）**: Stage1〜3（DB/OfflineStore/Syncエンジン/キュー履歴 UI）は完了し、Runbook・UI 仕上げが残課題。Phase4 も Phase3 と同様に「完了済み」「MVP残」「MVP後」に整理する。

### ✅ 完了済み（Stage1〜3 + 永続化テンプレート）

#### 4.1 ローカルファーストデータ管理

> **ステータス**: ✅ `sync_queue` / `offline_actions` / `cache_metadata` マイグレーション（20250915〜20251107）と `offline_store` 実装が完了。`offline_handler::list_sync_queue_items` 追加に伴い `.sqlx` データを再生成し、`./scripts/test-docker.ps1 rust -NoBuild` で `cargo test offline_handler::tests::add_to_sync_queue_records_metadata_entry` を完走。

#### タスク
1. ローカルDBスキーマの拡張
   - 同期ステータステーブル（sync_queue）
   - オフラインアクションログ（offline_actions）
   - キャッシュメタデータテーブル（cache_metadata）
2. オフラインストレージAPIの実装
   - save_offline_action - オフラインアクションの保存
   - get_offline_actions - 保存済みアクションの取得
   - sync_offline_actions - オフラインアクションの同期
   - get_cache_status - キャッシュ状態の取得
3. オフラインストアの実装
   - offlineStore.ts - オフライン状態管理
   - 接続状態監視（navigator.onLine）
   - オフラインキュー管理

#### 4.2 楽観的UI更新の実装

> **ステータス**: ✅ `useSyncManager` / `offlineStore` が `add_to_sync_queue`・`update_sync_status`・`update_cache_metadata` を呼び出すフローまで導線化。`pnpm vitest src/tests/unit/stores/offlineStore.test.ts src/tests/unit/hooks/useSyncManager.test.tsx` を `scripts/test-docker.{sh,ps1} ts` 経由で監視し、`useDeletePost` 実装前の共通ロールバックを維持。

#### タスク
1. 操作のローカル実行
   - 投稿作成 - 即座にUIへ反映、背景で同期
   - いいね/リアクション - ローカルステート即座更新
   - トピック参加/離脱 - UI即座反映、同期待ちキュー追加
2. ロールバック機能
   - 同期失敗時のローカル変更の巻き戻し
   - エラー通知と再試行オプション
3. Tanstack Queryの最適化
   - optimistic updatesの設定
   - キャッシュ無効化戦略
   - 背景再フェッチの制御

#### 4.3 同期と競合解決

> **ステータス**: ✅ Stage3 で `list_sync_queue_items` / Queue 履歴 UI を提供し、`SyncStatusIndicator` / `OfflineIndicator` のヒストリー表示を `pnpm vitest src/tests/unit/components/SyncStatusIndicator.test.tsx src/tests/unit/components/OfflineIndicator.test.tsx` で回帰テスト。`phase5_ci_path_audit.md` にテスト ID を登録済み。

#### タスク
1. 同期エンジンの実装
   - 差分同期アルゴリズム
   - タイムスタンプベースの競合検出
   - 並列同期処理（トピック別）
2. 競合解決戦略
   - Last-Write-Wins (LWW) ベースライン
   - カスタムマージルール（トピック参加状態など）
   - ユーザーへの競合通知UI
3. 同期ステータスの管理
   - 同期進捗の表示
   - 同期エラーのハンドリング
   - 手動同期トリガー

#### 4.5 Zustand 永続化テンプレート整備 ✓ 完了

##### 実装概要（2025年10月20日更新）
- `src/stores/utils/persistHelpers.ts` に `withPersist` / `createPersistConfig` / `createMapAwareStorage` を実装し、すべてのストアで同一テンプレートを利用できるようにした。
- `src/stores/config/persist.ts` でキー名 (`persistKeys`)・partialize 設定・Map 対応ストレージを集中管理。新しいストアを追加する際はここで設定を定義し、ストア側では `withPersist(initializer, createXxxPersistConfig())` を呼び出すだけで済む。
- Map フィールドを扱うストア（`offlineStore`, `p2pStore`, `topicStore` など）は `createMapAwareStorage` を使用することで、従来の手動シリアライズ処理を排除。
- テストでは `src/stores/utils/testHelpers.ts` に追加した `setupPersistMock` を利用し、`localStorage`/`sessionStorage` の差し替えとリセットを共通化。

##### 移行手順と注意点
1. 既存ストアをテンプレートへ移行する際は、旧 `persist` 設定の `name`（ローカルストレージキー）を維持し、過去データが失われないことを確認する。  
2. キー名を変更する場合はマイグレーションロジック（旧キーからの読み込み → 新フォーマットへの変換）をストア初期化時に追加し、リリースノートへ追記する。  
3. `setupPersistMock` をテストの `beforeEach` で呼び出し、永続化データの汚染を回避する。ストア単体テストでは新テンプレートが正しく partialize を反映しているかを検証する。
4. Phase 4 のリファクタリングに伴い、`.sqlx` 再生成や DefaultTopicsRegistry の更新などバックエンド側で大きな変更を行う場合は、本テンプレートのキー互換性チェックを合わせて実施する。

### 🟡 MVP残タスク（Phase4）

#### 4.4 オフラインUI/UX

> **優先度**: Stage4（MVP Exit Checklist #3）は 2025年11月11日にクローズ済み。以降はヘッダー/サマリ導線と Runbook/artefact の突合、および `sync_engine` 再送ログ・Service Worker ジョブの監視強化を本節で扱う。

2025年11月11日: Stage4 初期要件（Doc/Blob `cache_metadata` 拡張、競合バナー、Service Worker、Docker `offline-sync` シナリオ）を完了。`cache_types` から `doc_version` / `blob_hash` / `payload_bytes` を提供し、`SyncStatusIndicator` に Doc/Blob サマリーと競合バナーを実装。`npx vitest run src/tests/unit/components/SyncStatusIndicator.test.tsx src/tests/unit/hooks/useSyncManager.test.tsx src/tests/unit/components/OfflineIndicator.test.tsx` と `./scripts/test-docker.{sh,ps1} ts --scenario offline-sync --no-build` を Runbook/CI に登録し、`tmp/logs/sync_status_indicator_stage4_<timestamp>.log` を採取して Chapter5 へリンクした。

#### タスク
1. オフラインインジケーター
   - ヘッダーにオフライン状態表示
   - 同期待ちアイテム数の表示
   - 最後の同期時刻表示
2. オフライン用UI調整
   - オフライン中の投稿に「同期待ち」バッジ
   - オンライン時の同期アニメーション
   - オフラインモード専用のトースト通知
3. Service Workerの活用
   - バックグラウンド同期の実装
   - キャッシュ管理
   - オフラインリソースの事前ロード

### ⏭️ MVP後プラン（Phase4）
- **Service Worker 拡張**: 4.4 の「バックグラウンド同期/キャッシュ管理/リソース事前ロード」は MVP 後に段階導入し、`docs/03_implementation/pwa_offline_runbook.md`（新規予定）で配布する。第一段階ではヘッダーUIと Runbook 整備を優先し、Push/バックグラウンドフェッチはベータ後に実験する。
- **オフライン添付ファイル & Queue 圧縮**: `OfflineActionType::CREATE_TOPIC` 等へ大型ペイロードを積むシナリオは Post-MVP backlog に移動。`refactoring_plan_2025-08-08_v3.md` Phase5（OfflineService 行）でスコープ管理する。

## Phase 5: アーキテクチャ再構成（準備中）

依存関係棚卸し（2025年10月23日更新, `docs/01_project/activeContext/artefacts/phase5_dependency_inventory_template.md`）で抽出したハイリスク領域に対応するためのメモ。
- 2025年11月01日: UI 導線と `invoke` 利用状況は `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` を参照し、Phase 5 タスクのインプットとする。
- 2025年11月02日: 同ドキュメントに RootRoute/MainLayout の遷移制御と設定>プライバシーの未接続トグルを追記。導線ギャップ解消タスク（新規投稿ショートカット/プロフィール編集/プライバシー設定連携）を Phase 5 backlog に取り込む。
- 2025年11月03日: サイドバーの `RelayStatus`/`P2PStatus` 監視内容とグローバルコンポーザー導線、プロフィール編集モーダルの反映手順を同ドキュメントに追記。セクション5.5でポーリング失敗時の挙動・リトライ UI・テスト計画を整理し、`phase5_ci_path_audit.md` のステータスカード検証項目と連携。追加ギャップ（プロフィール画像アップロード導線など）をバックログへ登録。
- 2025年11月03日: `pnpm vitest run src/tests/unit/components/posts/PostCard.test.tsx src/tests/unit/stores/postStore.test.ts` を実行し、削除メニューとオフライン削除キューの回帰を確認（`postStore.deletePostRemote` がオフライン時に `TauriApi.deletePost` を呼ばないことを検証）。
- 2025年11月03日: Relay/P2P ステータスカードのバックオフ実装を完了。`npx vitest run …` でコンポーネント/ストア/フックのユニットテストを実行し、`cargo test`（`kukuri-tauri/src-tauri`・`kukuri-cli`）で `connection_status` / `peers` 拡張後のフォールバック動作を確認。Runbook 9章および CI パス監査に反映。
- 2025年11月06日: `phase5_user_flow_inventory.md` 5.7 節を更新し、`list_trending_topics`/`list_trending_posts`/`list_following_feed` のデータ要件（limit/per_topic/cursor/キャッシュポリシー）とテスト計画を明記。`generated_at` をミリ秒エポックで返す必要性、Summary Panel・`trending_metrics_job`・Docker シナリオを backlog として追記し、Summary/CI 計画と同期。
- 2025年11月06日: ユーザー検索導線のページネーション/状態遷移/エラー UI/入力バリデーション方針を `phase5_user_flow_inventory.md` 5.8 節と `phase5_user_flow_summary.md`、`docs/03_implementation/error_handling_guidelines.md` に追記し、`SearchErrorState` コンポーネントと React Query デバウンス/AbortController 戦略を整理。`tauri_app_implementation_plan.md` の優先度項目を同方針と同期。
- 2025年11月06日: Inventory 5.9 にホーム/サイドバーからのトピック作成導線を追加。TopicSelector ショートカット／`TopicFormModal` mode=`create-from-composer`／`createAndJoinTopic` ヘルパー／`OfflineActionType::CREATE_TOPIC` を定義し、Summary・CI・依存関係ドキュメントへ反映。
- 2025年11月12日: Stage4（`OfflineActionType::CREATE_TOPIC`）を実装し、`TopicService::enqueue_topic_creation` / `topics_pending` / `PendingTopicRepository` を追加。Tauri には `enqueueTopicCreation` / `listPendingTopics` コマンドを、フロントには `topicStore.pendingTopics`・`TopicSelector` の「保留中」グループ・`TopicFormModal` のオフライン経路（`watchPendingTopic` → `resolvePendingTopic`）を導入。`Input` を `forwardRef` 化して Radix ref 警告を解消し、`topicCreateOffline` シナリオを含む `npx pnpm vitest run ... | Tee-Object -FilePath ../tmp/logs/topic_create_host_20251112-231141.log` と `./scripts/test-docker.ps1 ts -Scenario topic-create`（`tmp/logs/topic_create_20251112-231334.log`, `test-results/topic-create/20251112-231334-*.json`）で QA を実施。Runbook Chapter5 / `phase5_ci_path_audit.md` にも採取パスを反映済み。
- 2025年11月06日: Inventory 5.10 に投稿削除後の React Query キャッシュ整合性（`useDeletePost` ミューテーション、トレンド/フォローキャッシュ更新、Docker シナリオ `post-delete-cache`、Rust 統合テスト）と `error_handling_guidelines.md` のトーストキー更新を追加。`phase5_ci_path_audit.md` のテスト ID と Nightly 実行計画を同期。
- 2025年11月06日: Inventory 5.6 と `phase5_user_flow_summary.md` 2章に Kind4 DM 未読バッジ・再送導線・DirectMessageDialog 改修を反映し、`phase5_ci_path_audit.md` の `test:unit` 更新（DirectMessageDialog / Summary Panel / UserSearchResults）と整合を取った。
- 2025年11月06日: `useOfflineStore.refreshCacheMetadata` / `useSyncManager.persistSyncStatuses` を実装し、同期完了時に `update_cache_metadata`・`update_sync_status` を自動更新。`SyncStatusIndicator` の最終同期時刻がバックエンドのスナップショットに追従することを確認。
- 2025年11月07日: Inventory 5.6.1/5.6.2 と `phase5_user_flow_summary.md` 2章/Quick View を更新し、`/profile/$userId` の DM 呼び出し導線・フォロー/フォロワー一覧（ソート/検索/件数表示）・`profile.$userId.test.tsx` の Nightly 追加、Rust (`kukuri-cli`) 検証ログを記録。フォロー導線 backlog を再整理し、優先タスク（プロフィール導線/DM 再送/フォロー一覧拡張）を Phase 5 リストに反映。
- 2025年11月07日: Inventory 5.11 に `SyncStatusIndicator` / `OfflineIndicator` / `useSyncManager` / `offlineStore` / `offlineApi.update_cache_metadata` / `update_sync_status` / `get_cache_status` / `add_to_sync_queue` のフローとギャップ・テスト計画を追記し、`phase5_user_flow_summary.md` のグローバル要素および `phase5_ci_path_audit.md` のテーブル（SyncStatusIndicator/useSyncManager テスト行）へリンク。`pnpm vitest run src/tests/unit/hooks/useSyncManager.test.tsx src/tests/unit/components/SyncStatusIndicator.test.tsx` を Nightly へ追加したログを保存。
- 2025年11月09日: `list_sync_queue_items` / 再送キュー履歴 UI を実装し、Inventory 5.11 / Summary / CI path audit を更新。`npx vitest run src/tests/unit/hooks/useSyncManager.test.tsx src/tests/unit/components/SyncStatusIndicator.test.tsx src/tests/unit/components/OfflineIndicator.test.tsx`、`cargo test`（Docker 経由）を実行し、Phase4.3 Stage3 の要件を満たした。
- 2025年11月08日: Inventory 5.12 / Summary 2章・Quick View / `refactoring_plan_2025-08-08_v3.md` を更新し、`DirectMessageInbox` / `useDirectMessageBadge` / `MessageCircle` + `Plus` ボタン / Summary Panel CTA から DM を開始できる導線と `Header.test.tsx`・`useDirectMessageBadge.test.tsx`・`TrendingSummaryPanel.test.tsx`・`FollowingSummaryPanel.test.tsx` のカバレッジを整理。残課題（会話一覧 API、宛先検索/補完、未読永続化、会話リストの仮想スクロール）を Phase 5 backlog へ追加し、`direct_message_service` 次フェーズの要件として追跡。
- 2025年11月08日: `direct_message_conversations` テーブルと `list_direct_message_conversations` / `mark_direct_message_conversation_read` コマンドを実装し、DirectMessageInbox の会話一覧・未読数を永続化。`DirectMessageDialog` は既読更新時に Tauri API を呼び、`useDirectMessageBootstrap` がログイン直後に Inbox をハイドレートする構成へ刷新。`pnpm vitest run src/tests/unit/components/directMessages/DirectMessageDialog.test.tsx src/tests/unit/components/layout/Header.test.tsx`、`./scripts/test-docker.ps1 rust`、`cargo test`（`kukuri-cli`）を実行して回帰確認済み（Windows ネイティブ `cargo test` は既知の `STATUS_ENTRYPOINT_NOT_FOUND` のため Docker 実行で代替）。

### Phase 5 優先度更新（2025年11月03日）
- 進捗ログ: グローバルコンポーザー導線と設定画面モーダル（Priority 1-3）を2025年11月02日にプロトタイプ実装。2025年11月03日はステータス監視カードとプロフィール編集モーダルの反映手順を整理し、未実装の画像アップロード・鍵管理・未接続コマンド対応をバックログに追加。2025年11月06日はトレンド/フォロー導線のデータ要件・テスト計画を Inventory/Summary/CI 計画に反映し、`generated_at` ミリ秒化・Summary Panel・`trending_metrics_job` をフォロータスクとして明示。2025年11月07日は Docker シナリオ `trending-feed` の実行要件と Nightly / スクリプト統合方針を整理し、`docs/03_implementation/trending_metrics_job.md` に集計ジョブの実装・監視ドラフトを作成。QA/バックエンド連携は引き続き課題。
1. 投稿導線統一: `Sidebar`「新規投稿」ボタン → グローバルコンポーザー起動（`useComposerStore` 新設）に加え、TopicSelector の「新しいトピックを作成」ショートカットと `TopicFormModal` mode=`create-from-composer` を実装し、`createAndJoinTopic` で作成直後に投稿へ遷移する UX を整備。
2. プロフィール編集再利用: `ProfileSetup` 共通化と設定画面モーダル導線の実装。
3. プライバシー設定反映: `usePrivacySettingsStore` でトグル状態を管理し、将来のバックエンド連携を見据えて永続化。
4. トレンド/フォロー中フィード拡張: Inventory 5.7 に沿って `/trending`・`/following` の既存実装をブラッシュアップし、`generated_at` をミリ秒エポックへ修正、Summary Panel / DM 未読バッジを追加、`trending_metrics_job`・Docker シナリオ・Nightly テストを整備して発見体験を底上げする。2025年11月15日時点で Summary Panel のラグ表示・DM 会話ラベル・`test-results/trending-feed/{reports,prometheus,metrics}` の Nightly artefact 反映を完了。
5. ユーザー検索導線改善: Inventory 5.8 に基づき `search_users` コマンド拡張（cursor/sort/limit/レートリミット）、`useUserSearchQuery` のデバウンス + 無限スクロール対応、`SearchErrorState` コンポーネントと入力バリデーション（2〜64文字/制御文字除去）を実装し、Vitest/Rust/Docker テストを追加して検索 UX を底上げする。
6. テスト/UX 確認: 新規コンポーザー導線とプロフィール編集モーダル、`RelayStatus`/`P2PStatus` のステータス表示をユニット・統合テストでカバーし、操作ログを `phase5_ci_path_audit.md` に追記する。2025年11月03日: `pnpm test:unit` スクリプトと Nightly Frontend Unit Tests ワークフロー（cron 実行）を追加し、定期実行とローカル検証手順を共通化。フェイクタイマーを用いた `RelayStatus`/`P2PStatus` テストを整備し、同日付で `phase5_user_flow_inventory.md` 5.5節にリレー取得失敗時の UI/リトライ設計とバックオフ方針を追記。
7. 投稿削除キャッシュ整合性: Inventory 5.10 に基づき `useDeletePost` ミューテーションと `invalidatePostCaches` ヘルパーを実装し、トレンド/フォローキャッシュを更新。Docker シナリオ `post-delete-cache` と Rust 統合テスト（`post_delete_flow.rs`）で再現性を担保し、Nightly で監視する。
8. プロフィール画像アップロード: `ProfileForm` のアップロードボタン導線を実装し、`ProfileEditDialog`/オンボーディング双方で同一コードパスを利用できるようにする。
9. 鍵管理モーダル: `export_private_key`/`SecureStorageApi.addAccount` を利用したバックアップ・インポート導線を実装し、保存/復旧手順の注意喚起とロギング方針を明文化する。
10. プロフィール画像リモート同期: iroh-blobs 0.96.0 と iroh-docs 0.94.0 を組み合わせた `upload_profile_avatar` / `fetch_profile_avatar` コマンド、Capability ベースのアクセス制御、`StreamEncryptor` による暗号化、Doc レプリケーション監視フローを実装し、ローカル保存のみの経路を廃止する。

> 詳細設計は `docs/01_project/activeContext/artefacts/phase5_user_flow_inventory.md` のセクション6を参照。

### ハイリスク依存対策メモ（2025年10月24日更新）
- **WSA-01 EventGateway 再配線**: `phase5_event_gateway_design.md` Sprint 2 に沿って `LegacyEventManagerGateway` を `infrastructure::event` へ移設し、`state/application_container.rs`・各ハンドラーは `Arc<dyn EventGateway>` を受け取る。UI との境界は `application::shared::mappers::event` 経由で正規化する。
- **WSA-02 Offline Persistence ポート化**: `application::ports::offline_store` を導入し、Stage1 で `LegacyOfflineManagerAdapter` を挟みつつ Stage2 で `infrastructure/offline/sqlite_store.rs` に移行する。再索引ジョブは新ポート経由でキューを扱い、`SubscriptionStateStore` と同一基盤を共有する。
- **WSA-03 Bookmark Repository 移行**: `domain::entities::bookmark` と `infrastructure::database::bookmark_repository` を追加し、`PostService`／`presentation::handlers::post_handler` を新 Repository に再配線する。`AppState` の `BookmarkManager` フィールドは互換ラッパに縮退させ、最終的に削除する。
- 2025年10月26日: Bookmark API の Repository 統合と Legacy `modules::bookmark` 削除を完了。AppState/Handlers/Tauri コマンドは `PostService` + `BookmarkRepository` 依存のみで動作し、Runbook/タスクリストへ完了ログを追記。
- **WSA-04 SecureStorage / Encryption 再構成**: `infrastructure::storage::secure_storage` に debug/テストユーティリティを移し、`SecureStorageHandler` は新しい `SecureStoragePort`（仮称）を経由。暗号処理は `infrastructure::crypto::encryption_service` トレイトへ集約し、`AppState` の Legacy EncryptionManager / KeyManager 依存を排除する。
- **WSA-05 Legacy Database Connection 廃止（2025年10月25日完了）**: `state`／`EventManager`／`EventHandler` を `infrastructure::database::ConnectionPool` 経由へ再配線し、Legacy `modules::database::{connection,models}` を撤去済み。`.sqlx` は動的クエリのみのため再生成不要であることを確認。依存棚卸しドキュメントにも完了ステータスを反映した。
- **SubscriptionStateMachine**: 2025年10月25日 SSR-01/02 完了。`application::ports::subscription_state_repository.rs` と `infrastructure::database::subscription_state_repository.rs` で Repository を実装し、`SubscriptionStateMachine` はポート越しに遷移管理を行う。再同期バックオフ計算は `domain::value_objects::subscription` に移し、DI から `SqliteSubscriptionStateRepository` を注入する構成へ更新済み。

## MVP完成後の改善

### 予約投稿のバックエンド実装
- 予約投稿の保存機能（SQLite）
- 予約投稿の実行スケジューラー
- Tauriコマンドの実装
- 注：UIは既に実装済み（Phase 3.2）

### 検索機能の拡張
- バックエンドAPI統合
  - 全文検索エンジンの実装
  - 検索結果のキャッシング
- 高度な検索オプション
  - フィルター機能（日付範囲、ユーザー、トピック）
  - ソート機能（関連度、新着順、人気順）
- 注：基本的な検索機能は実装済み（Phase 2.4）

## 開発スケジュール

### 工数見積もり
- Phase 1: ✓ 完了（2日）
- Phase 2: ✓ 完了（3日）
- Phase 3: 3.1〜3.2 完了。Inventory 5.9（トピック作成ショートカット）/5.10（投稿削除キャッシュ）/5.1 Stage3（Doc/Blob + privacy）は 2025年11月10日にクローズ。3.3（リアクション強化）は Post-MVP へ移送。
- Phase 4: Stage1〜3 完了。4.4（Offline UI/UX + Runbook）仕上げと `sync_engine` 追加ログで 1.5 日想定。
  - 4.1 ローカルファーストデータ管理: ✓
  - 4.2 楽観的UI更新: ✓
  - 4.3 同期と競合解決: ✓
  - 4.4 オフラインUI/UX: 進行中（1.5日）
- Phase 5: アーキテクチャ再構成 2週間（依存関係棚卸し→モジュール再配線→テスト再編の順に実施）
- MVP完成後の改善: 2-3日

### 実績
- Phase 1: 2025年07月28日完了（認証フロー実装とテスト）
- Phase 2: 2025年07月30日完了（データ連携基盤と追加機能）
  - 2.1: ホームページの実データ表示（投稿＋トピック一覧）
  - 2.2: トピック機能の実装（投稿作成、トピック管理、P2P連携）
  - 2.3: リアルタイム更新の実装（Nostr/P2Pイベント、データ同期）
  - 2.4: 追加機能の実装（返信/引用機能、検索機能、P2P接続管理）
- Phase 3: 進行中
  - 3.1: 2025年07月31日完了（トピック参加・離脱機能の改善）
  - 3.2: 2025年08月01日完了（新規投稿機能の拡張、予約投稿のバックエンドは保留）
  - 3.3: 次の実装対象（その他のリアクション機能）

### 発見層実装との連携
- Phase 1-2完了後、並行して発見層実装を開始
- 手動接続機能により、発見層完成前でもP2P機能をテスト可能

### 優先順位による調整
- Phase 1-2: 完了 ✓
- Phase 3: 3.1/3.2 完了。トピック作成ショートカット / 投稿削除キャッシュ / プロフィール Stage3（Doc/Blob + privacy）は 2025年11月10日にクローズ。3.3（リアクション）は Post-MVP に回す。
- Phase 4: Stage1〜3 完了。4.4（Offline UI/UX + Service Worker 導線）を MVP の最終ゲートとして対応中。
- オフラインファースト機能は現状の SQLite + Tanstack Query + P2P 同期基盤を活用し、Docker `rust` テスト・`scripts/test-docker` 経由での検証を継続。
- MVP完成後は、ユーザーフィードバックを基にリアクション拡張・Service Worker 拡張・自動再送分析を実装。
- 発見層実装と並行して進行可能

## テスト計画

### 単体テスト
- 各コンポーネントのテスト作成
- ストアのテスト更新
- カバレッジ目標: 80%以上

### 統合テスト
- 認証フローのテスト
- 投稿作成から表示までのフロー
- トピック参加から投稿までのフロー

### E2Eテスト
- 新規ユーザーのオンボーディング
- 既存ユーザーの主要操作

## リスクと対策

### 技術的リスク
1. **Tauriイベントの信頼性**
   - 対策: イベントの再送・リトライロジック

2. **パフォーマンス問題**
   - 対策: 仮想スクロール、ページネーション

3. **データ整合性**
   - 対策: 楽観的UI更新、背景同期

4. **オフライン同期の複雑性**
   - 対策: 段階的実装、十分なテスト

5. **ストレージ容量問題**
   - 対策: 適応的キャッシュ、古いデータの自動削除

### スケジュールリスク
1. **予想外の技術的問題**
   - 対策: バッファ期間の確保、段階的リリース

## 成果物

1. 完全に動作する認証フロー
2. 実データを表示するホーム画面
3. トピック参加・投稿機能
4. リアルタイム更新機能
5. テストカバレッジ80%以上
6. ユーザードキュメント
