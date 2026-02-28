# Docker ベース Peer クライアント設計・実装計画（ローカル複数ピア検証）

作成日: 2026年02月28日  
最終更新日: 2026年02月28日  
ステータス: Draft（実装前）

## 1. 目的

- ローカル環境で「複数 Peer が同時接続したときの挙動」を再現可能にする。
- 既存の `kukuri-tauri` 実装と E2E 導線を最大限再利用し、新規実装範囲を最小化する。
- Windows 開発環境でも `./scripts/test-docker.ps1` 経由のみで再現できる状態を作る。
- 自動 E2E だけでなく、開発者が手動操作で接続・投稿・観測を行える運用導線を持つ。

## 2. 前提と再利用方針

### 2.1 再利用対象（確認済み）

| 資産 | 現状 | 再利用方針 |
| --- | --- | --- |
| `kukuri-tauri/src-tauri` の P2P 実装 | アプリ本体で稼働中、`p2p_*` テスト資産あり | Headless Peer 実装のコアとして再利用 |
| `kukuri-tauri/tests/e2e/wdio.desktop.ts` | Docker E2E 実行の標準導線 | 新シナリオでもそのまま利用 |
| `community-node.cn-cli-propagation.spec.ts` | 外部 publish -> Tauri 反映を検証済み | 複数 Peer 検証のベースシナリオとして拡張 |
| `scripts/test-docker.ps1/.sh` | `e2e` / `e2e-community-node` 導線あり | `e2e-multi-peer` を追加して統合 |
| `docker-compose.test.yml` | `test-runner` / `p2p-bootstrap` / `community-node-*` 定義済み | Peer コンテナサービスを追加 |

### 2.2 関連ドキュメント最終更新日（確認値）

- `docs/01_project/activeContext/tasks/priority/critical.md`: 2026年01月23日
- `docs/01_project/activeContext/tasks/status/in_progress.md`: 2026年02月26日
- `docs/01_project/activeContext/tasks/README.md`: 2025年08月20日
- `docs/01_project/activeContext/build_e2e_test.md`: 2025年11月16日（本文記載）
- `docs/01_project/progressReports/2026-02-27_community_node_bootstrap_runtime_fallback.md`: 2026年02月27日

## 3. 要件

### 3.1 機能要件

1. Docker で Peer を複数（例: 3-5）起動できる。
2. 全 Peer が同一 topic に join し、publish/receive を実行できる。
3. `kukuri-tauri` クライアントから Peer 接続数と伝搬結果を E2E で観測できる。
4. 失敗時に Peer ごとのログとサマリ JSON を保存できる。
5. test-runner を使わずに、Peer 群のみ起動して手動操作で同等の観測ができる。

### 3.2 非機能要件

1. Windows は必ず `./scripts/test-docker.ps1` 経由で実行できる。
2. 既存 `e2e-community-node` の運用を壊さない（後方互換）。
3. CI で再現できる（少なくとも Linux + Docker で再現）。
4. 手動操作時も自動テストと同じログ出力先（`tmp/logs` / `test-results`）を使う。

## 4. 設計方針（採用案）

### 4.1 採用アプローチ

- **採用**: `kukuri-tauri` の Rust P2P 実装を使った Headless Peer バイナリを追加し、Docker コンテナとして複数起動する。
- **補助利用**: 既存 `cn-cli p2p publish` は比較検証/フォールバック用途として維持する。

### 4.2 非採用アプローチ

- Tauri GUI を Peer 数分コンテナ起動する方式  
  理由: WebView・ドライバ依存が重く、複数台検証用途としてオーバーコスト。

## 5. アーキテクチャ案

```text
scripts/test-docker(e2e-multi-peer)
  ├─ p2p-bootstrap (既存)
  ├─ community-node-user-api / bootstrap (既存、必要時)
  ├─ peer-client x N (新規: headless)
  └─ test-runner (既存: WDIO + Tauri)
        └─ E2E bridge 経由で P2P 状態/受信内容を検証

手動操作モード
  ├─ p2p-bootstrap (既存)
  ├─ peer-client x N (新規: headless)
  └─ 開発者の Tauri アプリ（手動起動）
        └─ 設定画面/トピック画面/ネットワーク状態画面で目視確認
```

### 5.1 Peer コンテナ仕様（新規）

- 実体: `kukuri-tauri/src-tauri/src/bin/p2p_peer_harness.rs`（新規）
- 実行モード:
  - `listener`: topic join + 受信カウント
  - `publisher`: topic join + 定期 publish
  - `echo`: 受信を再送してメッシュ性を確認
- 出力:
  - 標準ログ（`tmp/logs/multi-peer/*.log`）
  - 実行サマリ JSON（`test-results/multi-peer/*.json`）

### 5.2 E2E 連携仕様

- ベース: `community-node.cn-cli-propagation.spec.ts`
- 追加検証:
  - `getP2PStatus().peers` が閾値以上
  - `getP2PMessageSnapshot()` と `getPostStoreSnapshot()` で外部 Peer 投稿を確認
  - 必要に応じて topic ページ描画まで確認

### 5.3 手動操作モード仕様

- 前提: Peer 群と bootstrap を Docker で常駐起動し、Tauri は開発者が手動起動する。
- 目視観測ポイント:
  - RelayStatus/NetworkStatus の接続状態
  - topic 画面での外部投稿反映
  - 接続切断/再接続時の挙動（Peer コンテナ停止・再起動）
- 操作補助:
  - `cn-cli p2p publish` を手動実行して、任意 payload を topic に注入できるようにする。

## 6. 実装計画

### Phase 0: 仕様固定（0.5日）

- Peer CLI 引数、ログ出力フォーマット、成果物パスを確定。
- `SCENARIO=multi-peer-e2e` の命名を確定。

### Phase 1: Headless Peer 実装（1-2日）

- 追加:
  - `kukuri-tauri/src-tauri/src/bin/p2p_peer_harness.rs`
- 再利用:
  - `application/shared/tests/p2p/*` の待機・hint 解決ロジック
  - `application/services/p2p_service` の join/broadcast/status API

### Phase 2: Docker 導線追加（1日）

- 追加:
  - `scripts/docker/run-multi-peer-e2e.sh`
  - `scripts/docker/run-multi-peer-manual.sh`
- 更新:
  - `docker-compose.test.yml`（`peer-client` サービス追加）
  - `scripts/docker/run-smoke-tests.sh`（`SCENARIO=multi-peer-e2e` 分岐）
  - `scripts/test-docker.ps1` / `scripts/test-docker.sh`（`e2e-multi-peer` と `multi-peer-up|down|status` 追加）

### Phase 3: E2E シナリオ追加（1-2日）

- 追加:
  - `kukuri-tauri/tests/e2e/specs/community-node.multi-peer.spec.ts`
- 方針:
  - 既存 `community-node.cn-cli-propagation` のアサーションを共通化して再利用。
  - `E2E_MULTI_PEER_COUNT` を環境変数で受け取り、期待接続数を可変にする。

### Phase 4: CI 統合（0.5-1日）

- `.github/workflows/test.yml` に `desktop-e2e-multi-peer`（または既存 `desktop-e2e` 拡張）を追加。
- artefact:
  - `tmp/logs/multi-peer-e2e`
  - `test-results/multi-peer-e2e`

### Phase 5: 手動運用導線整備（0.5日）

- `docs/03_implementation/p2p_mainline_runbook.md` に「複数 Peer 手動検証手順」を追記。
- `scripts/test-docker.ps1/.sh` の help に手動操作コマンド例を追加。
- 手動操作用ログ採取テンプレート（開始/停止/確認）を `tmp/logs/multi-peer-manual/` へ統一。

## 7. 検証計画

### 7.1 自動実行（Windows）

```powershell
./scripts/test-docker.ps1 e2e-multi-peer
```

### 7.2 自動実行（Linux/macOS）

```bash
./scripts/test-docker.sh e2e-multi-peer
```

### 7.3 手動操作（共通）

1. Peer 群起動
   - `docker compose -f docker-compose.test.yml up -d p2p-bootstrap peer-client-1 peer-client-2 peer-client-3`
2. Tauri クライアントを手動起動
   - `pnpm --dir kukuri-tauri tauri dev`
3. 接続確認
   - 設定画面で bootstrap を適用し、NetworkStatus で peer 接続数を確認
4. 手動 publish
   - `docker compose -f docker-compose.test.yml run --rm --entrypoint cn community-node-user-api p2p publish --topic <topic_id> --content "<message>" --repeat 1`
5. 受信確認
   - topic 画面に投稿が反映されることを確認し、`docker compose ... logs peer-client-*` で受信ログを照合
6. 停止/クリーンアップ
   - `docker compose -f docker-compose.test.yml rm -sf peer-client-1 peer-client-2 peer-client-3 p2p-bootstrap`

### 7.4 受け入れ基準

1. Peer 3台以上で `community-node.multi-peer` シナリオが安定して pass する。
2. Tauri 側で `connection_status=connected` かつ期待 Peer 数以上が観測できる。
3. 外部 Peer 由来イベントが topic 画面に表示される。
4. 失敗時に Peer 単位のログと JSON サマリが必ず残る。
5. 手動操作手順（7.3）で同等の接続・伝搬確認を再現できる。

## 8. リスクと対策

| リスク | 影響 | 対策 |
| --- | --- | --- |
| UDP/port 競合で接続不安定 | E2E 失敗率上昇 | bind `:0` 運用、起動待機リトライ、Peer 起動順固定 |
| 非同期伝搬の揺らぎ | たまに false negative | wait/retry + topic ページテキスト確認の二段検証 |
| ログ不足で調査不能 | MTTR 悪化 | JSON サマリ出力を必須化し、Peer ごとに保存 |
| CI 時間増加 | パイプライン遅延 | Peer 数を `3` 既定、Nightly で `5` に拡張 |
| 手動手順の属人化 | 再現性低下 | `test-docker` サブコマンド化と Runbook 手順固定で差異を抑制 |

## 9. 実装順序の結論

1. `kukuri-tauri` ベースの Headless Peer バイナリを先に作る。  
2. 既存 Docker/E2E 導線へ `multi-peer-e2e` シナリオを追加する。  
3. `community-node.cn-cli-propagation` で使っている検証資産を共通化し、新シナリオへ移植する。  
4. 手動操作導線（`multi-peer-up|down|status` と Runbook）を整備して、E2E 非依存でも検証可能にする。  

この順序なら、既存資産の再利用率を高く保ちつつ、最小変更で「複数 Peer 接続挙動」を継続検証できる。
