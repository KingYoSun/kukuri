# トピック購読仕様レビュー
- 作成日: 2025年11月23日
- 目的: 公開/非公開トピックの扱いを再設計し、「購読のみ」モデルや公開トピックのハッシュ化などの変更要求が実現可能かを整理する。

## 現状整理（着手前の課題）
- IDスキームが二重化：P2P層で `kukuri:topic:<name>` → BLAKE3 ハッシュ、P2PService では SHA-256 版も存在。
- 作成/購読フロー: TopicService が作成・参加・離脱をDBに記録し、参加時に即 P2P/DHT join。AppState 起動時に `public` を作成・参加して UI サブスクライブ。
- Nostrの TopicId は平文タグ化で可視性区別なし。`Topic.is_public` は実質未使用。
- CLI: `ensure_kukuri_namespace` で `kukuri:topic:<lower>` を生成し BLAKE3 ハッシュ購読。`RELAY_TOPICS` との接続は未整備。

## 今回実装した変更
- 名前空間とデフォルトID刷新：`kukuri:tauri:` を採用し、デフォルト公開トピックを `kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0` に変更。レガシー `public` を正規化。
- visibility導入：Topic/TopicId/SubscriptionState に visibility を追加し、DBマイグレーションで列追加。
- ID生成ポリシー統一  
  - 公開トピック: 正規化IDをBLAKE3ハッシュし `kukuri:tauri:<64桁hex>` を生成。  
  - 非公開トピック: 公開トピックと同じハッシュ方式で生成。
- P2P/CLI統合：DHT/Gossip/CLI すべてで共通の TopicId 生成を使用し、RELAY_TOPICS のデフォルトを `kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0` に変更。
- デフォルト購読の扱い緩和：起動時は購読リスト登録のみ（強制joinを停止）、削除・離脱ガードを定数ベースに修正。
- DBは破棄・再作成前提のため、トピックID変更のマイグレーションは追加しない。
- format-checkフォロー: Prettier差分を解消し `gh act --job format-check` 成功を確認。

## 完了報告
- 購読レジストリ化の方針をドメイン/API/UIで統一し、購読追加/解除を基点にした導線へ変更（joinを遅延・再試行型で扱う前提に更新）。
- UI文言を購読モデルに合わせてリフレッシュ（TopicSelector/TopicFormModalなどで「購読追加」「同期待ち」等に統一）。
- 非公開トピックの鍵共有は NIP-44 ベースで後続タスクとして扱わない方針をドキュメント化し、現行仕様を完了とする。
- レガシーID（`public`）は正規化対象として扱い、既存DBは破棄・再作成前提で移行は行わない（オフラインキューも初期化）。

## 次のアクション
なし（本タスクは完了）。
