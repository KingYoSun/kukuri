# 2026年03月10日 foundation

## Status
- Linux-first MVP の Phase4 desktop 縦スライスは完了

## 実装済み
- root Cargo workspace と `cargo xtask` alias
- `next-core`, `next-store`, `next-transport`, `next-app-api`, `next-harness`
- `next-desktop-runtime` を追加し、desktop 用 command surface を pure Rust で検証できるようにした
- `next/apps/desktop` の Linux-first shell
- `next/apps/desktop/src-tauri` の thin wrapper を追加し、`invoke` 経由で `create_post/list_timeline/list_thread/get_sync_status/import_peer_ticket` を呼べる形にした
- remote-sync 向けに `KUKURI_NEXT_BIND_ADDR` / `KUKURI_NEXT_ADVERTISE_HOST` / `KUKURI_NEXT_INSTANCE` を導入し、loopback 固定を外せるようにした
- `next-fast.yml`, `next-nightly.yml`
- `next-transport` は公式 `iroh-gossip` example / docs に寄せて `receiver.joined()` ベースの join gating を導入
- `next-transport` に low-level baseline test を追加し、wrapper 依存の問題と `iroh-gossip` 本体の問題を分離できるようにした
- desktop UI に reply モードと thread pane からの reply 導線を追加した
- desktop UI を `trackedTopics + activeTopic + timelinesByTopic` 構成へ拡張し、複数 topic を同時購読できるようにした
- `peer診断表示の拡充` として topic ごとの `joined / peers / last_received_at` を UI 表示できるようにした
- `peer診断表示` に global/topic ごとの `status_detail / last_error` を追加し、接続待ちと直近エラー理由を UI 表示できるようにした
- desktop runtime で local 鍵を Linux keyring へ保存し、利用できない環境では 0600 の fallback file へ保存するようにした
- v3 foundation として `next-docs-sync` / `next-blob-service` を追加し、shared durable state の正本を docs/blobs に寄せる最小 data plane を導入した
- desktop-runtime で gossip/docs/blobs を shared iroh stack 上に統合し、`import_peer_ticket` が docs/blobs にも伝播するようにした

## 検証済み
- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`
- Linux 実機で `pnpm tauri dev` を使った `post -> restart -> persist`
- Linux 実機 2 台で固定 port / 相互 ticket import により `connected: yes, peers: 1` の双方向収束を確認
- Linux 実機 2 台で投稿伝播と、peer 終了後に polling で `connected: no, peers: 0` へ戻ることを確認
- Linux 実機 2 台で `root -> reply -> thread` 反映が peer 間で同期することを確認
- Linux 実機 2 台で複数 topic を同時購読し、各 topic で timeline が維持されることを確認
- Linux 実機 2 台で片側だけ購読している topic はその topic 行だけ `joined: false / peers: 0` になることを確認
- Linux 実機 2 台で共通購読 topic を片側で解除した後、その topic 行だけ `joined: false / peers: 0` になることを確認
- Linux 実機で追加した peer diagnostics 表示が正常に機能することを確認
- Linux 実機で global の `Connection Detail / Last Error` と topic ごとの `status_detail / error:` 表示が正常に機能することを確認
- Linux 実機で client 再起動後も `npub` が変わらず、author identity が維持されることを確認
- `cargo check --manifest-path next/apps/desktop/src-tauri/Cargo.toml`
- desktop-runtime test で restart 後も author pubkey が維持されることを確認
- desktop-runtime test で `late_joiner_backfills_timeline_from_docs` が green
- app-api test で `reply/thread` の peer 間伝播を確認
- app-api test で複数 topic 同時購読時の subscription 追跡を確認
- app-api と frontend test で topic ごとの diagnostics 表示を確認
- app-api test で invalid ticket import 時に `last_error` が diagnostics へ反映されることを確認

## 既知の制約
- `next-transport` は ticket からの direct connect と 2-process gossip roundtrip を required に昇格済み
- Tauri backend binding と鍵永続化は導入済み。Phase4 の残作業はない。
