# Development Runbook

## 初回セットアップ
```bash
npx pnpm@10.16.1 install --dir next/apps/desktop
cargo xtask doctor
```

## 日常コマンド
```bash
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke
```

## frontend だけ確認する場合
```bash
cd next/apps/desktop
npx pnpm@10.16.1 dev
npx pnpm@10.16.1 test
npx pnpm@10.16.1 tauri:dev
```

## remote-sync 用の環境変数
```bash
export KUKURI_NEXT_BIND_ADDR=0.0.0.0:0
export KUKURI_NEXT_ADVERTISE_HOST=<LANで到達可能なIPまたはホスト名>
export KUKURI_NEXT_ADVERTISE_PORT=<必要なら固定port>
export KUKURI_NEXT_INSTANCE=<同一マシンで複数起動する場合の識別子>
export KUKURI_NEXT_DISABLE_KEYRING=1
```

- `KUKURI_NEXT_ADVERTISE_HOST` を設定すると `Your Ticket` はその host を使う。
- `KUKURI_NEXT_INSTANCE` を設定すると app data dir が分離される。
- `KUKURI_NEXT_APP_DATA_DIR` を設定すると app data dir を丸ごと上書きできる。
- `KUKURI_NEXT_DISABLE_KEYRING=1` を設定すると Linux keyring を使わず、app data dir 内の 0600 fallback file を使う。

## 回帰用の手動確認
1. 各端末で `KUKURI_NEXT_BIND_ADDR=0.0.0.0:0` と `KUKURI_NEXT_ADVERTISE_HOST` を設定する。
2. 同一マシンで複数起動する場合は `KUKURI_NEXT_INSTANCE` も別値にする。
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

実機確認済み:
- Linux 実機 2 台で固定 port / 相互 ticket import による static-peer 接続が成立
- Linux 実機 2 台で `post -> reply -> thread` と複数 topic の双方向伝播が成立
- Linux 実機 2 台で topic 単位の unsubscribe と peer diagnostics 表示が期待どおりに機能
- Linux 実機で global の `Connection Detail / Last Error` と topic ごとの `status_detail / error:` 表示が期待どおりに機能
- Linux 実機で client 再起動後も `npub` が変わらず、author identity が維持されることを確認
- Linux-first MVP の Phase4 desktop 縦スライスは完了

補足:
- desktop shell は約 2 秒ごとに timeline / sync status / local ticket を再取得する。
- `Refresh` は強制再取得用で、通常の確認では押さなくても反映される想定。

## 現在の注意点
- `next-transport` の `transport_static_peer_can_connect_endpoint` は required。
- `next-transport` の `transport_two_process_roundtrip_static_peer` は required に戻した。
- deterministic な required lane は `FakeTransport` と `next-harness` が担う。
- Tauri wrapper の単体 compile は `cargo check --manifest-path next/apps/desktop/src-tauri/Cargo.toml` で確認する。
