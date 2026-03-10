# P2Pイベント配信/購読ルーティング設計

最終更新: 2025年09月14日

## 背景/目的
- 背景: 旧All配信はスパム/重複・トラフィック増の懸念がある。アプリはトピック単位でゴシップ網を形成するため、イベントもトピック単位で流通させたい。
- 目的: 非トピック系イベントは「選択中の既定トピック」へ、かつ発信者の「ユーザー固有トピック」に配信。元イベント準拠のイベントは、「元イベントが属するトピック」のみ配信する。

## 用語/前提
- 既定トピック（選択中）: UIで選択された配信対象トピック群（1件以上、複数可）。起動時は`public`を初期選択とする。
- ユーザー固有トピック: 送信者の公開鍵に紐づくトピック。
  - 内部ID: `kukuri:user:{pubkey}`（既存util `user_topic_id(pubkey)` を使用）
  - 外部表現: `/kukuri/users/{pubkey}`（UI表示向け。内部IDへマッピング）
- 元イベント準拠イベント: 反応/リポスト/削除など、`e`タグ等で他イベントを参照するタイプ（Reaction/Repost/EventDeletion 等）。
- 冪等Join: 配信先トピックは送信前に `GossipService.join_topic` を冪等に実行。

## 送信ルーティング（仕様）
- 非トピック系（テキストノート/メタデータ/任意イベント）
  - 配信先: 選択中の既定トピック（複数可） + ユーザー固有トピック
- 元イベント準拠（Reaction/Repost/EventDeletion など）
  - 配信先: 元イベントが属するトピック（のみ）
  - 元イベントの属するトピック解決: DBのイベント→トピック対応から取得。無い場合はイベントの`t`タグやアプリ内インデックスで推定。解決不可なら配信をスキップ（WARNログ）。
- トピック投稿（既存挙動）
  - 配信先: 投稿対象トピック（既存の`publish_topic_post`のまま）
- 重複排除: 配信先集合はSet化し、同一トピックへの二重送信を防止。

## 購読（受信）ルーティング
- 選択中の既定トピック（複数可）とユーザー固有トピックを購読対象へ含める。
- UIが参加中の個別トピックは従来通り購読。
- 受信イベントはイベントIDで重複排除後、UIへ一度だけ配信（複数トピック経由の重複を統合）。

## API/実装変更
- EventManager
  - 既定: `selected_default_topic_ids: HashSet<String>`（複数選択）。セッターは`set_default_p2p_topics(Vec<String>)`、増減APIとして `add_default_p2p_topic/remove_default_p2p_topic/list_default_p2p_topics` を提供。
  - 送信ヘルパ: `broadcast_to_topics(gossip, topics: &[String], event)`（Set化/冪等Join/送信）
  - ルーティング: 各`publish_*`で配信先を決定し`broadcast_to_topics`を呼び出す
  - ユーザー固有トピック: KeyManagerから現在ユーザーのpubkeyを取得→`user_topic_id(pubkey)`で導出
  - 元イベント準拠: 参照イベントID（`e`タグ）からDB→トピック解決（Fallback: `t`タグ）。解決不可ならスキップ。
- Presentation（Tauri）
  - 既定トピック操作: `set_default_p2p_topics(topics: string[])`（一括設定）、`add_default_p2p_topic(topic_id)`、`remove_default_p2p_topic(topic_id)`、`list_default_p2p_topics()`
  - 送信時のユーザー固有トピックは自動付加（追加操作不要）
- GossipService/IrohGossipService
  - 変更不要（join/broadcastは冪等前提で利用）

## データ永続化
- 参照解決を安定化するため、イベント保存時に「イベントID→トピックID」マッピングをDBへ保存することを推奨。
  - 取得: `EventRepository`拡張（例: `get_event_topics(event_id) -> Vec<String>`）
  - 保存: トピック投稿作成時・受信時にマッピング登録

## エッジケース/ポリシー
- 参照トピック未解決: 既定 + ユーザーのみへ送信（ログにWARNを出す）
- 既定トピック未参加: 送信前joinで吸収
- ユーザー未ログイン: ユーザー固有トピック宛ては省略（既定のみ）
- 送信失敗: 個別トピック送信の部分失敗は集約して上位へ通知（成功/失敗件数）

- ## テスト計画
- ユニット
  - 非トピック系: 選択中の既定（複数可）+ユーザーへ送信
  - 元イベント準拠: 参照トピックのみに送信（参照不能時はスキップ）
  - 重複排除: 配信先のSet化を検証
- 統合
  - IrohGossipServiceで実際にjoin→broadcast→subscribeの導線を確認（Dockerランナー推奨）

## 段階移行
- Phase A: 送信ルーティング実装（EventManagerのみ）
- Phase B: 参照解決のDB対応（EventRepository拡張）
- Phase C: 受信側の重複排除とUIへの集約導線

## セキュリティ/運用
- P2PEvent互換送出は継続（UI/既存導線の影響最小化）
- 不正イベント（署名NG）の破棄・ログ出力
- ルーティング先の最大数制限（DoS回避）を設定可能に（例: 3宛先まで）

## 未決事項
- 参照トピックが複数になる可能性（クロスポスト）への扱い：上限設定で対応
- ユーザー固有トピックのUI表示名（`/kukuri/users/{pubkey}` vs `kukuri:user:{pubkey}`）の統一
