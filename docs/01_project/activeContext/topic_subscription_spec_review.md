# トピック購読仕様レビュー
- 作成日: 2025年11月23日
- 目的: 公開/非公開トピックの扱いを再設計し、「購読のみ」モデルや公開トピックの非ハッシュ化などの変更要求が実現可能かを整理する。

## 現状整理（着手前の課題）
- IDスキームが二重化：P2P層で `kukuri:topic:<name>` → BLAKE3 ハッシュ、P2PService では SHA-256 版も存在。
- 作成/購読フロー: TopicService が作成・参加・離脱をDBに記録し、参加時に即 P2P/DHT join。AppState 起動時に `public` を作成・参加して UI サブスクライブ。
- Nostrの TopicId は平文タグ化で可視性区別なし。`Topic.is_public` は実質未使用。
- CLI: `ensure_kukuri_namespace` で `kukuri:topic:<lower>` を生成し BLAKE3 ハッシュ購読。`RELAY_TOPICS` との接続は未整備。

## 今回実装した変更
- 名前空間とデフォルトID刷新：`kukuri:tauri:` を採用し、デフォルト公開トピックを `kukuri:tauri:public` に変更。レガシー `public` を正規化。
- visibility導入：Topic/TopicId/SubscriptionState に visibility を追加し、DBマイグレーションで列追加。
- ID生成ポリシー統一  
  - 公開トピック: 平文IDを32byteにパディングして購読。  
  - 非公開トピック: BLAKE3ハッシュを32byteで生成。
- P2P/CLI統合：DHT/Gossip/CLI すべてで共通の TopicId 生成を使用し、RELAY_TOPICS のデフォルトを `kukuri:tauri:public` に変更。
- デフォルト購読の扱い緩和：起動時は購読リスト登録のみ（強制joinを停止）、削除・離脱ガードを定数ベースに修正。
- format-checkフォロー: Prettier差分を解消し `gh act --job format-check` 成功を確認。

## 残課題・リスク
- 購読レジストリ化: TopicService/フロント導線を「購読追加/解除」中心に整理し、接続状態から分離する対応が未完。
- UI文言/導線: 作成→購読追加の文言・UX調整が未完。
- 秘匿トピックの鍵共有: 招待/共有フロー（NIP-44等）実装と鍵紛失時のリカバリ方針が未決。
- 既存データ移行: ローカルDB内レガシーID・オフラインキュー整合性の確認が必要。

## 次のアクション
1. 購読レジストリ化のAPI/UI更新（joinを遅延・再試行型にし、購読リストを単一ソースに統一）。
2. トピック作成/購読UIの文言・操作導線を「購読追加」基準にリファイン。
3. 非公開トピックの招待/鍵共有仕様を決定し、NIP-44ベースのテストを追加。
4. レガシーIDデータの移行とオフラインキュー整合性チェックを実施。
