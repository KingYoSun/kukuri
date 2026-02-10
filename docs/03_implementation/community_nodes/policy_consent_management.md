# 規約/プライバシーポリシー管理と同意（Consent）設計

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`

## ゴール

- 各コミュニティノードで **利用規約（ToS）/プライバシーポリシー（Privacy）** を管理できる
- User API の保護機能は **同意しない限り利用できない**（必須化）
  - 例外: bootstrap/relay は認証OFFの間は同意不要（ユーザー操作の手間を最小化）
- 同意の履歴を監査可能にする（いつ/誰が/何に同意したか）

## 前提

- 外部公開する HTTP API は User API に集約する（`docs/03_implementation/community_nodes/user_api.md`）
- 管理操作は Admin API/Console に集約する（`docs/03_implementation/community_nodes/admin_console.md`）

## 方針

1. **ポリシーはバージョン管理**
   - `type=terms|privacy` と `version`（例: `2026-01-22`）で識別する
   - 公開（published）と発効（effective）を分ける
2. **同意は pubkey 単位で記録**
   - kukuri の ID は pubkey を基本とし、同意も pubkey に紐づける
3. **同意の必須化は User API の認可レイヤで実施**
   - 同意がない場合、保護された API は拒否する（同意が必要なことと取得方法を返す）
4. **ポリシー更新時の再同意**
   - `current` が更新されたら、ユーザーは新バージョンへの同意が必要になる
   - 既存トークンが残っていても、API 実行時に DB の同意状態で判定する
5. **外部送信/自動判定の開示**
   - 外部モデレーションAPI（例: OpenAI）へ送信する可能性がある場合、Privacy に送信範囲/保持/目的/停止条件を明記する
   - 詳細: `docs/03_implementation/community_nodes/llm_moderation_policy.md`

## 公開URL（policy_url）との連携

- ノードの `node.descriptor(kind=39000)` には `policy_url` を載せられるため、ここを「人間が読める規約ページ」へ向ける。
  - 例: `https://node.example/policies`（reverse proxy で `user-api` の `GET /v1/policies/...` を HTML として配信）
- アプリ/クライアントが機械的に参照する場合は、JSON の `GET /v1/policies/current` を利用する。

## データモデル（提案）

### ポリシー（Admin 管理）

- `policies`
  - `policy_id`
  - `type`（`terms|privacy`）
  - `version`（文字列。日付/セマンティックは運用で決める）
  - `locale`（例: `ja-JP`）
  - `title`
  - `content_md`（または `content_html`。v1 は MD を推奨）
  - `content_hash`（同意対象のハッシュ）
  - `published_at`
  - `effective_at`
  - `is_current`（type+locale で一意に current を持つ）

### 同意（User 管理）

- `policy_consents`
  - `consent_id`
  - `policy_id`
  - `accepter_pubkey`
  - `accepter_hmac`（削除要求後の匿名化用。任意）
  - `accepted_at`
  - `ip`（任意。既定は保存しない。保存する場合も短期保持）
  - `user_agent`（任意。既定は保存しない。保存する場合も短期保持）

補足（v2）:
- 法的/監査要件が強い場合、`accepter_sig`（同意内容ハッシュへの署名）を追加し、非否認性を上げる。

## 保持期間/削除要求への対応（v1）

- `policy_consents` は append-only を原則とする（更新は行わず、必要なら “撤回イベント” を別途追加）
- `ip`/`user_agent` は **既定で保存しない**。保存する場合も短期（例: 30日）で自動削除（NULL化）する
- アカウント削除要求時は、同意ログの最小レシートだけ残す
  - `accepter_pubkey` を削除（NULL化）し、`accepter_hmac` を保持（同一人物性の最小証跡）
  - `ip`/`user_agent` は即時削除

詳細は `docs/03_implementation/community_nodes/personal_data_handling_policy.md` を参照。

## Admin Console での規約管理（計画）

### 画面（提案）

- **Policies**
  - ToS / Privacy の current 版の表示（言語ごと）
  - 新版の作成（ドラフト→公開→current 反映）
  - 履歴（version 一覧）と diff/プレビュー
  - 反映状態（published/effective/current）

### 管理 API（例）

- `POST /v1/admin/policies`（作成）
- `PUT /v1/admin/policies/:policy_id`（更新）
- `POST /v1/admin/policies/:policy_id/publish`（公開）
- `POST /v1/admin/policies/:policy_id/make-current`（current 切替）
- 互換エイリアス（非推奨）: `/v1/policies*`

## User API での同意管理（計画）

### ポリシー取得

- `GET /v1/policies/current`（必要な current version/URL/ハッシュを返す）
- `GET /v1/policies/:type/:version?locale=ja-JP`（本文取得）

### 同意状態

- `GET /v1/consents/status`（認証必須。自分の同意済み version を返す）
- `POST /v1/consents`（認証必須。同意登録。ToS/Privacy をまとめて受け付け可能にする）

### 同意必須化（拒否レスポンス）

保護 API で同意が不足している場合は、次を返す。

- HTTP: `428 Precondition Required`（推奨）
- body（例）:
  - `code: "CONSENT_REQUIRED"`
  - `required: [{ type, version, locale, url, content_hash }]`

補足:
- `GET /v1/policies/*` は認証不要で到達できる必要がある。
- `POST /v1/consents` は認証は必要だが、同意がなくても到達できる必要がある（同意登録自体をブロックしない）。

## relay（WS）での同意必須化（計画）

課金/購読制御のために relay の利用に認証を要求する場合（NIP-42 等）、AUTH 成功時に次を追加で確認する。

- 当該 pubkey が current の ToS/Privacy に同意済みか
- 未同意の場合は `NOTICE` を返し、購読を拒否する（または接続を切る）

補足:
- relay が認証OFF（anonymous）の場合、pubkey を特定できないため「同意必須化」を relay 側で強制できない。
  - 本計画では **認証OFF = 同意不要**とし、ユーザー操作の手間を最小化する。
  - 同意を強制したい場合は、管理画面から relay の認証必須化を ON にする。
  - 認証OFF→ON 切替時の猶予/既存接続の扱いは `docs/03_implementation/community_nodes/auth_transition_design.md` を参照。

## 関連ドキュメント

- `docs/03_implementation/community_nodes/user_api.md`
- `docs/03_implementation/community_nodes/admin_console.md`
- `docs/03_implementation/community_nodes/topic_subscription_design.md`
- `docs/03_implementation/community_nodes/llm_moderation_policy.md`
- `docs/03_implementation/community_nodes/personal_data_handling_policy.md`
