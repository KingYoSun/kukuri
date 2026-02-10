# Moderation サービス 実装計画

**作成日**: 2026年01月22日  
**役割**: report 受理、label 発行、監査ログ（提案モデル）

## 責務

- `report(kind=39005)` の受理（入力。外部受付は User API に集約）
- relay が保存した取込レコード（Postgres）を対象に、内部ポリシーに基づく判定を行う
- `label(kind=39006)` の発行（署名付き提案、`exp` 必須）
- **ルールベース**フィルタの管理（管理画面から設定）
- **LLM によるラベリング自動化**
  - OpenAI Moderation API（外部）
  - オープンウェイトモデルの Self Hosting（内部/別コンテナ）

## 入力/出力（v1）

- 入力
  - report（User API 経由で受理 → Postgres）
  - 取込レコード（relay → Postgres。outbox（正）+ `LISTEN/NOTIFY`（起床通知）で追従）
    - offset/冪等の詳細: `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
    - 置換/削除/期限切れ等の扱いは relay が統一し、moderation は「有効なイベント（upsert）」を主に対象とする
    - 詳細: `docs/03_implementation/community_nodes/event_treatment_policy.md`
- 出力
  - label（Postgres に記録し、必要なら relay 経由で配信/発行する）

## ルールベースフィルタ（v1）

### ルール表現（例）

- 条件: `kind`, `content`（正規表現/キーワード）, `tags`, `author(pubkey)` 等
- アクション: `label=spam|nsfw|illegal|...`, `confidence`, `exp`, `policy_ref`

最初は「安全に表現できる範囲」を優先し、Turing 完全な DSL は避ける（誤設定が大事故になりやすいため）。

## LLM ラベリング（v2）

LLM の外部送信/保存/開示/予算（コスト上限）は v1 で先に方針を確定する（運用事故防止のため）。

- 詳細: `docs/03_implementation/community_nodes/llm_moderation_policy.md`

### Provider 抽象化（必須）

- `LLMProvider` インタフェースを定義し、結果を内部の共通ラベルへ正規化する
  - `openai`: OpenAI Moderation API を呼び出し
  - `local`: self-host された HTTP エンドポイント（例: `ollama`）を呼び出し

### 処理フロー（提案）

1. 新規イベント（または通報対象）をキューに積む（Postgres）
2. worker が LLM 判定
3. 閾値を超えたら `label(39006)` を発行（`exp` を短めに）
4. 管理画面で human review / 再判定 / 無効化ができる（監査ログ必須）

## 外部インタフェース（提案）

- 外部公開は User API に集約する
  - `POST /v1/reports`（受理）
  - `GET /v1/labels?target=...`（配布/参照）
- 管理者操作は Admin API 経由
  - `POST /v1/admin/moderation/labels`（手動発行）
  - 互換エイリアス: `POST /v1/labels`（非推奨。既存クライアント移行期間のみ）

## 重要な非機能要件（補完）

- DoS 耐性: report はゲームされやすいので rate limit 必須
- 透明性: `policy_url` と `policy_ref` を label に含め、根拠と責任範囲を明確化
- 運用（違法/通報対応、封じ込め、証跡）: `docs/03_implementation/community_nodes/ops_runbook.md`
