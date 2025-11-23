# トピック購読仕様レビュー
- 作成日: 2025年11月23日
- 目的: 公開/非公開トピックの扱いを見直し、「購読のみ」モデルや公開トピックの非ハッシュ化などの変更要求が実現可能かを整理する。

## 現状整理
- IDスキーム: P2P用のIDは `generate_topic_id`（kukuri-tauri/src-tauri/src/domain/p2p/message.rs）で `kukuri:topic:<name>` を生成し、iroh-gossip側（kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_gossip_service.rs）で BLAKE3 ハッシュ 32byte に変換して TopicId を作成。P2PService にも SHA-256 ハッシュ版の `generate_topic_id` があり二重化している（core.rs）。
- 作成/購読フロー: TopicService は作成/参加/離脱を DB（topics / user_topics）に記録し、参加時に P2P join & DHT join を実行（topic_service.rs 経由で p2p_service.join_topic）。AppState は起動時に `public` を作成・参加し UI サブスクライブを張る（kukuri-tauri/src-tauri/src/state.rs）。
- Nostr/イベント: TopicId は平文でタグ化され、暗号化や可視性区分はなし。`Topic.is_public` はあるが実質未使用。
- CLI/コンテナ: kukuri-cli の relay コマンドは `ensure_kukuri_namespace` で `kukuri:topic:<lower>` を生成し BLAKE3 ハッシュにサブスクライブ、デフォルトは `--topics=kukuri`。docker-compose の `RELAY_TOPICS` 環境変数は CLI で読まれておらず既定は public にならない。

## 要求ごとの検証と設計変更案
1. **公開/非公開の2種を持つこと**  
   - 現状はフラグだけあり未運用。Topic / TopicId / SubscriptionState に visibility を追加し、既存 DB に列追加とマイグレーションが必要。

2. **公開トピックはハッシュ化せず `kukuri:tauri:${name}` をそのまま購読**  
   - iroh TopicId は 32byte 固定のため、公開トピックだけ平文で 32byte にパディング/トリムする変換が要る（ハッシュを避けるが長さ制約を満たす必要あり）。`generate_topic_id` の名前空間を `kukuri:tauri:` に変更し、公開トピックは平文 → 32byte、非公開トピックは従来通り BLAKE3（または HKDF）で秘匿化する二段階 KDF を新設する。

3. **非公開トピックは従来通りハッシュ化**  
   - 生成ロジックを一本化し、公開/非公開でブランチ。ハッシュ前の識別子は漏らさないよう TopicId 生成に必ずシークレット（招待で共有）を混ぜる。

4. **「作成」せず「購読」するモデルへ**  
   - TopicService/API/フロントは「購読リスト管理」にリネームし、topics テーブルを「ローカル購読キャッシュ＋メタデータ」に再定義。`create_topic`/`enqueue_topic_creation`/`topics_pending` まわりは廃止か「招待受付/メタデータ登録」に置換。UI の TopicSelector/Composer も「購読追加」導線に変更。

5. **購読はローカル決定で、接続状態と切り離す**  
   - `join_topic` が即 P2P/DHT join を呼ぶ設計をやめ、購読意図を永続化 → 接続確立時に非同期で join/subscribe を試行する形に変更。`state.ensure_ui_subscription` と p2p_service の責務分離が必要。

6. **デフォルト公開トピックの扱い**  
   - 既定トピックを `kukuri:tauri:public` にリネームし、初回起動時に購読リストへ登録するだけにする（現在の「必ず参加」から「購読エントリ＋任意で解除可」に緩和）。既存 DB の topic_id=`public` → 新命名への移行スクリプトが必要。

7. **非公開トピックの招待/鍵共有（NIP 利用）**  
   - 実装候補: NIP-44（最新版 DM 方式）で共有鍵を DM（kind 4/104 相当）として配布し、内容に `topic_secret` とトピック表示名を含める。秘匿トピックのイベントは `topic_secret` から派生した鍵で暗号化し、タグはハッシュ済み ID だけを含める。受信側は招待イベントを持たないと復号できない設計。

8. **CLI デフォルト購読トピックを public にし docker compose で指定**  
   - clap 引数に `env = "RELAY_TOPICS"` を追加しデフォルトを `kukuri:tauri:public` へ変更。BLAKE3 ハッシュを「公開は平文32byte、非公開はハッシュ」に合わせて再実装。`docker-compose.yml` の `RELAY_TOPICS: public` を有効化。

## 実装インパクトまとめ
- ドメイン/DB: TopicId/Topic/SubscriptionState に visibility を追加し、topic_id 命名空間変更と既存データの移行（public → kukuri:tauri:public、ハッシュ化ポリシー変更による再購読）が必須。ビルド未公開のため破壊的マイグレーションで可。
- P2P: TopicId 生成の二経路化（公開=平文32byte、非公開=ハッシュ）。IrohGossipService/DhtIntegration と CLI の TopicId 生成を共通ヘルパーに寄せて二重化を解消。
- アプリ層: TopicService を購読レジストリに改修し、P2P join を接続後リトライ型に変更。イベント発行時は visibility に応じて暗号化/タグ付けを切り替える。
- フロント: topicStore の create/join/leave/offlineキューを「購読追加/削除＋招待受領」に整理し、Nostr subscribe の呼び出しも購読リストに紐付ける。
- テスト/CI: TopicId 変換の互換テスト、公開/非公開購読の E2E、招待鍵が無い場合に復号できないことを NIP-44 ベースで検証する契約テストを追加。

## 未決事項・リスク
- 公開トピックの 32byte 生成は「単純パディング/トリム」で実施し、互換性は考慮不要（破壊的変更を許容）。
- 既存データ/キャッシュ/オフラインキューの移行計画は不要（未公開かつローカルだけなので古いデータは手動削除で対応）。
- 非公開トピックの招待鍵喪失時は、参加者リストをローカル保持し、他参加者への自動鍵問い合わせ（再招待要求）でリカバリする設計とする。将来的には「招待発行権を絞る」機能を追加し、鍵発行権限を持つメンバーのみが再招待できる段階的なコミュニティ形成を想定。

## 実装タスク（ドラフト）
- ID/namespace整理  
  - `generate_topic_id` を公開/非公開で分岐（公開はパディング32byte、非公開はハッシュ）し、`kukuri:tauri:` 名前空間へ統一。IrohGossipService・DhtIntegration・CLI で共通ヘルパーを利用。
- ドメイン/DBスキーマ  
  - Topic/SubscriptionState に visibility フィールド追加。TopicId 値オブジェクトを新命名に対応させる。破壊的マイグレーションを実施。
- 購読レジストリ化  
  - TopicService を「購読追加/解除」中心に再設計（create/enqueue_pending は削除か招待受付用に置換）。P2P join を接続後のリトライ処理に変更。
- デフォルト購読  
  - 既定トピックを `kukuri:tauri:public` に変更し、起動時は購読リスト登録のみ（離脱可能）。AppState の自動UI購読を新命名で張り直す。
- 非公開トピック招待/鍵流通  
  - NIP-44 DM で招待鍵を配布し、鍵喪失時は参加者リストから自動再招待要求を送るフローを実装。参加者リストをローカルに保持する仕組みを追加。
- イベント送受信の暗号/タグ切替  
  - EventPublisher/Gateway で visibility に応じたタグ付けと暗号化を切り替え、非公開は共有鍵派生ID・暗号化コンテンツで送信。
- フロントエンド更新  
  - topicStore/TopicSelector/Composer を「購読追加/解除＋招待受領」導線に変更。Nostr subscribe 呼び出しも購読リスト依存へ寄せる。
- CLI/コンテナ  
  - relay デフォルトトピックを `kukuri:tauri:public` へ変更し、`RELAY_TOPICS` env を clap で受ける。TopicId 生成を共通化。
- テスト/検証  
  - 公開/非公開TopicId生成の単体テスト、招待鍵無しで復号できないことの契約テスト（NIP-44ベース）、購読レジストリ導線のE2Eを追加。`gh act` workflows のformat/testが通るまで修正。
