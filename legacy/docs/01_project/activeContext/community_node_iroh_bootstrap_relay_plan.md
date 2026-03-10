# Community Node iroh 接続確立 + Gossip Bootstrap 両立計画（cn-iroh-relay）

作成日: 2026年03月04日  
対象: `kukuri-community-node` / `kukuri-tauri`

## 1. 背景

- 現行の `cn-relay` は「Nostr互換 relay の入口 + iroh-gossip ブリッジ」が中心であり、iroh の接続確立（custom relay 機能）を直接提供しない。
- 一方で kukuri は `iroh-gossip` を中心に運用するため、Community Node 側に「bootstrap peer 機能（動的トピック参加）」が必要である。
- 投稿を `cn-relay` に取り込む導線を維持するなら、`cn-relay` 自体が安定した gossip bootstrap peer として振る舞う方が構成効率が高い。

## 2. 目的

1. iroh Endpoint 間の接続確立を安定化する（IPv6/到達性制約を含む）。
2. `iroh-gossip` の bootstrap peer 機能を Community Node に実装する（動的トピック参加を含む）。
3. `cn-relay` への投稿取込経路と、P2P 接続確立経路を矛盾なく共存させる。

## 3. 命名方針

- iroh custom relay サービスは `cn-iroh-relay` と命名する。
- `cn-relay` は既存どおり Nostr 互換 relay + gossip bridge の責務を維持する。

## 4. 役割分担（確定）

| コンポーネント | 主責務 | 非責務 |
|---|---|---|
| `cn-iroh-relay` | iroh custom relay（接続確立補助、到達性担保） | Nostr イベント永続化、WS `/relay` 処理 |
| `cn-relay` | Nostr 入口、DB 永続化、`iroh-gossip` bridge、bootstrap peer 参加 | iroh custom relay プロトコル実装 |
| `cn-bootstrap` | 39000/39001 生成、bootstrap 更新ヒント publish | iroh endpoint 接続制御 |
| `cn-user-api` | `/v1/bootstrap/*` 配布、hint API、runtime bootstrap 集約 | gossip join 制御 |
| `kukuri-tauri` | endpoint 初期化、bootstrap hint 反映、topic join | Community Node 側の bootstrap event 生成 |

## 5. 技術方針（iroh 公式資料準拠）

### 5.1 接続確立（custom relay）

- クライアントと Community Node の iroh endpoint は、`RelayMode::Default` 固定をやめ、`cn-iroh-relay` を含む custom relay 設定を使用する。
- 直結できる場合は direct 経路、不可時は relay 経路で成立させる。
- relay URL は複数指定可能にし、障害時フェイルオーバー可能にする。

### 5.2 Gossip bootstrap peer（動的トピック参加）

- `cn-relay` は node-level subscription に追従して topic を動的に `subscribe/unsubscribe` する。
- 各 topic 参加時は bootstrap peer ヒントを利用して join 収束時間を短縮する。
- hint 更新（`cn_bootstrap_hint`）受信時に topic 別 peer 候補を再評価し、再 join を許可する。

## 6. アーキテクチャ

### 6.1 接続層

- 新規サービス `cn-iroh-relay` を Community Node compose に追加する。
- `cn-relay` / `kukuri-tauri` / テストハーネスは、環境変数で `cn-iroh-relay` の URL 群を受け取り endpoint 作成時に適用する。

### 6.2 発見・配布層

- `cn-relay` の `/v1/p2p/info` は以下を返す。
  - `node_id`
  - `bootstrap_nodes`（互換）
  - `bootstrap_hints`（`node_id|relay=...|addr=...` 優先）
  - `relay_urls`（新規、配列）
- `cn-user-api /v1/bootstrap/nodes` は `bootstrap_hints` を優先配布し、互換で `bootstrap_nodes` も残す。
- `cn-bootstrap` の 39000 `endpoints.p2p` に relay hint を格納可能にする。

### 6.3 Gossip bootstrap peer 層

- `cn-relay` の topic 同期ループに以下を追加する。
  - topic 追加時: bootstrap hints から peer seed を解決し `subscribe(topic, peers)` 実行
  - topic 維持中: 一定間隔で join 状態確認、未収束なら peer seed を更新して再試行
  - topic 削除時: sender/task を確実に停止

## 7. 実装フェーズ

### Phase 0: 準備

- `cn-iroh-relay` crate（または外部 `iroh-relay` ラッパ）追加。
- docker-compose と `.env.example` に設定を追加。

### Phase 1: 接続確立基盤

- `kukuri-tauri` endpoint の relay 設定を custom relay 対応へ拡張。
- `cn-relay` 側 endpoint も同様に custom relay 設定を受け取れるようにする。
- 既存設定が無い場合の後方互換（現行挙動維持）を実装。

### Phase 2: bootstrap hint 契約拡張

- `/v1/p2p/info` と `/v1/bootstrap/nodes` のレスポンスを拡張。
- `node_id|relay=...|addr=...` 形式を標準形式として定義。
- `bootstrap_config` / `parse_peer_hint` との整合テストを追加。

### Phase 3: 動的トピック参加

- `cn-relay` の topic join を「peer seed 付き subscribe」に変更。
- bootstrap hint 更新時の再 join 制御を追加。
- join 収束と再試行のメトリクスを追加。

### Phase 4: E2E 経路統一

- E2E で bridge 直注入を禁止し、必ず実機相当経路を通す。
  - Community Node bootstrap API 取得
  - relay hint 反映
  - topic join
- `community node bootstrap/relay 経由で peer 間通信成立` を必須シナリオ化する。

### Phase 5: 段階ロールアウト

- canary 環境で IPv4/IPv6 混在試験を実施。
- 失敗率閾値超過時は `RelayMode::Default` へフォールバック可能な feature flag を残す。

## 8. 受け入れ基準（DoD）

1. `cn-relay` が bootstrap peer として topic 動的参加し、対象 topic の gossip join が安定する。
2. `cn-iroh-relay` を使った接続確立経路で、IPv6 直結不可環境でも peer 通信が成立する。
3. `cn-user-api` が relay hint を含む bootstrap 情報を返し、`kukuri-tauri` がそのまま適用できる。
4. E2E が実機同等経路で green になる。
5. 既存ノード設定（relay hint なし）でも後方互換で起動可能。

## 9. テスト計画

- 単体:
  - hint 文字列パース（relay-only / relay+addr / 互換形式）
  - config 正規化（重複 node_id の置換）
- 統合:
  - `cn-relay` topic 動的参加・再 join
  - `cn-user-api` bootstrap payload 契約
- E2E:
  - `community node bootstrap/relay` 経由での peer 接続成立
  - IPv6 優先条件での relay fallback 成立

## 10. リスクと対策

- リスク: `cn-relay` と `cn-iroh-relay` の役割混同による運用事故  
  対策: 設定キー・監視メトリクス・Runbook をサービス単位で明確化する。

- リスク: relay URL 不整合（WS URL と HTTP URL 混在）  
  対策: `relay_public_url`（Nostr）と `iroh_relay_urls`（接続確立）を分離し、バリデーションを追加する。

- リスク: E2E が実機差分を再導入  
  対策: bridge 経由の peer 直注入を禁止し、API経路のみ許可するテストガードを追加する。

## 11. 参照

- iroh docs: Gossip broadcast  
  https://docs.iroh.computer/connecting/gossip
- iroh docs: Custom relays  
  https://docs.iroh.computer/connecting/custom-relays
- iroh docs: Dedicated infrastructure  
  https://docs.iroh.computer/deployment/dedicated-infrastructure
