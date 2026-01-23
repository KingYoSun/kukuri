# LLM Moderation 送信/保存/開示ポリシー（v1）

**作成日**: 2026年01月23日  
**対象**: `./kukuri-community-node`（moderation / User API / Admin API/Console）

## 目的

- moderation の LLM ラベリング自動化において、**外部送信範囲**・**保存/ログ/保持**・**コスト上限**・**Privacy への記載**を v1 方針として確定する
- “ルールベース + LLM 補助”を安全に運用できる前提（監査、最小化、停止条件）を揃える

## 前提

- LLM は **補助**（既定はルールベース）。LLM 失敗や停止が発生しても、サービス全体が破綻しない設計にする
- 外部送信（OpenAI Moderation API 等）は **デフォルトOFF**。運用者が Admin Console で明示的に有効化する
- private scope（`invite/friend/friend_plus` 等の暗号化領域）は **v1 では LLM 対象外**（外部送信もしない）

## 参照（ローカル）

- `docs/03_implementation/community_nodes/services_moderation.md`
- `docs/03_implementation/community_nodes/policy_consent_management.md`
- `docs/03_implementation/community_nodes/user_api.md`

## 外部送信（OpenAI Moderation API）

### 送信してよいデータ（最小化）

v1 は **テキストのみ**を対象にする（添付/リンク先本文/画像は送らない）。

- `event.content`（最大長を設定して truncate）
- 追加で必要なら “通報理由” 等の短い補助情報（ただしユーザー識別子は含めない）

### 送信してはいけないデータ

- pubkey、IP、User-Agent、JWT、メール/電話等の識別情報
- topic_id（原則送信しない。必要なら運用上のカテゴリ名に丸める）
- tags 全量、イベント JSON 全文、添付のバイナリ、鍵/招待情報（39020/39021 等）

### 前処理（推奨）

- 個人情報らしきパターンの簡易マスク（メール/電話/URL等）
  - 注意: マスクは漏れを完全に防げないため、送信範囲そのものを最小化する
- `X-Request-Id` 等の相関IDは内部用に発行し、外部へ “ユーザー識別に繋がるID” を送らない

## Self Hosting（オープンウェイト）運用

- `local` provider は外部送信ではないが、**保存/ログ/保持**は同じ最小化方針を適用する
- モデル/推論サーバ（例: `ollama`）は別コンテナで起動し、Admin Console から ON/OFF できる

## 保存/ログ/保持（v1）

### DBに保存してよいもの（推奨）

- `event_id` 参照（生イベント本文は参照で引けるため、重複保存しない）
- `provider`（`openai|local`）、モデル識別子、判定カテゴリ/スコア、実行時刻
- `policy_version` / `prompt_version`（運用変更の追跡）
- `input_hash`（入力を復元できない形での照合用。任意）

### DBに保存しない（デフォルト）

- 送信した本文（raw input）
- 外部レスポンス全文（必要なカテゴリ/スコアのみ抽出して保存）

例外（デバッグ）:
- “短期・限定アクセス・明示ON” の場合のみ、送信本文スナップショットを別テーブルへ保存してよい
- 保持期間は短期（例: 7日）で、自動削除する

### ログ出力（推奨）

- 本文はログに出さない（成功/失敗/例外時も同様）
- 出すのは `request_id`, `event_id`, `provider`, `latency_ms`, `status`, `estimated_cost` 程度に留める

### 保持期間（初期値の例）

- LLM 判定結果: 30〜180日（運用で調整）
- デバッグ用スナップショット: 0日（保存しない）または 7日

## コスト上限/停止条件（必須）

外部送信は運用コストが読みづらいので、必ず上限と停止条件を持つ。

- `max_requests_per_day`（日次リクエスト上限）
- `max_cost_per_day`（日次コスト上限。推定でよい）
- `max_concurrency`（並列数上限）
- 上限超過/連続失敗時は **LLM を自動停止**し、ルールベースにフォールバックする
  - 「LLM未実施」フラグ（例: `llm_skipped_reason=budget|disabled|error`）を記録して監査可能にする

冪等/重複呼び出し防止（推奨）:
- `(event_id, provider, prompt_version)` を一意扱いにし、同一入力の再判定は手動re-runのみ許可する

## 開示（Privacy への記載項目）

運用者が外部送信を有効化する場合、Privacy には少なくとも以下を記載する。

- 目的（スパム/違法/有害コンテンツ検出等）
- 外部送信の有無（設定により外部モデレーションサービスへ送信する場合がある）
- 送信するデータの範囲（本文の必要最小限、識別子は送らない、添付は送らない等）
- 保存するデータの範囲（判定結果のみ等）、保持期間
- 人手レビューの可能性（管理者が監査/再判定する場合がある）
- v1 の割り切り（private scope は対象外等）

詳細な文面の整備は `docs/03_implementation/community_nodes/policy_consent_management.md` のポリシー管理フローに従う。

## Admin Console に置くべき設定（v1）

- `llm_enabled`（全体ON/OFF）
- `provider = openai|local`
- `external_send_enabled`（openai のみ。既定OFF）
- 送信前処理（truncate長、マスクON/OFF）
- 対象条件（topic/kind、report対象のみ、ルール一致時のみ等）
- 予算（`max_requests_per_day`, `max_cost_per_day`, `max_concurrency`）
- retention（判定結果/スナップショット）
- 監査ログ（設定変更、手動ラベル、re-run）

