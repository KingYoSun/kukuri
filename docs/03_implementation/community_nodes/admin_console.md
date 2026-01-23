# 管理画面（Admin Console）実装計画

**作成日**: 2026年01月22日  
**技術スタック（要件）**: React / TypeScript / Vite / shadcn/ui / zod / zustand / TanStack Query / TanStack Router

## ゴール

- `bootstrap` / `relay` / `index` / `moderation` / `trust` を一元管理できる
- サービスが分離（未起動）でも、UI が壊れず「オフライン/未接続」として扱える
- 設定変更・手動操作（ラベル発行、ジョブ実行等）が監査ログに残る

## 画面（提案）

1. **Dashboard**
   - 全サービスの稼働状態（health）、DB/Meilisearch 接続状態、ジョブ滞留数
   - outbox backlog、reject 急増、DBディスク逼迫などの主要指標/アラートを可視化（詳細: `docs/03_implementation/community_nodes/ops_runbook.md`）
2. **Services**
   - 各サービスの設定（有効/無効、エンドポイント、閾値、レート制限など）
   - relay/bootstrap の認証モード切替（デフォルトOFF → 後から必須化）
     - OFF→ON は予約（`enforce_at`）+ 猶予（`grace_seconds`）で段階的に切替し、既存接続の扱いを明確化する（詳細: `docs/03_implementation/community_nodes/auth_transition_design.md`）
3. **Subscriptions**
   - 購読申請（pending/approved/rejected）の審査
   - node-level subscription（relay取込対象）の一覧/上限/停止
   - user-level subscription（pubkey別）の確認（課金導線は後回しでも状態確認は必要）
   - プラン/利用量（usage）/超過状況の可視化（課金/利用量計測の運用のため）
     - 詳細: `docs/03_implementation/community_nodes/billing_usage_metering.md`
4. **Policies**
   - 利用規約（ToS）/プライバシーポリシー（Privacy）の作成・公開・current切替
   - バージョン履歴、発効日（effective）管理、プレビュー
   - 同意必須化の前提（User API の `CONSENT_REQUIRED` と連動）
5. **Privacy / Data**
   - 個人データの取扱い（保持期間の初期値、削除/エクスポート要求）の運用ビュー
   - 同意ログの監査（IP/UA 等の短期保持設定、削除要求時の匿名化方針の確認）
   - 削除/エクスポート要求のジョブ状況（queued/running/completed/failed）と再実行/中止
   - 詳細: `docs/03_implementation/community_nodes/personal_data_handling_policy.md` / `docs/03_implementation/community_nodes/ops_runbook.md`
6. **Moderation**
   - ルールベースフィルタ（CRUD / 優先度 / テスト実行）
   - 手動ラベリング（label 発行）と監査ログ
   - LLM 連携設定（OpenAI / Local、外部送信ON/OFF、送信範囲、保存/保持、予算上限）
     - 詳細: `docs/03_implementation/community_nodes/llm_moderation_policy.md`
7. **Trust**
   - 2方式（通報/コミュ濃度）のパラメータ（時間窓、重み）
   - trust 計算ジョブの実行/スケジュール、結果の確認（対象検索）
8. **Index**
   - Meilisearch の状態、インデックス再構築、ランキングパラメータ
9. **Audit Logs**
   - 管理操作の履歴（ユーザー/時刻/差分/対象）
10. **Access Control**
   - 招待capability（39021）の発行/失効、利用状況（expires/max_uses/used_count）
   - メンバーシップ（topic+scope+pubkey）の一覧/検索、追放（revoke）
   - epoch ローテ（topic+scope 単位）、再配布状況（失敗/未配布の検知）
   - 詳細: `docs/03_implementation/community_nodes/access_control_design.md`

## 状態管理

- **Server state**: TanStack Query（fetch/retry/cache、ポーリングで health 反映）
- **UI state**: Zustand（ログイン状態、サイドバー状態、フォーム途中状態など）
- **スキーマ**: zod（API レスポンス/フォームバリデーション共通化）

## API 連携（前提）

- ブラウザから直接各サービスを叩かず、`Admin API` に集約する（CORS/認証/障害耐性のため）
- `Admin API` が各サービスの health を収集し、UI は統一のモデルで表示する
- 詳細: `docs/03_implementation/community_nodes/admin_api.md`

## 認証（補完）

- 最小: `POST /v1/admin/auth/login`（password）→ session cookie（推奨）または JWT
  - ブラウザ運用では `httpOnly cookie` を優先（XSS 耐性）
- RBAC は後回し（v1 は admin のみ）。ただし監査ログは v1 から必須。
