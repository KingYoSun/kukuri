# Bootstrap サービス 実装計画

**作成日**: 2026年01月22日  
**役割**: discovery 補助（ヒント配布）+ node capability の広告

## 責務

- `node.descriptor(kind=39000)` / `node.topic_service(kind=39001)` の署名付き配布
- `scope`（public/friend/invite 等）を含む広告の管理（Access Control と整合させる）
  - 詳細: `docs/03_implementation/community_nodes/access_control_design.md`
- クライアントが初回接続時に必要とする bootstrap ヒント（既知ノード/エンドポイント）の配布
- 管理画面からの設定変更（対応 topic、ポリシーURL、管轄、連絡先等）の反映

## 外部インタフェース（提案）

- **HTTP（外部公開は User API に集約）**
  - `GET /v1/bootstrap/nodes`（node descriptor の一覧/差分）
  - `GET /v1/bootstrap/topics/:topic/services`（topic_service の一覧）
- **イベント（KIP）**
  - 39000/39001 を定期発行し、クライアントは gossip/DHT/既知URL 経由で収集できる

## 39000/39001 の配布経路と運用上の正（v1確定）

### 運用上の正（source of truth）

v1 では「**DB上の広告設定**」を入力として `bootstrap` が 39000/39001 の **署名済み event JSON** を生成し、その生成物（event JSON）を **DBに保存**する。  
外部へ配る 39000/39001 は、原則としてこの「DBに保存された署名済み event」を配る（再生成の重複や service 間の差異を避ける）。

- SoT:
  1. `cn_bootstrap`（推奨）または `cn_admin` の「広告設定」（roles/endpoints/policy_url/対象topic等）
  2. `bootstrap` が生成した「署名済み 39000/39001（event JSON）」の保存領域
- 生成者: `bootstrap` のみ（Node Key で署名）
- 配布者:
  - **HTTP 配布は User API が担う**（外部I/F集約）。`bootstrap` は内部サービスとして DB の生成物を更新する
  - gossip/DHT/既知URL は「発見/更新のヒント」であり、最終整合は HTTP 取得で行う

### 配布経路の優先順位（v1）

1. **DB（正）**: 署名済み event JSON と失効判定（`exp`）の根拠
2. **HTTP（正）**: `User API` の `GET /v1/bootstrap/*`（配布I/Fの正。差分取得/キャッシュ制御をここで提供）
3. **gossip（ヒント）**: 39000/39001 の更新通知・加速（取りこぼし前提）
4. **DHT（ヒント）**: node の HTTP endpoint 発見（必要なら descriptor hash 程度）
5. **既知URL（シード）**: 手動固定/配布された接続先（HTTP取得の起点）

### event の安定キー（置換/キャッシュ/失効のための前提）

39000/39001 は kind=390xx で addressable なので、v1 の運用として `d` タグを必須とする（`docs/03_implementation/community_nodes/event_treatment_policy.md` の addressable 取り扱いに合わせる）。

- 39000 `node.descriptor`:
  - `["d","descriptor"]`（ノードにつき 1 本に収束）
- 39001 `node.topic_service`:
  - `["d","topic_service:<topic_id>:<role>:<scope>"]`（topic×role×scope を置換キーにする）

補足:
- これにより、配布経路が混在しても「有効な最新（effective view）」が定義できる（同一 `created_at` 競合の取り扱いは NIP-01 に従う）。

### 発行頻度 / `exp`（推奨デフォルト）

基本: **設定変更時に即時再発行 + 定期再発行（keep-alive）**。

- 39000（descriptor）
  - 推奨: `exp = now + 7日`
  - 定期再発行: 24h（または `exp/2`）
- 39001（topic_service）
  - 推奨: `exp = now + 48h`
  - 定期再発行: 6h〜12h（topic構成の変化を吸収）

失効:
- `exp` 超過は **無効扱い**（HTTP配布でも原則返さない）
- Node Key ローテーション時は、ローテ後の鍵で再発行し、旧鍵の広告は `exp` により自然失効させる（緊急停止が必要なら DB 側で `is_active=false` のようなフラグで即時停止できる設計にする）

### HTTP キャッシュ（User API 配布の推奨）

`GET /v1/bootstrap/*` は条件付きGETを前提にする。

- `ETag`: event JSON（またはレスポンスボディ）のハッシュ
- `Last-Modified`: `bootstrap` の生成時刻（または DB 更新時刻）
- `Cache-Control`: 短め（例: `max-age=300`）+ `stale-while-revalidate`（任意）
- レスポンスに `next_refresh_at = min(exp, now + 24h)` のような「次回更新推奨」を含める（クライアントの再取得戦略を安定化）

### gossip / DHT の扱い（ヒント）

- gossip:
  - 39000/39001 の “更新があった” ことを伝えるヒントとして扱う（受信=即採用ではなく、HTTPで再取得して確定）
  - 取りこぼし前提なので、クライアントは一定間隔（例: `next_refresh_at`）で HTTP 再取得できるようにする
- DHT:
  - v1 は **endpoint discovery** に寄せる（例: `node_pubkey -> https://node.example`）
  - TTL は `exp` より短く（例: 6h）して定期的に再公告する（停波したノードが残りにくい）

## 認証（デフォルトOFF / 後から必須化）

- デフォルトでは bootstrap の取得は **認証OFF**（public）とし、初回接続・発見の導線を壊さない
- 認証OFFの間は **同意（ToS/Privacy）も不要**とする（ユーザー操作の手間を最小化）
- 管理画面（Services）から `bootstrap` の **認証必須化**を切り替え可能にする
  - ON の場合、User API 側で「認証済み + 同意済み（ToS/Privacy）」を要求できる
  - OFF の場合でも、返す情報は **公開可能な範囲**（public topic/公開エンドポイント）に限定する
- 認証OFF→ON 切替の運用（予約/猶予/互換性、最小 public bootstrap の扱い）は `docs/03_implementation/community_nodes/auth_transition_design.md` を参照

## データ

- Postgres に「広告設定」（name, roles, endpoints, policy_url, jurisdiction, contact, topics）を保存
- 発行した event のメタ（発行時刻/次回期限）を保存（再発行管理）

補足:
- `policy_url` は `docs/03_implementation/community_nodes/policy_consent_management.md` の「公開URL」方針に沿って設定し、運用者が更新できるようにする。

## 実装手順（v1）

1. Node Key の読み込み/生成（volume に保管）
2. 39000/39001 の生成（必須 tag / `exp` / 署名）
3. HTTP API での配布（クライアントが取り込める最低限）
4. Admin API からの設定変更反映（DB → bootstrap）
5. rate limit（DoS 対策）と監査ログ（設定変更のみでも記録）
   - rate limit の実装方針（v1）は in-mem（Redis無し）。設定の正は cn_admin（詳細: `docs/03_implementation/community_nodes/rate_limit_design.md`）
