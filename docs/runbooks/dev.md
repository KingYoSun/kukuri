# Development Runbook

## 初回セットアップ
```bash
npx pnpm@10.16.1 install --dir apps/desktop
cargo xtask doctor
```

## 日常コマンド
```bash
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke
cargo xtask cn-check
cargo xtask cn-test
cargo xtask scenario community_node_public_connectivity
```

`cargo xtask check` は workspace lint/test に加えて `apps/desktop/src-tauri` の Tauri backend compile も確認する。

`cargo xtask cn-check` / `cargo xtask cn-test` は `cn-*` server slice の compile/test 用。

- `cargo xtask test` は workspace 全体を通すが、Postgres が必要な `cn-*` integration test は実行しない。
- `cargo xtask cn-test` は `docker-compose.community-node.yml` の `cn-postgres` を自動起動し、`KUKURI_CN_RUN_INTEGRATION_TESTS=1` を付けて contract/integration test を流す。
- `cargo xtask scenario community_node_public_connectivity` も `cn-postgres` を自動起動し、in-process の `cn-user-api` / `cn-relay` / `cn-iroh-relay` を立てて 2 desktop scenario を流す。

## community-node compose
```bash
docker compose -f docker-compose.community-node.yml up --build cn-user-api cn-relay cn-iroh-relay
```

- host port の既定値は `18080` (`cn-user-api`), `18081` (`cn-relay`), `13340` (`cn-iroh-relay`), `55432` (`cn-postgres`)
- compose 内の service 名は `cn-postgres`, `cn-user-api`, `cn-relay`, `cn-iroh-relay`
- public URL を変える場合は `CN_BASE_URL`, `CN_PUBLIC_BASE_URL`, `CN_RELAY_WS_URL`, `CN_IROH_RELAY_URLS` を上書きする

## community-node 検証
```bash
cargo xtask cn-test
cargo xtask scenario community_node_public_connectivity
```

- `cn-test` は `/v1/auth/challenge`, `/v1/auth/verify`, `/v1/consents/status`, `/v1/consents`, `/v1/bootstrap/nodes`, `/v1/p2p/info`, `/relay` の contract を確認する。
- `community_node_public_connectivity` scenario は `config -> auth -> consent -> restart -> post -> reply/thread -> live -> game -> reconnect` を 1 community-node stack + 2 desktops で確認する。
- crate test を直接叩く場合は `KUKURI_CN_RUN_INTEGRATION_TESTS=1` と `COMMUNITY_NODE_DATABASE_URL` を明示する。

## frontend だけ確認する場合
```bash
cd apps/desktop
npx pnpm@10.16.1 dev
npx pnpm@10.16.1 test
npx pnpm@10.16.1 tauri:dev
```

- `pnpm tauri dev` / `pnpm tauri:dev` は loopback の空き port を自動選択し、5173 が使用中なら次の空き port へ退避する。

## Windows 前提
- Windows prerequisites は Tauri 公式手順を使う: <https://v2.tauri.app/start/prerequisites/#windows>
- 初回 Windows cut の対象は `x86_64-pc-windows-msvc` のみ
- installer build は current-user NSIS + WebView2 download bootstrapper を前提にする

## Windows packaging
```powershell
cargo xtask desktop-package
```

- 実行可能なのは Windows host のみ
- 生成物は `apps/desktop/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/` に出る
- `cargo xtask desktop-package` は `src-tauri/tauri.windows.conf.json` を使った Windows bundle config を前提にする

## remote-sync 用の環境変数
```bash
export KUKURI_BIND_ADDR=0.0.0.0:0
export KUKURI_ADVERTISE_HOST=<LANで到達可能なIPまたはホスト名>
export KUKURI_ADVERTISE_PORT=<必要なら固定port>
export KUKURI_INSTANCE=<同一マシンで複数起動する場合の識別子>
export KUKURI_DISABLE_KEYRING=1
export KUKURI_DISCOVERY_MODE=<static_peer|seeded_dht>
export KUKURI_DISCOVERY_SEEDS=<node_id または node_id@host:port をカンマ区切り>
```

- `KUKURI_ADVERTISE_HOST` を設定すると `Your Ticket` はその host を使う。
- `KUKURI_INSTANCE` を設定すると app data dir が分離される。
- `KUKURI_APP_DATA_DIR` を設定すると app data dir を丸ごと上書きできる。
- `KUKURI_DISABLE_KEYRING=1` を設定すると OS keyring を使わず、app data dir 内の fallback file を使う。
- `KUKURI_DISCOVERY_MODE` / `KUKURI_DISCOVERY_SEEDS` を設定すると discovery panel は read-only になり、env が local file より優先される。

PowerShell 例:
```powershell
$env:KUKURI_BIND_ADDR="0.0.0.0:0"
$env:KUKURI_ADVERTISE_HOST="<LANで到達可能なIPまたはホスト名>"
$env:KUKURI_INSTANCE="desktop-a"
```

## Linux / Windows 共通の回帰用手動確認
1. 各端末で `KUKURI_BIND_ADDR=0.0.0.0:0` と `KUKURI_ADVERTISE_HOST` を設定する。
2. 同一マシンで複数起動する場合は `KUKURI_INSTANCE` も別値にする。
3. `npx pnpm@10.16.1 tauri:dev` を起動する。
4. 両方の `Your Ticket` を相互に `Peer Ticket` へ貼って import する。
5. 片方で post し、もう片方の timeline に反映されることを確認する。
6. 片方を再起動しても timeline が維持されることを確認する。
7. どちらかの client を終了し、相手側が polling で `connected: no, peers: 0` に戻ることを確認する。
8. timeline または thread pane の `Reply` ボタンから返信し、相手側の thread に反映されることを確認する。
9. `Add Topic` で 2 つ以上の topic を登録し、切り替えながら各 timeline が維持されることを確認する。
10. peer 接続中に複数 topic へ post し、相手側で各 topic の timeline に反映されることを確認する。
11. tracked topic 一覧の各 topic について `joined / peers / expected / missing / last_received_at / status_detail` が妥当な値になることを確認する。
12. 共通購読 topic を片側で解除し、その topic 行だけ `joined: false / peers: 0` になることを確認する。
13. invalid な `Peer Ticket` を import したときに global `Last Error` が更新されることを確認する。
14. client 再起動後に新規 post を作成し、restart 前後で author identity が変わらないことを確認する。
15. live session を `create -> join -> end` し、viewer count と ended state が相手側に反映されることを確認する。
16. game room を `create -> update score/status` し、相手側に score card が反映されることを確認する。

## Seeded DHT 手動確認
1. 2 instance とも `KUKURI_DISCOVERY_MODE=seeded_dht` を使うか、desktop の discovery panel で seed を保存できる状態にする。
2. 両方を起動し、`Local Endpoint ID` を相互に `Seed Peers` へ登録する。`node_id` だけで通ることを確認する。
3. `Save Seeds` 後に `Stored Seed IDs` と `Connected / Discovered` が埋まることを確認する。
4. `Peer Ticket` import を使わずに `post -> reply/thread -> live/game` が相互に伝播することを確認する。
5. 片側を再起動し、seed 再入力なしで再接続と timeline backfill が成立することを確認する。
6. `Seed Peers` に invalid な entry を入れて保存し、apply 全体が失敗して既存 seed が保持されることを確認する。

- `seeded_dht` は `direct_only` 前提なので、port または advertise address を変えた場合は新しい到達先が peer 間で到達可能であることを確認する。
- `node_id@host:port` は addr_hint 付き接続を含む。DHT 自体の確認は `node_id` のみで行う。

## Windows native smoke
1. native Windows host で `cargo xtask doctor`、`cargo xtask check`、`cargo xtask test` を通す。
2. `cd apps/desktop && npx pnpm@10.16.1 tauri:dev` を起動し、`post -> restart -> persist` と author `npub` 不変を確認する。
3. `KUKURI_DISABLE_KEYRING` を外した状態でも author `npub` が維持されることを確認する。
4. `KUKURI_INSTANCE` を分けた 2 instance で static-peer ticket import、`reply/thread`、live/game の伝播を確認する。
5. 片側終了時に相手側が polling で `connected: no, peers: 0` に戻ることを確認する。
6. 複数 topic の維持、topic 単位の unsubscribe、invalid ticket import 時の `Last Error` 更新を確認する。
7. 可能なら別 host 間でも `KUKURI_ADVERTISE_HOST` を使った static-peer 接続を確認する。
8. `cargo xtask desktop-package` で NSIS installer を build し、install 後に packaged app が通常の app data dir を使って起動することを確認する。

実機確認済み:
- Linux 実機 2 台で固定 port / 相互 ticket import による static-peer 接続が成立
- Linux 実機 2 台で `post -> reply -> thread` と複数 topic の双方向伝播が成立
- Linux 実機 2 台で topic 単位の unsubscribe と peer diagnostics 表示が期待どおりに機能
- Linux 実機で `片側だけ購読 -> 0`, `後から参加 -> 1`, `再び片側だけ -> 0` が topic peer diagnostics に反映され続けることを確認
- Linux 実機で global の `Connection Detail / Last Error` と topic ごとの `status_detail / error:` 表示が期待どおりに機能
- Linux 実機で client 再起動後も `npub` が変わらず、author identity が維持されることを確認
- Linux 実機 2 台で `seeded_dht` + 相互 `node_id` seed 設定だけで、ticket import なしの接続、再接続、投稿伝播が成立
- Linux 実機 2 台で `seeded_dht` + 相互 `node_id` seed 設定だけで、`reply/thread` と live/game の伝播、および restart 後 reconnect without reimport が成立
- Linux 実機 2 台で片側 port 変更後も、新 port が到達可能なら seed 再入力なしで再接続、投稿伝播、reply が成立
- Linux 実機 2 台で `seeded_dht` の invalid seed 保存が reject され、既存 seed が維持されることを確認
- Windows 実機で `cargo xtask doctor` / `cargo xtask check` / `cargo xtask test` が成功
- Windows 実機で `tauri:dev` の `post -> restart -> persist` と author `npub` 不変を確認
- Windows 実機で Credential Manager を使う keyring 有効状態でも author identity が維持されることを確認
- Windows 実機で `KUKURI_INSTANCE` を分けた 2 instance による static-peer ticket import、`post -> reply -> thread`、live/game 伝播が成功
- Windows 実機で片側終了後に相手側が `connected: no, peers: 0` に戻ることを確認
- Windows 実機で複数 topic 維持、topic 単位 unsubscribe、invalid ticket import 時の `Last Error` 更新が期待どおりに機能
- Windows 実機で別 host 間の static-peer 接続と投稿伝播が成功
- Windows 実機で `cargo xtask desktop-package` による NSIS installer build、install、packaged app 起動が成功
- Linux-first MVP の Phase4 desktop 縦スライスは完了

## Phase5 Cutover Check
1. `cargo xtask doctor` を通す。
2. `cargo xtask check` を通す。
3. `cargo xtask test` を通す。
4. `cargo xtask e2e-smoke` を通す。
5. `cargo xtask check` に含まれる Tauri backend compile が通ることを確認する。
6. `sqlite_deletion_does_not_lose_shared_state` と `restart_restores_from_docs_blobs_without_sqlite_seed` が green であることを確認する。
7. `missing_gossip_but_docs_sync_recovers_post` と `gossip_loss_does_not_lose_durable_post` が green であることを確認する。
8. `compat_event_gossip` が current code から除去されていることを確認する。
9. `legacy/` を参照せず root workspace だけで Linux の開発・テスト・起動が完結することを確認する。

現在の HEAD では上記 1-9 を local で確認済みで、Phase5 cutover は完了。

補足:
- desktop shell は約 2 秒ごとに timeline / sync status / local ticket を再取得する。
- `Refresh` は強制再取得用で、通常の確認では押さなくても反映される想定。

## 現在の注意点
- `kukuri-transport` の `transport_static_peer_can_connect_endpoint` は required。
- `kukuri-transport` の `transport_two_process_roundtrip_static_peer` は required に戻した。
- deterministic な required lane は `FakeTransport` と `kukuri-harness` が担う。
- Tauri wrapper の単体 compile は `cargo xtask check` に含めて確認する。
- `cargo xtask desktop-package` は Windows host 専用で、current-user NSIS installer を生成する。

補足:
- GitHub branch protection の required check 名は repo 外設定なので、`Next Fast/Nightly` から `Kukuri Fast/Nightly` への手動更新が必要。
