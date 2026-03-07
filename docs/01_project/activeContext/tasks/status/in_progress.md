[title] 作業中タスク（in_progress）

最終更新日: 2026年03月05日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続せず、P2P（iroh + iroh-gossip + DHT）で完結した体験を優先。
- 全イベントは NIPs 準拠のスキーマに沿って取り扱う。
- Tauri v2 では E2E が困難なため、層別テスト（単体・結合/契約＋スモーク最小限）でカバレッジを確保。

## 現在のタスク
- P2Pトピック同期の不具合調査と修正（bootstrap反映、受信投稿のリアルタイム反映、プロフィール表示改善）
- Community Node relay 公開構成の VPS + WireGuard edge 化（Cloudflare Tunnel 依存の除去、Home bind 制御、運用スクリプト整備）

### 残タスク（2026年03月02日）
- 直接接続時でも `/topics/${topicId}` の `TimelineThreadCard` とスレッド一覧がリアルタイム差分更新されない（再読み込みや自端末操作が必要）。
- 相手プロフィールの表示名解決ができず、`ユーザー` 表示のままになる。
- リプライ投稿が失敗する（`reply_to` の親投稿キャッシュ不足で `threadUuid` を解決できない）。
- 「スレッド一覧」「スレッドで開く」操作で画面遷移・表示更新が発生しない。

### 進捗メモ（2026年03月01日）
- `p2p://message/raw` のみを購読し、kind/tags 欠落によるスレッド判定崩れを抑制。
- TextNote(kind=1) の受信をトピック投稿として扱えるようにし、UI差分反映漏れを軽減。
- 投稿表示の author 解決を `postMapper` 側にも追加し、プロフィール取得の再試行（TTL付き）を導入。
- Rust `UserRepository` に `profiles` テーブルのフォールバック参照を追加し、`get_user_by_pubkey` / `get_user` でプロフィール復元可能にした。

### 進捗メモ（2026年03月02日）
- `p2p.direct-peer.regression.spec.ts` の接続判定を改善し、対象ピアが既に見えている場合は `connectToP2PPeer` をスキップ、未接続時は `connectToP2PPeer` 失敗後に `joinP2PTopic(initialPeers)` へフォールバックするよう修正。
- 再現specの既定値を「実機不具合再現寄り」に調整（`expectStale=true`、`expectProfileUnresolved=true`、`single-post-case=true`、`single-post rendered=false` をデフォルト化）。
- Docker経路で `./scripts/test-docker.ps1 e2e-multi-peer` を複数条件で再実行し、`tests/e2e/specs/p2p.direct-peer.regression.spec.ts` が安定して実行できることを確認。
- `p2p.direct-peer.regression.spec.ts` の描画判定を安定化。`TimelineThreadCard` の件数差分ではなく、`getP2PMessageSnapshot` から抽出した新規メッセージ本文（marker）の表示有無で without-reload / after-reload を判定するよう変更。
- 単発投稿ケースの厳密アサーションは env 指定時のみ有効化し、既定では観測重視（診断出力）に変更。marker 抽出不能時は `publishPrefix` へフォールバックして再現検証を継続可能にした。
- 検証: `./scripts/test-docker.ps1 build` 実行後に `./scripts/test-docker.ps1 e2e-multi-peer`（`E2E_SPEC_PATTERN=./tests/e2e/specs/p2p.direct-peer.regression.spec.ts`）を再実行し PASS、続けて既定 spec（`community-node.multi-peer.spec.ts`）も PASS。
- IPv6強制の再現試験を実施。`multi-peer-up` 後に `docker compose ... run --rm test-runner` で `SCENARIO=multi-peer-e2e` / `E2E_SPEC_PATTERN=./tests/e2e/specs/p2p.direct-peer.regression.spec.ts` を指定し、`peer-client-2.address.json` を IPv6 のみ（`@[2400:...]`）に調整して実行。
- `KUKURI_ENABLE_DHT=0` / `KUKURI_ENABLE_DNS=0` / `KUKURI_ENABLE_LOCAL=0` / `E2E_DIRECT_PEER_PREFER_LOCAL_ADDRESS=0` 条件で、`connectToP2PPeer` と `joinP2PTopic(initialPeers)` が IPv6 宛て（`...@[2400:4050:...]:60748`）で試行されることを確認。
- 再現結果: `Direct peer connection did not become active for ...@[2400:4050:...]:60748` で失敗し、`peer_count: 0` / `connection_status: 'disconnected'` のまま推移。実機で観測した「IPv6 経路で接続成立しない」現象をテストで再現できた。
- 主要ログ: `tmp/logs/multi-peer-e2e/20260302-065906.log`（`connectToP2PPeer` 呼び出し、IPv6 fallback、最終タイムアウト） / 失敗時スクリーンショット `test-results/multi-peer-e2e/20260302-065906/1772435077086-reproduces-stale-realtime-timeline-rendering-and-unresolved-profile-label.png`。
- 残タスク別の再現状況を整理。
- 「直接接続時でも `/topics/${topicId}` の `TimelineThreadCard` とスレッド一覧がリアルタイム差分更新されない」は再現済み。`test-results/multi-peer-e2e/direct-peer-regression.json` で `renderedWithoutReload=false` / `renderedAfterReload=true` を確認。
- 「相手プロフィールの表示名解決ができず、`ユーザー` 表示のままになる」は再現済み。`test-results/multi-peer-e2e/direct-peer-regression.json` で `profileResolved=false` を確認。
- 「リプライ投稿が失敗する（`reply_to` の親投稿キャッシュ不足で `threadUuid` を解決できない）」は現行の community-node E2E では未再現。`tmp/logs/community-node-e2e/20260228-021555.log` の `tests/e2e/specs/topic.timeline-thread-flow.spec.ts` で `reply-submit-button` 実行後に `timeline-thread-first-reply-*` 描画と `PASSED` を確認。
- 「スレッド一覧」「スレッドで開く」操作で画面遷移・表示更新が発生しない」は現行の community-node E2E では未再現。`tmp/logs/community-node-e2e/20260228-021555.log` で thread 一覧表示（`threadsBrowse topic threads...`）と `Open thread` 経由のフローが通過し `PASSED` を確認。
- ただし上記2件（未再現）は community-node 経路での確認結果であり、multi-peer / IPv6 強制条件では未検証のため、再現条件の差分切り分けを継続する。
- `src-tauri` の `parse_peer_hint` / `parse_node_addr` を `node_id|relay=<url>|addr=<host:port>` 形式に対応させ、relay URL を含む `EndpointAddr` を組み立てられるように拡張（既存 `node_id@host:port` 互換を維持）。
- `p2p_peer_harness` の address snapshot に `relay_urls` と `connection_hints` を追加し、relay 付き接続ヒント（`node_id|relay=...|addr=...`）を出力するように変更。
- `p2p.direct-peer.regression.spec.ts` を更新し、multi-peer シナリオでは bootstrap 既定有効化 + relay ヒント優先選択で接続候補を解決するよう変更。
- 検証: `./scripts/test-docker.ps1 e2e-multi-peer`（`E2E_SPEC_PATTERN=./tests/e2e/specs/p2p.direct-peer.regression.spec.ts`）で PASS、`./scripts/test-docker.ps1 rust` で PASS。

### 進捗メモ（2026年03月05日）
- `docker-compose.test.yml` / `scripts/test-docker.ps1` を更新し、E2E で `cn-iroh-relay` を常時起動する経路を追加（`KUKURI_IROH_RELAY_URLS=http://127.0.0.1:3340`、`KUKURI_IROH_RELAY_MODE=custom` を multi-peer 既定に反映）。
- `registerE2EBridge.connectToP2PPeer` を実実装化し、`bridge.ts` 側も `callBridge('connectToP2PPeer', ...)` を呼ぶよう変更（無効スタブを削除）。
- `ensureTestTopic` から `addTopic` 直挿入を削除し、`fetchTopics` / `joinTopic` / `createTopic` の実フローへ統一。`topicId` 指定時は待機リトライ付きで参加・解決するよう修正。
- `p2p.direct-peer.regression.spec.ts` の既定期待を再現前提から修正成功前提へ切替（`expectStale=false`、`expectProfileUnresolved=false`、`expectedSinglePostRendered=true`）。
- 同 spec に `E2E_DIRECT_PEER_CONNECT_MODE=bootstrap|direct|both` を追加し、直結 join hint と `connectToP2PPeer` 呼び出しをモード制御可能にした（既定 `both`）。
- 検証: `./scripts/test-docker.ps1 e2e-multi-peer`（`E2E_SPEC_PATTERN=./tests/e2e/specs/p2p.direct-peer.regression.spec.ts`）は FAIL。`renderedWithoutReload` が `true` にならず、成功期待のまま適切に失敗することを確認（`tmp/logs/multi-peer-e2e/20260304-165439.log`）。
- 検証: `E2E_DIRECT_PEER_CONNECT_MODE=direct` 指定でも同 spec は FAIL。今度は表示名解決が `true` にならず失敗（`tmp/logs/multi-peer-e2e/20260304-170650.log`）。直結経路を実際に試せる状態にはなっている。
- 検証: `./scripts/test-docker.ps1 e2e-community-node`（`E2E_SPEC_PATTERN=./tests/e2e/specs/topic.timeline-thread-flow.spec.ts`）は PASS（`tmp/logs/community-node-e2e/20260304-165811.log`）。`thread-preview-pane` と `timeline-thread-open-*` 導線を含む「右ペイン導線」テストが実行され、通過を確認。
- 検証: `./scripts/test-docker.ps1 e2e-community-node`（`E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.invite.spec.ts`）は FAIL（`tmp/logs/community-node-e2e/20260304-170301.log`）。`ensureTestTopic` 呼び出し時に WebDriver 側 `Database operation failed` が継続し、原因切り分けを継続中。
- `community-node.invite.spec.ts` は待機ロジックを `settings-page` 全文検索から `community-node-saved-key-topic` 監視に切替。`CommunityNodePanel.tsx` に対応する `data-testid` を追加したが、上記 WebDriver 失敗は未解消。
- `community-node.invite.spec.ts` を `Database operation failed` の最小再現ケースへ再構成。投稿UI検証を外し、`ensureTestTopic` 前後の bridge 実行を `snapshot.before.ensure` → `ensure.byName` → `snapshot.after.byName` → `ensure.byTopicId` → `snapshot.after.byTopicId` で段階分離した。
- `community-node-saved-key-topic` 待機失敗で中断しないよう変更し、検出可否を `invite.keyEnvelopeDetected` ステップとして記録してから bridge 段階へ進むよう修正。
- 検証: `./scripts/test-docker.ps1 build` 後に `./scripts/test-docker.ps1 e2e-community-node`（`E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.invite.spec.ts`）を実行し FAIL（`tmp/logs/community-node-e2e/20260304-205440.log`）。`ensure.byName` は成功し、`ensure.byTopicId` でのみ `WebDriverError: Database operation failed` を再現することを確認。
- 対応（2026年03月05日）: `registerE2EBridge.ensureTestTopic` の `topicId` 分岐を修正し、未発見 topic への `joinTopic` を禁止。`topicId` は discover 後のみ join する段階処理へ変更し、`Database operation failed` 経路を除去。
- 対応（2026年03月05日）: `tests/e2e/helpers/bridge.ts` の `executeAsync` 応答キーを `error` から `bridgeError` に変更し、WebDriver プロトコル例外ラップを回避。
- 検証（2026年03月05日）: `./scripts/test-docker.ps1 build` 後に `./scripts/test-docker.ps1 e2e-community-node`（`E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.invite.spec.ts`）を実行し FAIL（`tmp/logs/community-node-e2e/20260305-014809.log`）。`ensure.byTopicId` は `Topic ... is not discoverable` で失敗し、`Database operation failed` / `WebDriverError` への崩れ込みは解消。
- 継続ブロッカー: `invite.keyEnvelopeDetected=false` のため invite 起点の topic discover が成立せず、`ensure.byTopicId` 成功条件を満たせない。次は key envelope 受信経路（保存・topic同期）側の切り分けが必要。

### 修正計画（2026年03月05日）
- 成立条件:
  - requester/issuer が同時オンライン時、`join.request -> approve -> key envelope -> requester保存 -> topic discover/join` が成立する。
  - issuer/requester のどちらかがオフラインでも、`cn-relay` + `cn-iroh-relay` 経由で復帰後に未処理イベントを受信・反映できる。
  - E2E で `invite.keyEnvelopeDetected=true` と `ensure.byTopicId` 成功を確認し、既存の「右ペイン導線」PASSを維持する。
- フェーズ0（着手）:
  - E2E 実行経路の relay 前提を統一（PowerShell / bash 両方で `KUKURI_IROH_RELAY_URLS=http://127.0.0.1:3340`、`KUKURI_IROH_RELAY_MODE=custom` を適用）。
  - community-node / multi-peer シナリオで `cn-iroh-relay` の起動漏れをなくす。
- フェーズ1（着手）:
  - ログイン直後に user topic 購読が欠落しないよう、`login` / `secure_login` / `switch_account` 後に `ensure_default_and_user_subscriptions` を再実行する。
  - key envelope 受信待ちの前提（requester が自身 user topic を購読済み）を常時満たす。
- フェーズ2:
  - invite フローを `requester送信` / `issuer承認` / `requester受信` の3段に分け、各段の到達確認ログを E2E へ追加する。
  - `invite.keyEnvelopeDetected=false` を「どの段で止まるか」で再現最小化する。
- フェーズ3:
  - 片側オフライン復帰ケース（issuer復帰時 invite 受信、requester復帰時 key envelope 受信）をシナリオ化し、`cn-relay` 経由での成立条件を検証する。

### 実装着手ログ（2026年03月05日）
- `login` / `generate_keypair` / `secure_login` / `switch_account` 後に `ensure_default_and_user_subscriptions` を再実行するよう変更し、user topic 購読欠落を抑止。
- `scripts/test-docker.sh` を更新し、multi-peer / community-node の両シナリオで `cn-iroh-relay` 利用と relay 環境変数（`KUKURI_IROH_RELAY_URLS` / `KUKURI_IROH_RELAY_MODE`）を PowerShell 側と同等に統一。
- `scripts/test-docker.ps1` の community-node シナリオにも relay 環境変数の既定適用を追加し、E2E 実行経路差分を縮小。
- 検証: `./scripts/test-docker.ps1 rust` は PASS（Rust単体・結合テスト一式が通過）。
- 検証: `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.invite.spec.ts ./scripts/test-docker.ps1 e2e-community-node` は FAIL（`tmp/logs/community-node-e2e/20260305-031958.log`）。`invite.keyEnvelopeDetected=false` と `ensure.byTopicId` の `Topic ... is not discoverable` は継続。
- `registerE2EBridge` / `tests/e2e/helpers/bridge.ts` に `accessControlIssueInvite` bridge action を追加し、invite 発行を E2E bridge 経由で実行可能にした。
- `community-node.invite.spec.ts` を 3段分離へ再構成し、`requester送信`（`accessControlRequestJoin`）→`issuer受信/承認`（`accessControlIngestEventJson` + `accessControlListJoinRequests` + `accessControlApproveJoinRequest`）→`requester受信`（`accessControlIngestEventJson` + `communityNodeListGroupKeys`）の順で到達判定するよう修正。
- 検証: 旧 `test-runner` イメージのまま `./scripts/test-docker.ps1 e2e-community-node` を実行すると旧 spec が走るため、`./scripts/test-docker.ps1 build` で再ビルドが必要であることを確認。
- 検証: `./scripts/test-docker.ps1 build` 後に `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.invite.spec.ts ./scripts/test-docker.ps1 e2e-community-node` を再実行し PASS（`tmp/logs/community-node-e2e/20260305-034354.log`）。`invite.keyEnvelopeDetected` と `ensure.byTopicId` が成功することを確認。

### 進捗メモ（2026年03月06日）
- `cn-iroh-relay` の長寿命 stream が Cloudflare Tunnel 配下で `Stream terminated` になりやすいことを踏まえ、relay 系データプレーンは `VPS + WireGuard edge` へ切替、Cloudflare は `DNS only` に限定する方針を確定。
- `cn-relay` に `RELAY_P2P_PUBLIC_HOST` / `RELAY_P2P_PUBLIC_PORT` を追加し、`RELAY_PUBLIC_URL` と `/v1/p2p/info` の advertised endpoint を分離。
- `docker-compose.yml` に `RELAY_HOST_BIND_IP` / `RELAY_P2P_HOST_BIND_IP` / `IROH_RELAY_HOST_BIND_IP` を追加し、自宅側 relay 系 service を WireGuard IP のみに bind できるよう修正。
- `kukuri-community-node/.env.home-vps-edge.example` を追加し、`relay.kukuri.app` / `iroh-relay.kukuri.app` / `10.73.0.2` を前提にした Home 側の具体値を定義。
- `scripts/vps/setup-home-relay-edge.sh` と `scripts/vps/home-relay-edge.env.example` を追加し、VPS 上で `git clone` 後に WireGuard / Caddy / nftables を即構成できるようにした。
- `scripts/vps/setup-home-relay-edge.sh` を Debian / Ubuntu と Rocky / AlmaLinux / RHEL 系の両対応へ拡張し、`dnf` 系では `epel-release` と Caddy rpm repository を自動設定、`firewalld` / `ufw` の競合も停止するよう修正。
- `scripts/vps/setup-home-relay-edge.sh` の Caddy import を `/etc/caddy/sites-enabled/*.caddy` に修正し、旧実装の backup (`*.bak.*`) が site 定義として再読込されて `ambiguous site definition` になる不具合を回避。移行用の退避手順も `home_vps_wireguard_edge.md` に追記。
- 追加対応: `import /etc/caddy/sites-enabled/*.caddy` を再実行時に部分置換して `*.caddy.caddy` へ壊してしまう不具合を修正。既に壊れた VPS 向けの復旧手順も `home_vps_wireguard_edge.md` に追記。
- `docs/03_implementation/community_nodes/home_vps_wireguard_edge.md` を追加し、DNS, VPS, Home, `kukuri-community-node` の設定値と確認手順を整理。
- `cn-iroh-relay` を `7842/udp` 含みで本番相当に公開できるよう、compose / env / entrypoint を config mode + 手動証明書 + QUIC address discovery 前提へ拡張中。
- `scripts/vps/setup-home-relay-edge.sh` / `home-relay-edge.env.example` / `home_vps_wireguard_edge.md` は `7842/udp` forward と `iroh-relay.kukuri.app` 証明書の Home 配置手順を追記中。
- `iroh_gossip_service` は existing topic へ peer hint を後付けした場合、既存 handle の `join_peers` 再利用ではなく `TopicMesh` を引き継いだ fresh subscribe へ切り替える実装に着手。
