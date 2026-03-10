# rate limit（DoS/濫用対策）設計（v1）

**作成日**: 2026年01月23日  
**対象**: `./kukuri-community-node`（`cn-user-api` / `cn-bootstrap` / `cn-relay` / reverse proxy）

## 目的

- 外部公開面（HTTP/WS/P2P）に対する DoS/濫用の影響を局所化し、**サービス停止・DB過負荷・コスト暴騰**を避ける
- 認証OFF運用（導入容易性）でも、最小限の防御（rate limit 等）を常に有効にする（`docs/03_implementation/community_nodes/auth_transition_design.md`）
- 「瞬間レート制限（429）」と「課金/上限（402）」を混同せず、設計・運用・監査の責務を分離する

## 結論（v1で確定）

- **Redis/Valkey は v1 では導入しない**
- rate limit は **各サービス内の in-mem** を正とする（プロセス再起動でリセットされる。DoS 対策としては許容）
- Postgres を rate limit のホットパスに使わない（DB は **設定の正（SoT）/監査/永続 denylist・penalty**に限定）
- profiles 分離（サービス分割）と整合する前提:
  - v1 は「各サービス = 1インスタンス（単一ホスト/単一Compose）」を基本とし、in-mem を成立条件にする
  - 水平スケール（同一サービス複数台）したい場合は v2 で分散 backend を追加する（本ドキュメント末尾）

## レート制限（429）とクォータ（402）の切り分け

- **429（rate limit）**: “瞬間的な濫用/DoS” の抑止が目的。単位は requests/sec 等。再起動で状態が消えてもよい
- **402（quota）**: “日次/期間上限・課金” が目的。単位は requests/day 等。永続カウンタ/監査が必要
- 402/クォータの設計は `docs/03_implementation/community_nodes/billing_usage_metering.md` が正

## 適用点（v1）

### 1) User API（HTTP）

- 差し込み点: `tower` layer（`axum` の middleware）
- キーの基本:
  - public: **IP**
  - authenticated: **pubkey + IP**（併用。片方が効いてもよい）
  - token は “pubkey を引ける場合の別名” として扱い、過度に token 単独へ寄せない（短命tokenの再発行で回避されるため）
- 入口の分離（例）:
  - `/v1/auth/challenge`, `/v1/auth/verify`: **強め**（総当たり/署名爆撃対策）
  - `/v1/bootstrap/*`, `/v1/policies/*`: **中**（初回導線を壊さない）
  - 検索/トレンド/トラスト等: **中〜強**（コストが高いものは特に）
- 超過時:
  - HTTP: `429 Too Many Requests` + `Retry-After`
  - エラーコード（例）: `RATE_LIMITED`

### 2) bootstrap（HTTP）

- 基本は public 前提のため、**IP rate limit は常時有効**（認証OFF時も必須）
- 差し込み点は HTTP と同様（`tower` layer）

### 3) relay（WS）

- 目的: 認証OFFでもゲームされにくい最小防御を入れる（`#t` 必須等とセット）
- キーの基本:
  - 認証OFF/未AUTH: **IP**
  - 認証ON/ AUTH 済み: **pubkey + IP**（併用）
- 最低限のバケット分離（例）:
  - 接続確立（接続数/再接続頻度）: per IP
  - メッセージ受信: per IP（種別で分離。`REQ` は別枠）
  - バックフィル要求: per IP / per pubkey（DB負荷に直結するため強め）
- 超過時（互換性優先）:
  - publish（`EVENT`）: `OK false`（reason: `rate-limited` など）
  - subscribe（`REQ`）: `CLOSED`（reason: `rate-limited`）+ 必要なら `NOTICE`

### 4) relay（iroh-gossip ingest）

- P2P では IP に依存できないため、キーは **remote peer（node id）** を基本にする（取得できない場合のみ remote addr）
- バケット（例）:
  - per peer
  - global（全体の受理上限。CPU/メモリ保護）
- 超過時: メッセージを破棄し、監視指標（メトリクス）へ記録する（ネットワークへ過剰な応答を返さない）

## 設定モデル（運用上の正）

- rate limit の設定の正（SoT）は **`cn_admin.service_configs`**（`config_json`）に置く
  - Admin Console → Admin API → DB 更新 → 監査ログ記録 → `NOTIFY` + 各サービスの再取得
  - 設定反映方式は `docs/03_implementation/community_nodes/admin_api.md` の方針に従う
- 例（概念）:
  - `rate_limit.enabled`
  - `rate_limit.http.by_ip`, `rate_limit.http.by_pubkey`
  - `rate_limit.ws.conn_by_ip`, `rate_limit.ws.req_by_ip`, `rate_limit.ws.event_by_pubkey`
  - `rate_limit.iroh.by_peer`, `rate_limit.iroh.global`
- サービス側の安全策:
  - 設定が取得できない/壊れている場合は、**安全なデフォルト**（低めの上限）で起動できる
  - per-key のメモリ消費を抑えるため、キー状態は TTL/LRU 等で上限を持つ

## 監視（メトリクス/ログ）

- 監視指標は `docs/03_implementation/community_nodes/ops_runbook.md` の方針を踏襲し、少なくとも reject を集計する
  - 例: `*_rejected_total{reason=ratelimit,...}`
- 注意:
  - IP/pubkey/token 等の **高カーディナリティ値をラベルに入れない**
  - ログにも生の識別子を出さない（必要ならハッシュ化）

## v2（将来の拡張）

次の要件が出たら、分散 backend（Redis/Valkey 等）を追加する。

- `cn-user-api` / `cn-relay` を **複数台**にして負荷分散したい（インスタンス横断で “同じレート制限” を強制したい）
- 複数ホスト構成で「片系が落ちても rate limit の整合が欲しい」

v2 の実装方針:

- backend を `inmem|redis` のように差し替え可能にする（ただし v1 は in-mem のみ）
- 追加コスト（監視/バックアップ/障害点）に見合うタイミングでのみ profile 追加する

