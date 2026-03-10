# コミュニティノード 実装計画（Community Nodes）

**作成日**: 2026年01月22日  
**対象**: `./kukuri-community-node`（Docker Compose で起動するコミュニティノード一式）

本ディレクトリは `docs/01_project/activeContext/community_node_plan.md`（KIP-0001 Draft）をベースに、実際に運用できる「コミュニティノード」を実装するための設計・実装計画を、サービス単位で分割して整理する。

## 要件（今回の前提）

- 実装は `./kukuri-community-node` 配下に追加する
- 管理画面（React + TypeScript + Vite + shadcn/ui + zod + zustand + TanStack Query/Router）を持つ
  - `kukuri-community-node/apps/admin-console/src/components/ui/` の共通 UI（shadcn/ui 構成）を利用する
- 管理画面で `bootstrap` / `relay` / `index` / `moderation` / `trust` を一元管理する（ただし各サービスは分離可能）
- relay は取込・配信・永続化の必須コンポーネントとして扱い、取込経路を一本化する
  - 取込/再配信のP2Pプロトコルは iroh-gossip（旧CLI互換）とする
- relay と bootstrap はデフォルト認証OFFで起動し、管理画面から後から認証必須化できる
- relay/bootstrap は認証OFFの間は同意（ToS/Privacy）も不要として扱い、後から認証必須化した場合に同意チェックを有効化できる
- 起動は Docker Compose（profiles でサービス出し分け）
- RDB は単一の Postgres サービスに集約する
- 旧CLI は `bootstrap` / `relay` として `./kukuri-community-node` に統合する
- 外部公開する HTTP インターフェイスは User API に集約する（認証/課金/購読/レート制限の統一）
- Access Control（invite/keys）は P2P-only。User API に `/v1/invite/redeem` `/v1/keys/envelopes` は提供しない。
- 利用規約/プライバシーポリシーへの同意を必須化し、User API で同意状態を管理する
- indexer は PostgreSQL（`cn_search.post_search_documents`）を利用する
- moderation はルールベースのフィルタ設定 + LLM によるラベリング自動化を組み込む
  - LLM は OpenAI Moderation API と、オープンウェイトモデルの Self Hosting に両対応する
- trust は「通報ベース」「ユーザーごとのコミュニケーション濃度ベース」の2種類を用意し、Apache AGE（Postgres 拡張）で計算する
- topic 購読は「ユーザー購読（課金/権限）」と「ノード取込購読（relay取込対象）」を分離して設計する

## ドキュメント一覧

- `docs/03_implementation/community_nodes/architecture_overview.md`: 全体アーキテクチャ/責務分割/データフロー
- `docs/03_implementation/community_nodes/repository_structure.md`: `kukuri-community-node` 構成案（Rust サービス + Admin UI）
- `docs/03_implementation/community_nodes/docker_compose_profiles.md`: Compose + profile 設計、環境変数、起動手順
- `docs/03_implementation/community_nodes/ops_runbook.md`: 運用要件（監視/バックアップ/マイグレーション/違法・通報対応）Runbook
- `docs/03_implementation/community_nodes/user_api.md`: 外部I/F統合（User API）、認証/課金/レート制限の方針
- `docs/03_implementation/community_nodes/topic_subscription_design.md`: topic 購読（ユーザー購読/ノード取込購読）設計
- `docs/03_implementation/community_nodes/event_treatment_policy.md`: イベント種別（削除/置換/エフェメラル等）と保存/配信/下流反映ポリシー
- `docs/03_implementation/community_nodes/ingested_record_persistence_policy.md`: 取込レコード永続化ポリシー（dedupe/保持期間/容量上限/パーティション）
- `docs/03_implementation/community_nodes/outbox_notify_semantics.md`: outbox/NOTIFY 配信セマンティクス（at-least-once/offset/リプレイ/バックプレッシャ）
- `docs/03_implementation/community_nodes/access_control_design.md`: Access Control（39020/39021/39022、P2P join、epochローテ/追放）設計
- `docs/03_implementation/community_nodes/auth_transition_design.md`: relay/bootstrap 認証OFF→ON切替（既存接続/猶予期間/互換性）設計
- `docs/03_implementation/community_nodes/billing_usage_metering.md`: 課金/利用量計測（課金単位、無料枠/上限、超過時挙動、監査）設計
- `docs/03_implementation/community_nodes/rate_limit_design.md`: rate limit（DoS/濫用対策）の実装方式（Redis無し/in-mem、適用点、設定の正、監視）
- `docs/03_implementation/community_nodes/llm_moderation_policy.md`: LLM moderation 送信/保存/開示ポリシー（外部送信範囲、ログ/保持、コスト上限、Privacy への記載）
- `docs/03_implementation/community_nodes/policy_consent_management.md`: 規約/プライバシーポリシー管理と同意（必須化）設計
- `docs/03_implementation/community_nodes/personal_data_handling_policy.md`: 個人データの取扱い（保持期間、削除/エクスポート要求、同意ログ）方針
- `docs/03_implementation/community_nodes/admin_console.md`: 管理画面の画面/状態/通信設計
- `docs/03_implementation/community_nodes/admin_api.md`: Admin API（control plane）の最小設計（設定モデル/監査/health 集約/設定反映）
- `docs/03_implementation/community_nodes/api_server_stack.md`: User/Admin API の実装スタック決定（Web FW/OpenAPI/認証/middleware/logging/metrics）
- `docs/03_implementation/community_nodes/postgres_age_design.md`: Postgres 集約 + Apache AGE のスキーマ/運用設計
- `docs/03_implementation/community_nodes/services_bootstrap.md`: bootstrap サービス実装計画
- `docs/03_implementation/community_nodes/services_relay.md`: relay サービス実装計画
- `docs/03_implementation/community_nodes/services_index.md`: index サービス（PG-only）実装計画
- `docs/03_implementation/community_nodes/services_moderation.md`: moderation サービス（ルール + LLM）実装計画
- `docs/03_implementation/community_nodes/services_trust.md`: trust サービス（Apache AGE）実装計画
- `docs/03_implementation/community_nodes/cn_cli_migration.md`: 旧CLI統合方針（bootstrap/relay）

## 全体マイルストーン（推奨）

`community_node_plan.md` の M0-M6（プロトコル中心）を、今回の「運用可能なサービス群」へ落とし込む。

1. **M0: 仕様・境界の確定**
   - KIP-0001 + NIP-85（39000/39001/39005/39006/30382-30385/10040/39020/39021/39022）と HTTP API の責務分界を確定
2. **M1: リポジトリ雛形 + Compose**
   - `postgres(+age)` / `relay` / `user-api` / `admin-api` / `admin-console` を `docker compose up` で起動
3. **M2: bootstrap/relay 統合**
   - 旧CLIを `kukuri-community-node` に統合し、Compose サービス化（profile 対応）
4. **M3: Index v1（PostgreSQL検索）**
   - relay取込レコード → 正規化 → `cn_search.post_search_documents` 反映 → 検索 API
5. **M4: Moderation v1/v2**
   - v1: ルールベースで label(39006) 発行（exp 必須）
   - v2: LLM でラベリング自動化（OpenAI / Self Hosting）
6. **M5: Trust v1**
   - AGE による2方式（通報/コミュ濃度）計算 → NIP-85 assertion（30382-30385）発行

## v2互換の実装ルール（破壊的変更を避ける）

v2/後回し事項（RBAC、決済連携、NIP-98互換、bytes課金、分散rate limit、outbox水平化等）を v1 の後に後付けできるよう、v1 実装では次のルールを守る。

- **APIバージョニング**: `v1` の挙動は変えず、拡張は `v2` 追加または後方互換な“追加”に寄せる（レスポンスのフィールド追加は許容、必須化/意味変更は避ける）
- **文字列enum前提**: `scope/role/status` 等は文字列（DBは `TEXT`）として扱い、未知値は安全側（deny/ignore）で処理できるようにする（Postgres `ENUM` は原則使わない）
- **設定の前方互換**: `cn_admin.service_configs.config_json` は未知フィールド/未知設定値で落ちない（ignore + default）。必要なら `schema_version` を持たせる（`docs/03_implementation/community_nodes/admin_api.md`）
- **DB移行の非破壊**: expand→deploy→contract の順で行い、同一リリースでの破壊的DDL（`DROP`/rename等）を避ける（`docs/03_implementation/community_nodes/ops_runbook.md`）
- **公開URLの固定**: `PUBLIC_BASE_URL`（例: `https://node.example/api`）とパス（`/api/*`、`/relay`）は長期固定を前提にする（NIP-42 `relay` の一致/JWT `iss` の一致を壊さない）。やむを得ず変更する場合は alias/許可リストを実装側で持てる余地を残す
- **Nostrイベント拡張耐性**: tags/content は“追加される”前提で扱い、tag順に依存しない。検証は必須tagの最小セットに限定する
- **内部I/Fの差し替え**: rate limit/outbox などは backend/consumer 追加を見越した差し込み点を維持する（`docs/03_implementation/community_nodes/rate_limit_design.md` / `docs/03_implementation/community_nodes/outbox_notify_semantics.md`）

## 未決定事項チェックリスト

実装に着手する前に、少なくとも以下を確定する。

- [x] relay の取込/配信の役割分担（iroh-gossip + WS）と topic→購読フィルタの写像、バックフィル/再接続時の整合性を v1 方針として決定
- [x] relay の取込プロトコルは iroh-gossip（旧CLI互換）に確定
- [x] topic→iroh-gossip topic の写像、バックフィル/再接続時の整合性（EOSE 等の扱い）を v1 方針として決定（`docs/03_implementation/community_nodes/services_relay.md` / `docs/03_implementation/community_nodes/topic_subscription_design.md`）
- [x] topic→iroh-gossip topic の写像の安定性（正規化/環境分離/バージョニング/移行）を v1 方針として決定
- [x] バックフィルの提供元と保証範囲（relay DB を正とし、P2P 側の履歴同期は v2 で検討）を決定
- [x] 再接続時カーソル設計（`since/until` のマージン、同一 timestamp の並び順、EOSE 後の増分取得）を v1 方針として決定
- [x] WS 側の購読フィルタ（REQ/EOSE など）を `docs/nips/01.md` に寄せる範囲（互換性/拡張）と制約（上限/拒否条件）を v1 方針として決定
- [x] WS 側の購読フィルタの濫用対策（REQフィルタ複雑さ、購読数、バックフィル要求のレート制限）を v1 方針として決定
- [x] topic_id の抽出/正規化仕様（どの tag/フィールドから決めるか、複数topic/未指定の扱い）を v1 方針として決定
- [x] イベント種別の扱い（削除/置換/エフェメラル等）と、保存/再配信/下流サービスへの反映方針を v1 方針として決定（`docs/03_implementation/community_nodes/event_treatment_policy.md` / `docs/03_implementation/community_nodes/services_relay.md`）
- [x] 取込レコード永続化ポリシー（dedupe/削除/編集/保持期間/容量上限/パーティション）を v1 方針として決定（`docs/03_implementation/community_nodes/ingested_record_persistence_policy.md` / `docs/03_implementation/community_nodes/services_relay.md`）
- [x] outbox/NOTIFY の配信セマンティクス（at-least-once 前提の idempotency、offset、リプレイ、バックプレッシャ）を v1 方針として決定（`docs/03_implementation/community_nodes/outbox_notify_semantics.md`）
- [x] 冪等性とループ回避（at-least-once/重複配送/順不同を前提に `event.id` で dedupe、ブリッジでの再注入禁止）を v1 方針として決定
- [x] KIP-0001 Access Control（39020/39021/39022）と **P2P join**、epochローテ/追放運用の v1 方針を決定（`docs/03_implementation/community_nodes/access_control_design.md` / `docs/03_implementation/community_nodes/services_relay.md`）
- [x] relay/bootstrap の認証OFF→ON 切替時の挙動（既存接続の扱い、猶予期間、互換性）を v1 方針として決定（`docs/03_implementation/community_nodes/auth_transition_design.md` / `docs/03_implementation/community_nodes/services_relay.md` / `docs/03_implementation/community_nodes/services_bootstrap.md`）
- [x] 課金/利用量計測の定義（課金単位、超過時の挙動、無料枠/上限、監査）を v1 方針として決定（`docs/03_implementation/community_nodes/billing_usage_metering.md` / `docs/03_implementation/community_nodes/user_api.md`）
- [x] LLM moderation の送信/保存/開示ポリシー（外部送信範囲、ログ/保持、コスト上限、Privacy への記載）を v1 方針として決定（`docs/03_implementation/community_nodes/llm_moderation_policy.md` / `docs/03_implementation/community_nodes/services_moderation.md` / `docs/03_implementation/community_nodes/policy_consent_management.md`）
- [x] 運用要件（監視/メトリクス/ログ、バックアップ/リストア、マイグレーション手順、違法/通報対応 Runbook）を v1 方針として決定（`docs/03_implementation/community_nodes/ops_runbook.md`）
- [x] 個人データの取扱い（保持期間、削除/エクスポート要求、同意ログの扱い）を v1 方針として決定（`docs/03_implementation/community_nodes/personal_data_handling_policy.md` / `docs/03_implementation/community_nodes/user_api.md` / `docs/03_implementation/community_nodes/policy_consent_management.md` / `docs/03_implementation/community_nodes/billing_usage_metering.md` / `docs/03_implementation/community_nodes/ops_runbook.md`）

## レビュー指摘（要修正/要追記/要検討）チェックリスト

### 要修正（矛盾/読み違いの余地を潰す）

- [x] topic 識別子の表現を統一する（KIP Draft/実装設計ともに `topic_id = kukuri:<64hex>` を正とし、イベントタグは `t`（`#t` フィルタ）に統一）
- [x] 同意（consent）の「到達可能（public）/認証必須（authenticated）/同意必須（consent_required）」の定義を整理し、`POST /v1/consents` を「認証必須（同意は不要）」として明文化する（`policy_consents` が pubkey 単位である前提と衝突しないようにする）

### 要追記（実装ブレ/運用事故を減らす）

- [x] 39000/39001 の配布経路を確定し、優先順位（DB正/HTTP正/gossip/DHT/既知URL）と運用上の正（source of truth）を明文化する（発行頻度/`exp`/キャッシュ/失効の扱いも含める。詳細: `docs/03_implementation/community_nodes/services_bootstrap.md` / `docs/03_implementation/community_nodes/user_api.md`）
- [x] `Admin API` の最小設計を 1 枚にまとめる（設定モデルの正: `cn_admin`、監査ログ、認証方式、services health 集約、各サービスへの設定反映方式（poll vs `LISTEN/NOTIFY`）。詳細: `docs/03_implementation/community_nodes/admin_api.md`）

### 要検討（技術選定の“最後の確定”）

- [x] Rust（User API/Admin API）で採用する Web フレームワークと周辺（OpenAPI、認証、middleware、structured logging、metrics）を確定する（詳細: `docs/03_implementation/community_nodes/api_server_stack.md`）
- [x] User API の認証方式を確定する（v1: 署名チャレンジ（kind=22242 推奨）→ 短命 access token。v2候補: NIP-98 互換）（詳細: `docs/03_implementation/community_nodes/user_api.md`）
- [x] rate limit の実装方式を確定する（v1: Redis無し/in-mem。設定の正は cn_admin。profiles 分離と整合。詳細: `docs/03_implementation/community_nodes/rate_limit_design.md`）

## 実装着手前の残件（決め打ち）

チェックリストは完了しているが、実装着手時に “選択肢のまま残っている部分” をブレなく決めるため、少なくとも以下を決め打ちする。

- [x] User API の `access_token` は **JWT（HS256）**に確定（短命/refresh無し）。失効は DB の状態（disable/deletion等）で即時反映（`docs/03_implementation/community_nodes/user_api.md` / `docs/03_implementation/community_nodes/api_server_stack.md` / `docs/03_implementation/community_nodes/personal_data_handling_policy.md`）
- [x] `cn_admin.service_configs` の正（SoT）は **DB**に確定（env は secrets + 初回seed入力に限定し、seed後は DB を優先してドリフトを避ける）（`docs/03_implementation/community_nodes/admin_api.md` / `docs/03_implementation/community_nodes/docker_compose_profiles.md`）
- [x] 初期 admin ユーザー作成/復旧は **`cn-cli` で行う**（bootstrap/reset-password 等）（`docs/03_implementation/community_nodes/admin_api.md`）
- [x] reverse proxy は **Caddy** に確定（外部公開は `https://<host>/api/*`（User API）+ `wss://<host>/relay`（relay）に集約。Admin系は原則インターネット公開しない）（`docs/03_implementation/community_nodes/user_api.md` / `docs/03_implementation/community_nodes/docker_compose_profiles.md`）
- [x] DB/migrations は **sqlx + migrations** に確定（migrate は `cn-cli` の one-shot で実行。`query!` を使う場合は `.sqlx/` をコミット）（`docs/03_implementation/community_nodes/repository_structure.md`）
