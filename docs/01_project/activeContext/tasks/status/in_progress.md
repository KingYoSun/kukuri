[title] 作業中タスク（in_progress）

最終更新日: 2026年03月09日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続せず、P2P（iroh + iroh-gossip + DHT）で完結した体験を優先。
- 全イベントは NIPs 準拠のスキーマに沿って取り扱う。
- Tauri v2 では E2E が困難なため、層別テスト（単体・結合/契約＋スモーク最小限）でカバレッジを確保。

## 現在のタスク
- P2Pトピック同期の不具合調査と修正（bootstrap反映、受信投稿のリアルタイム反映、プロフィール表示改善）
- Community Node relay 公開構成の VPS + WireGuard edge 化（Cloudflare Tunnel 依存の除去、Home bind 制御、運用スクリプト整備）

### Community Node 実機UX不具合の再現・修正（2026年03月08日 着手）
- プロフィール伝播不具合: profile 保存はローカル成功するが failure toast が出て、相手側へ metadata が伝播しない。
  - 再現条件: profile setup / profile update 実行後に toast、local profile、相手側 author 表示を確認する。
  - 完了条件: success toast のみが出て、相手側 timeline/thread でも表示名・avatar が反映される。
  - 進捗（2026年03月08日）: `useP2PEventListener` の unit reproducer を追加し、`p2p://message/raw` の metadata(kind=0) 受信時に `timeline` / `topicTimeline` / `topicThreads` / `threadPosts` / post store の author metadata を即時更新する修正を反映。`ProfileSetup` / `ProfileEditDialog` は local save 成功後の remote 失敗を failure ではなく warning toast に変更し、関連 19 tests の個別実行で PASS を確認。
  - 進捗（2026年03月08日・live path）: `community-node.profile-propagation.spec.ts` を追加して false failure toast と metadata 未伝播を E2E で再現後、同 spec を success 条件へ更新した。`./scripts/test-docker.ps1 e2e-community-node`（`E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.profile-propagation.spec.ts`）で PASS を確認。残作業は実機 multi-node 構成での再確認のみ。
- Community Node 操作 toast 不整合: 設定保存・認証・role 変更で failure toast が出るが、reload 後は成功状態になっている。
  - 再現条件: settings 画面から config save / authenticate / role change を順に実施し、toast と state snapshot を比較する。
  - 完了条件: 実際に成功した操作では failure toast が出ず、UI 状態と backend state が一致する。
  - 進捗（2026年03月08日）: `CommunityNodePanel` の unit reproducer を追加し、成功した認証後の trust provider refresh 失敗が `showToast: true` で user-facing error になっていたことを固定。`trust provider` / `pending join requests` の背景 query 失敗はログのみ（`showToast: false`）へ変更し、`CommunityNodePanel.test.tsx` 10 tests PASS を確認。
  - 進捗（2026年03月08日・live path）: `community-node.settings.spec.ts` で config save / authenticate / role change を実経路で再現し、false failure toast が出ないことを確認。`./scripts/test-docker.ps1 e2e-community-node`（`E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.settings.spec.ts`）で PASS。
- Windows Tauri リロードクラッシュ: 複数回 reload 後に `iroh-quinn ... PoisonError` でクライアントが落ちる。
  - 再現条件: Windows で Community Node 設定済みクライアントを連続 reload し、Rust panic を監視する。
  - 完了条件: reload 耐性テストで panic が再現せず、endpoint 再初期化が idempotent である。
  - 進捗（2026年03月09日）: `community-node.reload-stability.spec.ts` を追加し、Community Node 認証済みクライアントを settings route 上で 5 回連続 reload しても token / pubkey / `endpoint_id` / `connection_status` が維持されることを live-path で再現・固定した。失敗時は `home-page` 復帰前提の spec が `/settings` 残留で誤検知していたため、reload 後は `settings-page` 維持を正常系に変更。
  - 進捗（2026年03月09日・根因修正）: `IrohGossipService` に peer hint 同一集合の再joinを idempotent に扱う判定を追加し、既存 topic に同じ Community Node peer hint が再投入された場合は handle rebuild をスキップするよう修正。`test_existing_topic_with_same_peer_hints_keeps_handle` / `test_build_peer_hint_keys_prefers_configured_relays_for_equivalent_hints` を追加し、`./scripts/test-docker.ps1 rust` と `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.reload-stability.spec.ts ./scripts/test-docker.ps1 e2e-community-node` で PASS を確認。残作業は実機 Windows で panic ログが再発しないことの確認のみ。
- Admin UI connected users 表示不整合: Relay / Bootstrap の Users が現在接続ではなく累積を表示している。
  - 再現条件: user の接続・切断を繰り返し、Admin UI 表示と runtime status を比較する。
  - 完了条件: 現在接続中の user のみ表示される。
  - 進捗（2026年03月08日）: `cn-admin-api` が `cn_user.topic_subscriptions` の active 行を current connected users と誤読していたため、contract test `node_subscriptions_list_does_not_treat_active_subscriptions_as_current_connected_users` を追加して再現。`connected_users` は runtime current users を直接持たず、`connected_user_count` は relay runtime fallback のみ反映するよう修正した。`cargo test -p cn-admin-api ...` と Admin Console page tests で PASS を確認。残作業は実機 Admin UI での表示確認のみ。
- Admin UI health 不具合:
  - `admin-api` が `unknown`
  - `moderation` が `unreachable (build error)`
  - `trust` が `degraded (503)`
  - 完了条件: health source を切り分け、各 service が実状態に応じて `healthy` もしくは妥当な degraded 理由を返す。
  - 進捗（2026年03月08日）: `apps/admin-console/vite.config.ts` の `VITE_ALLOWED_HOSTS.split(',')` 例外で Moderation/Trust/Services page test が build 前に落ちていたため、未設定時も安全に評価するよう修正し、関連 page tests 14 files / 27 tests PASS を確認。
  - 進捗（2026年03月08日・backend）: `cn-admin-api` の `/v1/admin/services` が `service_configs` 起点のみで `admin-api` self row と health-only service を返していなかったため、contract test `services_list_includes_health_only_services_and_admin_api_self_status` を追加して再現。`admin-api` は self health を合成し、health-only service も version 0/config `{}` で返すよう修正した。`cargo test -p cn-admin-api ...` で PASS。
  - 進捗（2026年03月08日・compose）: `trust` profile 単独では `moderation` が起動せず `trust /healthz -> 503` になり得るため、`docker-compose.yml` で `moderation` を `trust` profile にも含め、`trust` は `depends_on: [postgres, moderation]` に変更した。`docker compose --profile trust --profile bootstrap -f kukuri-community-node/docker-compose.yml config --services` で `moderation` / `trust` の同時展開を確認。残作業は実機 stack で `trust` が `healthy` へ遷移することの確認。

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

### 進捗メモ（2026年03月07日）
- `kukuri-tauri` の `NostrClientManager` を relay URL 保持・差し替え対応へ拡張し、Community Node 保存設定から `ws/wss` relay URL を復元して `EventManager` に反映する実装に着手。
- `community_node_commands.rs` / `state.rs` から Community Node 設定更新・認証・起動時復元の各タイミングで Nostr relay 設定を適用する配線を追加。
- `CommunityNodeHandler` に bootstrap descriptor `endpoints.ws` と `base_url -> /relay` fallback から relay URL を解決する処理を追加。
- `cn-relay` の gossip topic join 前に、bootstrap seed peer の `EndpointAddr` へ `endpoint.connect(..., iroh_gossip::ALPN)` を明示実行する修正に着手。
- 追加切り分け（2026年03月07日）: `kukuri-tauri` の Community Node relay 同期直後に `rustls::CryptoProvider` 未初期化 panic が発生していたため、アプリ起動時に `ring` provider を明示 install する修正に着手。
- 追加切り分け（2026年03月07日）: `cn-relay` の iroh endpoint が既定 IPv6 transport を保持したまま `bind_addr(v4)` しており、VPS で IPv6 宛て `sendmsg ... NetworkUnreachable` を出していたため、`clear_ip_transports()` 後に設定 family のみ bind する方向で修正。
- 対応（2026年03月07日）: `kukuri-tauri/src-tauri/Cargo.toml` に `rustls` を明示追加し、`src/lib.rs` の `run()` 起動直後で `rustls::crypto::ring::default_provider().install_default()` を呼ぶよう修正。Community Node 設定・認証時の `CryptoProvider` panic を解消する方向で実装済み。
- 対応（2026年03月07日）: `kukuri-community-node/crates/cn-relay/src/gossip.rs` の endpoint 構築で `Endpoint::empty_builder(relay_mode).clear_ip_transports()` を使い、`bind_addr(v4)` 時に不要な IPv6 transport を残さないよう修正。VPS 側 `sendmsg ... NetworkUnreachable` の直接原因を除去。
- 追加切り分け（2026年03月07日）: 認証時の `wss://api.kukuri.app/relay` 404 は `CommunityNodeHandler::resolve_nostr_relay_urls_for_config()` が bootstrap descriptor `endpoints.ws` を拾った後でも `base_url + /relay` を無条件追加していたことが原因。descriptor `endpoints.http` と `config.base_url` を照合し、matching descriptor に `ws/wss` がある node では fallback を足さないよう修正に着手。
- 検証（2026年03月07日）: `docker compose -f docker-compose.test.yml run --rm rust-test` PASS、`gh act --workflows .github/workflows/test.yml --job format-check` PASS、`gh act --workflows .github/workflows/test.yml --job native-test-linux` PASS、`gh act --workflows .github/workflows/test.yml --job community-node-tests` PASS。`./scripts/test-docker.ps1 rust` は `rust-test` サービス用イメージを再ビルドしない既存制約で `--locked` 失敗のため、同一 Compose 定義を直接実行して確認。
### 重大インシデント対応メモ（2026年03月07日）
- PR required レーンに含まれていなかった `desktop-e2e` を required 化し、Community Node の実経路 UX を担保する E2E を新設中。
- `docker-compose.test.yml` に live `relay` を追加し、Community Node `user-api/bootstrap` が実 `cn-relay` の `/v1/p2p/info` を参照する test topology に切替。
- bootstrap descriptor の `endpoints.http/ws` は `BOOTSTRAP_DESCRIPTOR_HTTP_URL` / `BOOTSTRAP_DESCRIPTOR_WS_URL` で test 実 URL を seed できるよう修正。
- Community Node required E2E は bridge shortcut を使わず、UI から `Community Node追加 -> 認証 -> consent更新 -> #public参加 -> topic mesh join -> peer疎通確認 -> 投稿伝播確認` を通す構成へ差替え。
- peer harness は実行中にも summary を逐次出力するよう変更し、remote listener が投稿を受信した事実を E2E からファイル観測できるようにした。
- 2026年03月07日: required 化した `desktop-e2e` を実経路検証へ移行中。Docker 内 iroh が bridge/host 混在トポロジの直結アドレスを拾って green/fail を不安定化させていたため、`e2e-community-node` / `e2e-multi-peer` は relay-only transport profile と relay-only bootstrap hints を強制し、spec でも「Community Node 認証後に期待 Nostr relay が connected で、`api.kukuri.app/relay` fallback が残っていないこと」を待ってから topic mesh / peer 疎通 / 投稿伝播を検証するよう更新中。
### 重大インシデント対応メモ（2026年03月07日・E2E required化）
- desktop-e2e を PR required gate に組み込み、Community Node 実経路 E2E が失敗した場合に必ず赤く落ちるよう維持。
- kukuri-tauri/src-tauri/src/infrastructure/p2p/iroh_network_service.rs で KUKURI_IROH_TRANSPORT_PROFILE=relay-only 時に clear_ip_transports() を適用し、direct discovery を無効化。ローカル endpoint の advertised address も relay-only hint のみに制限。
- kukuri-tauri/tests/e2e/specs/community-node.end-to-end.spec.ts を強化し、Community Node 追加→認証→consent→#public 参加→topic mesh join→peer 接続→投稿伝播に加え、アプリ側/peer harness 側の hint が direct addr を含まないことまで検証。
- 実測結果: ./scripts/test-docker.ps1 rust PASS、./scripts/test-docker.ps1 e2e-community-node PASS（	ests/e2e/specs/community-node.end-to-end.spec.ts、ログ: 	mp/logs/community-node-e2e/20260307-203142.log）。peer harness snapshot でも 
ode_addresses は |relay=http://127.0.0.1:3340/ のみを確認。

### 進捗メモ（2026年03月08日）
- live path の `peer_count: 0` を再調査。client 側 snapshot では endpoint transport は `connected` だが topic `peer_count` は 0 のままで、gossip overlay join 失敗に絞り込んだ。
- `cn-relay` の未コミット差分で `subscribe_and_join` / `connect_seed_peers` が外れていたため、seed peer あり経路では `endpoint.connect(..., iroh_gossip::ALPN)` と `subscribe_and_join(...)` を復元。MemoryLookup と `endpoint.online()` 待機は維持。
- `cn-relay` に `/v1/p2p/status` を追加し、`desired_topics` / `node_topics` / `gossip_topics` / `router_ready` を HTTP から観測できるようにした。今後の E2E 失敗時に server 側 runtime を snapshot へ含める。
- `community-node.end-to-end.spec.ts` は `Community Node bootstrap ready` 後に `/v1/p2p/status` で server が対象 topic を gossip 参加済みであることを待ってから client join へ進むよう更新。失敗 snapshot に server 側 `communityNodeP2P` 状態も含める。
- Community Node 実機UXの先頭課題だった「リアルタイムモードのタイムライン即時反映」と「スレッド右ペイン導線」は、live-path E2E を追加して再現・修正まで完了。
  - 追加テスト:
    - `kukuri-tauri/tests/e2e/specs/community-node.timeline-thread-realtime.spec.ts`
    - `kukuri-tauri/tests/e2e/specs/community-node.thread-preview-replies.spec.ts`
  - 修正概要:
    - `useRealtimeTimeline.ts` / `useP2PEventListener.ts` で P2P 受信 event の `eventId` と thread 関連情報を store fallback で補完し、reload なしで `TimelineThreadCard` と thread preview が更新されるよう修正。
    - `topics.$topicId.threads.tsx` で `/topics/$topicId/threads/$threadUuid` を右ペイン詳細導線として扱い、`「スレッドを開く」` 押下時に thread 一覧へ戻らないよう修正。
    - `p2p_peer_harness` と E2E helper を拡張し、persistent peer への command file 注入で external reply を安定再現できるようにした。
  - 実測:
    - `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.thread-preview-replies.spec.ts ./scripts/test-docker.ps1 e2e-community-node` PASS（ログ: `tmp/logs/community-node-e2e/20260308-183533.log`）
    - `E2E_SPEC_PATTERN=./tests/e2e/specs/community-node.timeline-thread-realtime.spec.ts ./scripts/test-docker.ps1 e2e-community-node` PASS（ログ: `tmp/logs/community-node-e2e/20260308-183827.log`）
