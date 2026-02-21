# 個人データの取扱い方針（保持期間 / 削除・エクスポート / 同意ログ） v1

**作成日**: 2026年01月23日  
**対象**: `./kukuri-community-node`（User API / Admin API/Console / Postgres / observability）

## 目的

- コミュニティノードが取り扱う個人データの範囲と保持期間を明確にし、運用と Privacy 記載に整合を取る
- ユーザー（pubkey）が自分のデータの削除/エクスポートを要求できる仕組みを v1 で用意する
- 同意ログを監査可能（append-only）にしつつ、最小化と削除要求への対応を両立する

## 前提

- 利用者の識別子は原則 pubkey（Nostr 鍵）とする（`docs/03_implementation/community_nodes/user_api.md`）
- relay/bootstrap はデフォルト認証OFFであり、その間は同意も不要として扱う（`docs/03_implementation/community_nodes/auth_transition_design.md` / `docs/03_implementation/community_nodes/policy_consent_management.md`）
- P2P（iroh-gossip/WS）で拡散したイベントはネットワークから回収できないため、削除は本ノード上の保存/検索/再配信/下流反映に対する best-effort とする（`docs/03_implementation/community_nodes/event_treatment_policy.md`）

## 1. 個人データの分類（v1）

本計画では「個人データ」を、直接識別子（メール等）に限らず、**識別可能性がある情報**（pubkey、IP、User-Agent 等）まで含めて広く扱う。

### 1.1 ユーザー識別・認証

- pubkey（生値）
- トークン/セッション（JWT、refresh token 等）
- 認証ログ（成功/失敗、最終ログイン時刻等。本文/秘密情報は含めない）

### 1.2 規約/プライバシー同意（Consent）

- 同意対象（policy type/version/locale、content_hash）
- 同意者（pubkey）
- 同意日時
- 監査情報（任意）: IP / User-Agent、同意署名（否認性対策。v2候補）

### 1.3 購読・課金・利用量

- topic購読状態（申請/承認/停止、プラン、クォータ）
- 利用量イベント（`X-Request-Id`、metric、units、outcome 等）
- 監査ログ（プラン変更、例外付与等）

### 1.4 Access Control（KIP-0001）

Access Control は **P2P-only** を正とし、本ノードでは原則扱わない。
（将来的に扱う場合は個人データとして同等の管理が必要。）

### 1.5 通報・モデレーション・トラスト

- report（通報者 pubkey、対象、理由等）
- label/assertion（発行者 pubkey、対象、期限等）
- trust 計算用のグラフ（エッジ、スコア、根拠。pubkey を含む）
- LLM moderation の判定結果（event_id 参照、カテゴリ/スコア、provider 等）
  - 外部送信範囲/保存/保持は `docs/03_implementation/community_nodes/llm_moderation_policy.md` を優先する

### 1.6 取込イベント（relay）

- 保存イベント（`event_json`）に個人情報が含まれる可能性がある（投稿本文、プロフィール等）
- 保持/削除（NIP-09/NIP-40）・永続化ポリシーは以下に従う
  - `docs/03_implementation/community_nodes/event_treatment_policy.md`
  - `docs/03_implementation/community_nodes/ingested_record_persistence_policy.md`

### 1.7 運用ログ/メトリクス/トレース

- 原則として本文/識別子（pubkey 生値、IP、生 User-Agent）をログに残さない（`docs/03_implementation/community_nodes/ops_runbook.md`）
- 追跡が必要な場合は salt 付きハッシュ等で代替する（例: `pubkey_hash`）

## 2. 保持期間（retention）方針（v1 初期値）

初期値であり、Admin Console からノード運用として調整可能にする（テーブル/サービスごとの上書きを許容）。

|分類|主な対象（例）|保持期間（v1 初期値）|備考（削除/匿名化）|
|---|---|---:|---|
|認証トークン|access/refresh token|有効期限まで|削除要求時は即時失効/削除|
|同意ログ（本体）|policy_consents|アカウント存続中 + 2年|削除要求時は pubkey をハッシュ化し、IP/UA を削除（最小の同意レシートのみ保持）|
|同意ログ（IP/UA）|policy_consents.ip/user_agent|30日|短期保持。期限後は NULL 化（同意本体は維持）|
|購読/プラン|subscriptions/topic_subscriptions|アカウント存続中 + 1年|削除要求時は無効化→削除（監査目的はハッシュ化で保持可）|
|利用量イベント|usage_events|180日|削除要求時は pubkey をハッシュ化して保持（会計/濫用調査）。再同意/再登録で復元しない|
|利用量集計|usage_counters_*|90日|削除要求時は削除（再計算可能）|
|通報イベント|reports|180日|削除要求時は reporter_pubkey をハッシュ化（ユニーク数/重み付けを維持）|
|トラスト計算結果|trust_edges/scores|90日（再計算）|削除要求時は対象 pubkey のエッジを削除し、再計算（v1）|
|Access Control|（v1は対象外）|—|—|
|取込イベント（relay）|cn_relay.events 等|topicごとの ingest_policy|削除要求は「本ノードの保存/検索からの除外」に留まる（ネットワークからの回収は不可）|
|運用ログ|stdout/trace|7〜30日|本文/識別子は原則含めない。必要ならハッシュ化|
|バックアップ|pg_dump 等|30日（世代管理）|削除要求の反映は “バックアップの自然消滅” で遅延する（Privacy に明記）|

補足:
- 「保持期間」は `created_at` ではなく、運用上は `ingested_at` / `accepted_at` / `created_at` のどれを基準にするかを各テーブルで明確化する（取込イベントは `ingested_at` を正: `docs/03_implementation/community_nodes/ingested_record_persistence_policy.md`）。
- 将来、決済連携（メール/請求情報）を追加する場合は、別途 “法務/会計” の保持要件を上乗せする（v2）。

## 3. 削除要求（Right to deletion）設計（v1）

### 3.1 対象範囲（v1）

削除要求は「本ノードが管理しているユーザー関連データ」を対象とし、次を提供する。

- User API/DB 上のユーザー関連データ（同意、購読、利用量、通報、メンバーシップ等）の削除または匿名化
- 本ノードの検索（`cn_search.post_search_documents`）・トラスト計算（AGE）・モデレーション結果からの除外（派生データの削除/再計算）

削除要求で **できない** こと（明記）:

- iroh-gossip/WS で既に拡散したイベントを、ネットワーク上から回収すること

### 3.2 削除方式（推奨）

v1 は「全消去」ではなく **匿名化（pseudonymization）+ 追跡停止** を基本にする。

- 即時:
  - 認証トークンを失効（ログアウト）
  - topic購読を停止（user-level subscription を `ended`）
  - 以後の API を拒否（`410 Gone` または `403`）
- 非同期ジョブ（Deletion Job）で実施:
  - 参照整合性を保った削除/匿名化（下記）
  - 派生データの削除（PostgreSQL検索）と再計算（AGE）

匿名化の基本:
- pubkey 生値を保持せず、`subject_hmac = HMAC(node_secret, pubkey)` を保存して “同一人物判定だけ可能” にする
  - `node_secret` はローテーション可能な secret とし、ローテーション後は過去データの追跡性を落とせる（運用で選択）

### 3.3 削除ジョブの具体（例）

1. `user_deletion_requests` を作成（status=`queued`）
2. トークン/セッションを失効（JWT の場合も `cn_user.subscriber_accounts.status=deleting` 等で以後の保護 API を即時拒否できる）
3. `cn_user` のユーザー関連テーブルを削除/匿名化
   - consents: `accepter_pubkey` を NULL 化し、`accepter_hmac` のみ保持（同意レシートとして必要な範囲）
   - subscriptions/topic_subscriptions: 無効化→削除（監査が必要なら `subscriber_hmac` を残す）
   - usage_events/reports: pubkey を hmac 化（ユニーク数/監査用）
  - access_control: v1 は対象外（将来導入時は memberships を無効化し、invites/key_envelopes を削除または暗号化済みでも hmac 化して保持）
4. 派生系の削除/再計算
   - PostgreSQL検索: subject が author のドキュメントを削除（取込イベントが残っても検索結果から除外）
   - AGE: subject を頂点/エッジごと削除し、必要ならスコア再計算
5. 監査ログを残し、status=`completed`

## 4. エクスポート要求（Right to export）設計（v1）

### 4.1 エクスポート対象（v1）

認証済みユーザーに対し、少なくとも次をエクスポート可能にする。

- 同意履歴（type/version/locale/hash、accepted_at）
- topic購読（申請/承認/停止の履歴）
- プラン/購読状態（課金が導入されていない v1 でも “状態” は出せる）
- 利用量（メトリクス別の集計 + 監査用イベント）
- 通報履歴（自分が行った通報）
- Access Control: 自分の membership 状態（topic/role/epoch 等）
- relay 取込イベント: `event.pubkey == subject_pubkey` のイベント（本ノードが保持している範囲）

含めない（v1）:
- 他者の個人データ（第三者のイベント本文、他者の通報者等）
- 秘密情報（JWT secret、node鍵等）

### 4.2 形式/配布

- 形式: `zip`（中身は JSON/CSV/NDJSON を用途別に分割）
- 配布: User API から “一度だけ” 取得できる署名付きURL（または download token）
- 保持: 生成物は短期保持（例: 24時間）で自動削除

## 5. 同意ログ（Consent log）の扱い（v1）

同意ログは監査・紛争対応に必要な一方、IP/UA 等の付帯情報は個人データとして強い。

- `policy_consents` は append-only を原則とする（更新は行わず、撤回は “撤回イベント” を別途追加）
- `ip`/`user_agent` は既定で保存しない（必要時のみ保存し、30日で自動削除）
- アカウント削除要求時:
  - `accepter_pubkey` は削除し、`accepter_hmac` のみ保持（同意した事実の最小レシート）
  - `ip`/`user_agent` は即時削除

否認性対策（v2候補）:
- `content_hash` に対する署名（`accepter_sig`）を保持し、同意の否認性を下げる

## 6. Privacy/ToS に必ず記載する事項（v1）

Admin Console のポリシー作成時テンプレートに、少なくとも以下を含める。

- 収集するデータの種類（pubkey、購読、利用量、通報、ログ等）と目的
- 保持期間（上表の初期値と、運用で変更し得ること）
- 削除要求の範囲と制約（P2P 拡散済みイベントは回収できない）
- エクスポート要求の方法（User API の申請、生成物の短期保持）
- バックアップからの削除は遅延する可能性（最大保持期間）
- 問い合わせ窓口/手順

## 関連ドキュメント

- `docs/03_implementation/community_nodes/user_api.md`
- `docs/03_implementation/community_nodes/policy_consent_management.md`
- `docs/03_implementation/community_nodes/billing_usage_metering.md`
- `docs/03_implementation/community_nodes/ops_runbook.md`
- `docs/03_implementation/community_nodes/event_treatment_policy.md`
- `docs/03_implementation/community_nodes/ingested_record_persistence_policy.md`
- `docs/03_implementation/community_nodes/llm_moderation_policy.md`
