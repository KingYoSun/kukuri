# GitHub Actions E2E 失敗対応レポート

**作成日**: 2025年12月20日  
**作業者**: Codex  
**カテゴリ**: CI/E2E

## 概要
GitHub Actions の desktop E2E 失敗（WebDriver 接続不安定と React の無限再描画）を調査し、起動待ちと投稿経路の整理で安定化しました。関連ユニットテストも合わせて修正し、CI の format-check / native-test-linux を再実行して正常終了を確認しました。

## 実施内容

### 1. tauri-driver 起動待ちの追加
- WebDriver がポート待ちになる前に接続して失敗していたため、proxy/driver の listen 完了を待機。

### 2. Reply/Quote 投稿経路の統一
- `ReplyForm` / `QuoteForm` を `usePostStore.createPost` に切り替え。
- topicId がない場合のみ従来の `TauriApi.createPost` を使用。
- `PostCard` 系ユニットテストの mock と期待値を更新。

### 3. テスト専用分岐の撤廃
- `isE2E` 分岐を削除し、`useProfileAvatarSync` / `usePosts` / `useTopics` を通常挙動に戻した。

### 4. CI ジョブの再実行
- `gh act --workflows .github/workflows/test.yml --job format-check`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`

## 影響範囲
- E2E 実行の安定性向上（driver 起動待ち）。
- Reply/Quote の投稿処理と PostCard テストの期待値。
- Profile Avatar 同期と Topic/Post 取得が通常挙動に復帰。

## 確認結果
- format-check: 成功
- native-test-linux: 成功

## 追記: E2E専用分岐撤廃（2025年12月20日）
- アプリ側の `__E2E_*` / `isE2E` 分岐を削除し、Topic/Post/検索/オフライン挙動を通常経路へ統一。
- E2Eテストは bridge/UI 操作へ移行（オフライン切替、トピック削除、アバターURL入力、ユーザー検索のページングなど）。
- E2E実行環境変数の撤廃（`VITE_ENABLE_E2E` / `TAURI_ENV_DEBUG` 依存を削除）。
