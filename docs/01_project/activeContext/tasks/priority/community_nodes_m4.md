# Community Nodes 実装タスク（M4: Moderation v1/v2準備）

最終更新日: 2026年01月24日

目的: report（39005）受理と label（39006）発行を実装し、運用者がルールベースで抑制できる状態にする。LLM は v2 として差し込める形にする。

参照（設計）:
- `docs/03_implementation/community_nodes/services_moderation.md`
- `docs/03_implementation/community_nodes/llm_moderation_policy.md`
- `docs/03_implementation/community_nodes/event_treatment_policy.md`
- `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
- `docs/03_implementation/community_nodes/user_api.md`
- `docs/03_implementation/community_nodes/policy_consent_management.md`
- `docs/03_implementation/community_nodes/rate_limit_design.md`

## M4-1 データモデル（reports/labels/rules/job）

- [x] report/label の永続化テーブルを作る（exp必須、policy_ref/policy_url を紐付け）
- [x] ルールベースフィルタの定義/優先度/有効無効を管理できるスキーマを作る
- [x] moderation の処理キュー/進捗/再実行を管理できるようにする

## M4-2 User API: report 受理 / label 参照

- [x] `POST /v1/reports` を実装する（同意必須 + rate limit + metering（attempts））
- [x] `GET /v1/labels?target=...` を実装する（topic で絞れる形を検討）

## M4-3 Moderation worker（ルールベース v1）

- [x] outbox を追従し、対象イベントをルールベースで評価して label を発行する
- [x] label（39006）は Node Key で署名し、`exp` を必須にする
- [x] `policy_url`/`policy_ref` を label に含め、透明性を担保する

## M4-4 Admin API/Console: ルール管理と監査

- [x] ルールの CRUD を Admin API に実装し、更新を監査ログへ残す
- [x] Admin Console に Moderation 画面を実装する（ルール、通報一覧、手動ラベル）

## M4-5 LLM（v2準備）

- [x] `LLMProvider` 抽象を用意し、openai/local/disabled を差し替え可能にする（既定: disabled）
- [x] 送信範囲/保持/予算/停止条件は `llm_moderation_policy.md` に従い、設定の正を `cn_admin` に置く

## M4 完了条件

- [x] report を受理でき、ルールベースで label が発行され、User API から取得できる
- [x] 運用者が Admin Console からルールを変更し、監査ログに残る
