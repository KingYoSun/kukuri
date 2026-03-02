[title] 作業中タスク（in_progress）

最終更新日: 2026年03月02日

## 方針（2025年09月15日 更新）

- 当面は Nostr リレーとは接続せず、P2P（iroh + iroh-gossip + DHT）で完結した体験を優先。
- 全イベントは NIPs 準拠のスキーマに沿って取り扱う。
- Tauri v2 では E2E が困難なため、層別テスト（単体・結合/契約＋スモーク最小限）でカバレッジを確保。

## 現在のタスク
- P2Pトピック同期の不具合調査と修正（bootstrap反映、受信投稿のリアルタイム反映、プロフィール表示改善）

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
