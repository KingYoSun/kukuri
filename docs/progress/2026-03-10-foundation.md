# 2026年03月10日 foundation

## Status
- Linux-first MVP の Phase4 desktop 縦スライスは完了
- v3 Phase4.5E data source policy lock は完了
- v3 Phase5 cutover は完了
- Phase6-1 image post canonical source 設計に着手
- Phase6-6 video post canonical source 設計に着手

## 実装済み
- root Cargo workspace と `cargo xtask` alias
- `kukuri-core`, `kukuri-store`, `kukuri-transport`, `kukuri-app-api`, `kukuri-harness`
- `kukuri-desktop-runtime` を追加し、desktop 用 command surface を pure Rust で検証できるようにした
- `apps/desktop` の Linux-first shell
- `apps/desktop/src-tauri` の thin wrapper を追加し、`invoke` 経由で `create_post/list_timeline/list_thread/get_sync_status/import_peer_ticket` を呼べる形にした
- remote-sync 向けに `KUKURI_BIND_ADDR` / `KUKURI_ADVERTISE_HOST` / `KUKURI_INSTANCE` を導入し、loopback 固定を外せるようにした
- `kukuri-fast.yml`, `kukuri-nightly.yml`
- `kukuri-transport` は公式 `iroh-gossip` example / docs に寄せて `receiver.joined()` ベースの join gating を導入
- `kukuri-transport` に low-level baseline test を追加し、wrapper 依存の問題と `iroh-gossip` 本体の問題を分離できるようにした
- desktop UI に reply モードと thread pane からの reply 導線を追加した
- desktop UI を `trackedTopics + activeTopic + timelinesByTopic` 構成へ拡張し、複数 topic を同時購読できるようにした
- `peer診断表示の拡充` として topic ごとの `joined / peers / last_received_at` を UI 表示できるようにした
- `peer診断表示` に global/topic ごとの `status_detail / last_error` を追加し、接続待ちと直近エラー理由を UI 表示できるようにした
- desktop runtime で local 鍵を Linux keyring へ保存し、利用できない環境では 0600 の fallback file へ保存するようにした
- v3 foundation として `kukuri-docs-sync` / `kukuri-blob-service` を追加し、shared durable state の正本を docs/blobs に寄せる最小 data plane を導入した
- desktop-runtime で gossip/docs/blobs を shared iroh stack 上に統合し、`import_peer_ticket` が docs/blobs にも伝播するようにした
- root 直下の pre-cutover app/service tree を `legacy/` へ移し、root 入口を current kukuri 実装中心へ縮退した
- `ADR 0003` で image post の canonical source を `docs header + blobs payload + gossip hint + SQLite projection` に固定した
- `image_post_visible_before_full_blob_download` を attachment metadata と blob status 遷移まで広げた
- `ADR 0004` で video post の canonical source を `docs header + blobs payload + gossip hint + SQLite projection` に固定した
- video post の最小 poster UI と sync/durability contract を追加した
- desktop frontend の remote media 表示を `data URL` から `Blob + object URL` へ切り替え、image/video で共通 cache を使うようにした
- composer の添付 UI を single attach に統合し、`image/*,video/*` の mixed selection を client-side で分類するようにした
- video upload 時に browser 内の `video + canvas` で poster を自動生成し、failure 時は publish blocker にした
- composer に draft attachment preview を追加し、image は object URL、video は generated poster で publish 前 preview できるようにした
- video manifest を取得できても local decode が失敗した client では、poster-only preview と `unsupported on this client` 表示へ倒すようにした

## 検証済み
- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`
- `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`
- Linux 実機で `pnpm tauri dev` を使った `post -> restart -> persist`
- Linux 実機 2 台で固定 port / 相互 ticket import により `connected: yes, peers: 1` の双方向収束を確認
- Linux 実機 2 台で投稿伝播と、peer 終了後に polling で `connected: no, peers: 0` へ戻ることを確認
- Linux 実機 2 台で `root -> reply -> thread` 反映が peer 間で同期することを確認
- Linux 実機 2 台で複数 topic を同時購読し、各 topic で timeline が維持されることを確認
- Linux 実機 2 台で片側だけ購読している topic はその topic 行だけ `joined: false / peers: 0` になることを確認
- Linux 実機 2 台で共通購読 topic を片側で解除した後、その topic 行だけ `joined: false / peers: 0` になることを確認
- Linux 実機で `片側だけ購読 -> 0`, `後から参加 -> 1`, `再び片側だけ -> 0` が topic peer diagnostics に反映され続けることを確認
- Linux 実機で追加した peer diagnostics 表示が正常に機能することを確認
- Linux 実機で global の `Connection Detail / Last Error` と topic ごとの `status_detail / error:` 表示が正常に機能することを確認
- Linux 実機で client 再起動後も `npub` が変わらず、author identity が維持されることを確認
- desktop-runtime test で restart 後も author pubkey が維持されることを確認
- desktop-runtime test で `late_joiner_backfills_timeline_from_docs` が green
- desktop-runtime test で `sqlite_deletion_does_not_lose_shared_state` が green
- desktop-runtime test で `restart_restores_from_docs_blobs_without_sqlite_seed` が green
- app-api test で `reply/thread` の peer 間伝播を確認
- app-api test で複数 topic 同時購読時の subscription 追跡を確認
- app-api と frontend test で topic ごとの diagnostics 表示を確認
- app-api test で invalid ticket import 時に `last_error` が diagnostics へ反映されることを確認
- app-api test で `missing_gossip_but_docs_sync_recovers_post` と `gossip_loss_does_not_lose_durable_post` が green
- app-api test で `thread_open_triggers_lazy_blob_fetch` / `image_post_visible_before_full_blob_download` / `new_writes_use_blob_text_payload_refs` が green
- docs-sync test で `private_cursor_not_in_public_replica` が green

## 既知の制約
- `kukuri-transport` は ticket からの direct connect と 2-process gossip roundtrip を required に昇格済み
- Tauri backend binding と鍵永続化は導入済み。Phase4 の残作業はない。

## Phase5 Cutover
- `SQLite` を削除しても docs/blobs から shared durable state が復元できることを確認済み
- `gossip` を取り逃しても late join/backfill が docs/blobs だけで成立することを確認済み
- `compat_event_gossip` は current code から除去済み
- root workspace と root README は kukuri 中心へ縮退済み
- pre-cutover の app/service tree は `legacy/` へ移動済み
- `legacy/` を参照せず Linux の開発・テスト・起動が完結することを local required lane で確認済み
