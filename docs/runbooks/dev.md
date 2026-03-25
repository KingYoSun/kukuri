# Development Runbook

## 初回セットアップ
```bash
npx pnpm@10.16.1 install --dir apps/desktop
cargo xtask doctor
```

- `apps/desktop` の frontend toolchain は Vite 8 に合わせて Node `^20.19.0 || >=22.12.0` を前提にする

## 日常コマンド
```bash
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke
cargo xtask desktop-ui-check
cargo xtask cn-check
cargo xtask cn-test
cargo xtask scenario community_node_public_connectivity
cargo xtask scenario community_node_multi_device_connectivity
```

`cargo xtask check` は workspace lint/test に加えて `apps/desktop/src-tauri` の Tauri backend compile も確認する。

`cargo xtask desktop-ui-check` は `apps/desktop` の `lint`, `typecheck`, `test`, `storybook:build`, `test:e2e:browser` をまとめて流す frontend 専用 gate。

`cargo xtask cn-check` / `cargo xtask cn-test` は `cn-*` server slice の compile/test 用。

- `cargo xtask test` は workspace 全体を通すが、Postgres が必要な `cn-*` integration test は実行しない。
- `cargo xtask cn-test` は `docker-compose.community-node.yml` の `cn-postgres` を自動起動し、`KUKURI_CN_RUN_INTEGRATION_TESTS=1` を付けて contract/integration test を流す。
- `cargo xtask scenario community_node_public_connectivity` も `cn-postgres` を自動起動し、in-process の `cn-user-api` / `cn-iroh-relay` を立てて 2 desktop scenario を流す。
- `cargo xtask scenario community_node_multi_device_connectivity` は same-author 2 desktop の endpoint-bound bootstrap で `post -> reply/thread -> reconnect` を確認する。

## community-node compose
```bash
docker compose --env-file .env.community-node -f docker-compose.community-node.yml run --rm cn-migrate
docker compose --env-file .env.community-node -f docker-compose.community-node.yml up --build cn-user-api cn-iroh-relay
```

- host port の既定値は `18080` (`cn-user-api`), `13340` (`cn-iroh-relay`), `55432` (`cn-postgres`)
- host 側 bind の既定値は loopback (`127.0.0.1`) なので、LAN/WireGuard 越しに公開する場合は `CN_*_HOST_BIND_IP` を上書きする
- compose 内の service 名は `cn-postgres`, `cn-migrate`, `cn-user-api`, `cn-iroh-relay`
- public URL を変える場合は `CN_BASE_URL`, `CN_PUBLIC_BASE_URL`, `COMMUNITY_NODE_CONNECTIVITY_URLS` を上書きする
- `cn-user-api` は `COMMUNITY_NODE_DATABASE_INIT_MODE=require_ready` で起動するので、`cn-migrate` または `cn-cli prepare` を先に流さないと fail-fast する

## community-node env 標準形
- `.env.community-node.example` をコピーして `.env.community-node` を作り、compose では `--env-file .env.community-node` を使う
- secret は `CN_POSTGRES_PASSWORD` と `COMMUNITY_NODE_JWT_SECRET` の 2 つを最低限上書きする
- `COMMUNITY_NODE_JWT_SECRET` を rotate すると既存 bearer token は即時無効化される
- `COMMUNITY_NODE_DATABASE_URL` は compose 内では `cn-postgres` 向けに組み立てる。外部 Postgres を使う場合だけ個別に差し替える

## community-node 公開 manual smoke
公開 URL を current community-node 構成で出す場合は、`.env.community-node` に最低限この 3 つを入れる。

```dotenv
CN_BASE_URL=https://api.kukuri.app
CN_PUBLIC_BASE_URL=https://api.kukuri.app
COMMUNITY_NODE_CONNECTIVITY_URLS=https://iroh-relay.kukuri.app
```

- `api.kukuri.app` は `cn-user-api` を向ける
- `iroh-relay.kukuri.app` は `cn-iroh-relay` を向ける
- desktop は `connectivity_urls` を server から受け取るので、websocket relay 前提は使わない

TCP 公開を Cloudflare Tunnel で行う場合:

- `CN_USER_API_HOST_BIND_IP=127.0.0.1`
- `CN_IROH_RELAY_HTTP_HOST_BIND_IP=127.0.0.1`
- tunnel 側で `api.kukuri.app -> 127.0.0.1:${CN_USER_API_PORT}`, `iroh-relay.kukuri.app -> 127.0.0.1:${CN_IROH_RELAY_PORT}` を割り当てる

`iroh-relay` の `7842/udp` を WireGuard/VPS edge 経由で公開する場合:

```dotenv
CN_IROH_RELAY_QUIC_BIND_ADDR=0.0.0.0:7842
CN_IROH_RELAY_QUIC_HOST_BIND_IP=10.73.0.2
CN_IROH_RELAY_QUIC_PORT=7842
CN_IROH_RELAY_TLS_CERT_PATH=/certs/default.crt
CN_IROH_RELAY_TLS_KEY_PATH=/certs/default.key
CN_IROH_RELAY_CERTS_HOST_PATH=./docker/cn/certs
```

- Cloudflare Tunnel は UDP を運べないので、`7842/udp` は WireGuard/VPS edge で home 側へ直接 forward する
- QUIC は tunnel を迂回するので、`docker/cn/certs/` には `iroh-relay.kukuri.app` 用の公開証明書と秘密鍵を置く
- `CN_IROH_RELAY_QUIC_HOST_BIND_IP` は WireGuard で到達可能な home 側 IP に合わせる
- Cloudflare Tunnel で `iroh-relay.kukuri.app` の TCP を公開しつつ QUIC を直公開したい場合は、`CN_IROH_RELAY_HTTPS_BIND_ADDR` を空のままにして `iroh-relay.kukuri.app -> http://127.0.0.1:${CN_IROH_RELAY_PORT}` を向ける
- `CN_IROH_RELAY_HTTPS_BIND_ADDR` を設定した場合、local HTTP listener は captive portal 用の `/generate_204` しか返さない。`/`, `/ping`, `/relay`, `/healthz` を Cloudflare Tunnel で通したいなら `iroh-relay.kukuri.app -> https://127.0.0.1:${CN_IROH_RELAY_HTTPS_PORT}` を向ける
- `https://iroh-relay.kukuri.app/ping` が `404 Not Found` を返す場合は、Cloudflare Tunnel が HTTP origin (`${CN_IROH_RELAY_PORT}`) に向いているのに `CN_IROH_RELAY_HTTPS_BIND_ADDR` が有効な構成になっている

起動:

```bash
docker compose --env-file .env.community-node -f docker-compose.community-node.yml run --rm cn-migrate
docker compose --env-file .env.community-node -f docker-compose.community-node.yml up -d --build cn-user-api cn-iroh-relay
```

公開確認:

```bash
curl -fsS https://api.kukuri.app/healthz
curl -fsS https://iroh-relay.kukuri.app/ping
```

期待値:

- `connectivity_urls` は `https://iroh-relay.kukuri.app`
- desktop client 側は `Save Nodes -> Authenticate -> Accept` の順で進め、その session のまま relay-assisted path を使える
- 公開 community-node path では `Peer Ticket` import は不要
- `Authenticate` 直後の `connectivity urls: pending consent acceptance` は正常で、`Accept` 後に resolved される
- `Accept` 後に `restart required: no` のまま relay-assisted sync へ移るのが正常で、`yes` が出たら regression とみなす
- discovery diagnostics では `Community Bootstrap Peers` が community-node 由来、`Configured Seed IDs` が local seed 設定、`Manual Ticket Peers` が手動 import を表す
- Linux 実機の公開 manual smoke では `Save Nodes -> Authenticate -> Accept -> post -> reply/thread -> blob sync` まで restart なしで成功を確認済み
- relay-only public path でも `Sync Status` / `Tracked Topics` diagnostics は relay-assisted docs/blob peer を含めて `connected` と `peer_count` を出す

## community-node deploy 順序
```bash
cargo run -p kukuri-cn-cli -- --database-url "$COMMUNITY_NODE_DATABASE_URL" prepare
cargo run -p kukuri-cn-cli -- --database-url "$COMMUNITY_NODE_DATABASE_URL" set-auth-rollout --mode off
cargo run -p kukuri-cn-user-api
cargo run -p kukuri-cn-iroh-relay
```

1. migration/seed は `cn-cli prepare` だけで行う
2. `cn-user-api` は prepared DB を前提に起動する
3. rollout 変更は deploy 後に `cn-cli set-auth-rollout` で行う
4. `COMMUNITY_NODE_DATABASE_INIT_MODE=prepare` は local bring-up と test 用に限定し、常用しない

compose を使う場合:
```bash
docker compose --env-file .env.community-node -f docker-compose.community-node.yml run --rm cn-migrate
docker compose --env-file .env.community-node -f docker-compose.community-node.yml up -d cn-user-api cn-iroh-relay
```

## community-node backup / restore
backup:
```bash
docker compose --env-file .env.community-node -f docker-compose.community-node.yml exec -T cn-postgres \
  sh -lc 'pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" -Fc' > cn-postgres.dump
```

restore:
```bash
cat cn-postgres.dump | docker compose --env-file .env.community-node -f docker-compose.community-node.yml exec -T cn-postgres \
  sh -lc 'dropdb --if-exists -U "$POSTGRES_USER" "$POSTGRES_DB" && createdb -U "$POSTGRES_USER" "$POSTGRES_DB" && pg_restore --clean --if-exists --no-owner -U "$POSTGRES_USER" -d "$POSTGRES_DB"'
```

- backup は schema + data をまとめて保持する `pg_dump -Fc` を標準にする
- restore 前に `cn-user-api` を停止して、Postgres への新規接続を止める
- restore 後に追加 migration がある場合だけ `cn-cli prepare` を流す
- `cn-postgres-data` volume を直接コピーして backup 代わりにしない

## community-node 検証
```bash
cargo xtask cn-test
cargo xtask scenario community_node_public_connectivity
cargo xtask scenario community_node_multi_device_connectivity
```

- `cn-test` は `/v1/auth/challenge`, `/v1/auth/verify`, `/v1/consents/status`, `/v1/consents`, `/v1/bootstrap/nodes` の contract を確認する。
- `community_node_public_connectivity` scenario は `config -> auth -> consent -> post -> reply/thread -> live -> game -> reconnect` を 1 community-node stack + 2 desktops で確認する。
- `community_node_multi_device_connectivity` scenario は same-author 2 desktop の `auth -> consent -> post -> reply/thread -> reconnect` を確認する。
- crate test を直接叩く場合は `KUKURI_CN_RUN_INTEGRATION_TESTS=1` と `COMMUNITY_NODE_DATABASE_URL` を明示する。
- 公開 community-node の手動確認では UI の peer source と peer count を見つつ、timeline / thread / attachment preview / blob media payload fetch の成否まで確認する。

## frontend だけ確認する場合
```bash
cd apps/desktop
npx pnpm@10.16.1 dev
npx pnpm@10.16.1 test
npx pnpm@10.16.1 storybook:build
npx pnpm@10.16.1 test:e2e:browser
npx pnpm@10.16.1 tauri:dev
```

- `pnpm tauri dev` / `pnpm tauri:dev` は loopback の空き port を自動選択し、5173 が使用中なら次の空き port へ退避する。
- desktop shell UI の primary route は hash-based (`#/timeline`, `#/channels`, `#/live`, `#/game`, `#/profile`) に固定されている。settings / context deep-link も hash search param で復元する。
- desktop の Tauri backend は `mainline::rpc::socket`, `noq_proto::connection`, `iroh::socket::remote_map::remote_state`, `iroh_docs::engine::live`, `iroh_gossip::net` を既定で `error` へ落としている。community-node connectivity assist / DHT / docs sync の内部 warning を調べたいときだけ `RUST_LOG=warn,mainline::rpc::socket=warn,noq_proto::connection=warn,iroh::socket::remote_map::remote_state=warn,iroh_docs::engine::live=warn,iroh_gossip::net=warn` を明示する。

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
- `KUKURI_DISABLE_KEYRING=1` を設定すると OS keyring を使わず、app data dir 内の `*.identity-key` fallback file を使う。
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

## Social graph manual verification

`friend of friend` まで見る場合は 3 author 構成を使う。`mutual` と restart 復元だけなら 2 desktop でもよい。

事前に流す自動テスト:

```bash
cargo test -p kukuri-store store_profile_upsert_latest_wins -- --nocapture
cargo test -p kukuri-store author_relationship_projection_rebuild_roundtrip -- --nocapture
cargo test -p kukuri-app-api social_graph_derives_friend_of_friend_and_clears_after_unfollow -- --nocapture
cargo test -p kukuri-desktop-runtime friend_only_channel_restore_keeps_archived_epoch_history -- --nocapture
cd apps/desktop && npx pnpm@10.16.1 test
```

操作手順:

1. 2-3 desktop を起動する。最小 lane は static-peer ticket import で、`friend of friend` まで見るなら A/B/C の 3 author を使う。
2. 同じ public topic を開き、author detail が開ける状態まで接続させる。
3. A で profile を更新し、A の表示名や profile 情報が B 側へ hydrate されることを確認する。
4. A -> B と B -> A で follow し、両端末の author detail に `mutual` が出ることを確認する。
5. `friend of friend` を見る場合は B -> C だけを follow し、A から見た C に `friend of friend` が出ることを確認する。
6. A で B を unfollow するか、`friend of friend` lane では A と B の link を外し、対応する relationship 表示が消えることを確認する。
7. 両端末を再起動し、profile 表示、follow 状態、必要なら `mutual` / `friend of friend` の表示が復元されることを確認する。

## Private channel manual verification

最小 lane は static-peer ticket import。manual verification は `invite_only`, `friend_only`, `friend_plus` の audience ごとに流す。

事前に流す自動テスト:

```bash
cargo test -p kukuri-docs-sync private_replica_requires_registered_capability -- --nocapture
cargo test -p kukuri-app-api private_channel_invite_scopes_posts_and_replies -- --nocapture
cargo test -p kukuri-app-api friend_only_grant_requires_mutual_and_rotate_requires_fresh_grant -- --nocapture
cargo test -p kukuri-app-api friend_plus_share_freeze_rotate_and_new_epoch_visibility -- --nocapture
cargo test -p kukuri-desktop-runtime private_channel_invite_restores_after_restart_without_reimport -- --nocapture
cargo test -p kukuri-desktop-runtime friend_only_channel_restore_keeps_archived_epoch_history -- --nocapture
cargo test -p kukuri-desktop-runtime friend_plus_channel_restore_redeems_rotation_after_restart -- --nocapture
cargo test -p kukuri-harness private_channel_invite_connectivity -- --nocapture
cargo test -p kukuri-harness friend_only_rotate_requires_fresh_grant -- --nocapture
cargo test -p kukuri-harness friend_plus_share_freeze_rotate_connectivity -- --nocapture
cd apps/desktop && npx pnpm@10.16.1 test
```

### `invite_only` lane

必ず `Create Channel -> Create Invite -> Join via Invite` の順で行う。

操作手順:

1. 2 desktop を起動する。最小構成は `Peer Ticket` の相互 import。可能なら 3 台目の未招待端末 C も起動する。
2. 両端末で同じ topic を開き、public timeline が同期することを確認する。
3. 端末 A で `Create Channel` を押し、label を入力して private channel を明示的に作成する。
4. 端末 A で `View Scope` と `Compose Target` が新しい channel に切り替わったことを確認する。
5. 端末 A で `Create Invite` を押し、invite token が表示されることを確認する。
6. 端末 B で `Join via Invite` に token を貼り付けて import し、topic が tracked state に入り、対象 channel が選択されることを確認する。
7. 端末 A でその private channel に post し、端末 B の `All joined` または当該 channel view にだけ表示され、`Public` には出ないことを確認する。
8. 端末 B でその private post に reply し、thread が同じ private channel 内でのみ見えることを確認する。
9. 端末 A でその private channel 上に live session を作成し、端末 B が `join -> leave -> end` を追従できることを確認する。
10. 端末 A でその private channel 上に game room を作成し、score / status 更新が端末 B に反映されることを確認する。
11. 両端末を再起動し、invite 再入力なしで joined private channel が復元され、private post / thread / live / game が再表示されることを確認する。
12. 端末 B でも `Create Invite` が可能で、fresh invite を再発行できることを確認する。
13. 3 台目の未招待端末 C を使う場合は、同じ topic の `Public` / `All joined` から private channel content が見えないことを確認する。

### `friend_only` lane

1. 最低 3 desktop を起動する。A を owner、B を joiner、D を fresh grant 確認用に使う。非 mutual reject を見るなら C も用意する。
2. A と B、および A と D で相互 follow を成立させる。C を使う場合は A と mutual にしない。
3. A で `Audience: Friends` を選んで `Create Channel` し、`Create Grant` で token を発行する。
4. B で `Join Grant` を行い、private channel が選択され、`Policy: Friends` が表示されることを確認する。
5. A で private post を作り、B にだけ表示され、`Public` に漏れないことを確認する。
6. A または B の follow を外して mutual を崩し、owner 側に `rotation required` が出ることを確認する。
7. rotate 前に発行した古い grant を保持したまま、A で `Rotate` を実行する。
8. 古い grant での join が失敗し、新しく発行した grant では join できることを確認する。
9. rotate 後に join した newcomer は old epoch の content を読めず、新 epoch の content だけ読めることを確認する。

### `friend_plus` lane

1. 4 desktop を起動する。A を owner、B/C を既存 participant、D を stale share / fresh share 確認用に使う。
2. A-B、B-C、B-D の pairwise mutual を成立させる。
3. A で `Audience: Friends+` を選んで `Create Channel` し、A -> B share、B -> C share の順で join させる。
4. C 側に `joined via <B>` が表示され、A/B/C 間で private post が同期し、`Public` に漏れないことを確認する。
5. B -> D の share を発行したまま未 import にしておき、A で `Freeze` を実行する。
6. freeze 後も既存 participant は write を継続できる一方、D の stale share import は失敗することを確認する。
7. A で `Rotate` を実行し、B/C が restart なしまたは restart 後復元で新 epoch へ移行することを確認する。
8. old share は rotate 後も失敗し、B が発行した fresh share では D が join できることを確認する。
9. rotate 後に join した D は old epoch content を読めず、新 epoch content だけ読めることを確認する。

自動テスト対応:

- `private_replica_requires_registered_capability`: capability 未登録では private replica を開けない
- `private_channel_invite_scopes_posts_and_replies`: invite import 後の private post / reply 継承を確認する
- `friend_only_grant_requires_mutual_and_rotate_requires_fresh_grant`: mutual 条件、rotation required、fresh grant 必須を確認する
- `friend_plus_share_freeze_rotate_and_new_epoch_visibility`: mutual-chain share、freeze、rotate、new epoch visibility を確認する
- `private_channel_invite_restores_after_restart_without_reimport`: desktop-runtime で restart 後の capability 復元を確認する
- `friend_only_channel_restore_keeps_archived_epoch_history`: friend-only の archived/current epoch 復元を確認する
- `friend_plus_channel_restore_redeems_rotation_after_restart`: friend-plus の encrypted rotation grant redeem を確認する
- `private_channel_invite_connectivity`: 3 client static-peer scenario で invite-only の private post / reply / live / game / restart を通す
- `friend_only_rotate_requires_fresh_grant`: static-peer runtime で friend-only rotate と stale grant block を通す
- `friend_plus_share_freeze_rotate_connectivity`: 4 client static-peer runtime で share chain / freeze / rotate / stale share block を通す
- `apps/desktop/src/App.test.tsx`: `Audience`, `Create Grant`, `Create Share`, `Freeze`, `Rotate`, invite/grant/share 導線の表示を確認する

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
2. `cd apps/desktop && npx pnpm@10.16.1 tauri:dev` を起動し、`post -> restart -> persist` と author pubkey 不変を確認する。
3. `KUKURI_DISABLE_KEYRING` を外した状態でも author pubkey が維持されることを確認する。
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
- Linux 実機で client 再起動後も author pubkey が変わらず、author identity が維持されることを確認
- Linux 実機 2 台で `seeded_dht` + 相互 `node_id` seed 設定だけで、ticket import なしの接続、再接続、投稿伝播が成立
- Linux 実機 2 台で `seeded_dht` + 相互 `node_id` seed 設定だけで、`reply/thread` と live/game の伝播、および restart 後 reconnect without reimport が成立
- Linux 実機 2 台で片側 port 変更後も、新 port が到達可能なら seed 再入力なしで再接続、投稿伝播、reply が成立
- Linux 実機 2 台で `seeded_dht` の invalid seed 保存が reject され、既存 seed が維持されることを確認
- Linux 実機 2 台で relay-only community-node (`https://api.kukuri.app`, `https://iroh-relay.kukuri.app`) に対し、ticket import なしの peer 間接続、`post -> reply/thread -> live -> game` 伝播が成立
- Windows 実機で `cargo xtask doctor` / `cargo xtask check` / `cargo xtask test` が成功
- Windows 実機で `tauri:dev` の `post -> restart -> persist` と author pubkey 不変を確認
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
