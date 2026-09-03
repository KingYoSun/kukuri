# Issue #857 Community Node consent allowlist rerun

## Summary

無償 Preview 公開のブロッカー（#853 Phase A）であるアプリ同意と Community Node 同意の分離について、再監査で不足していた通信境界を補強した。Node ごとのローカル同意が有効な場合だけ、その Node の report、relay、bootstrap seed、topic rendezvous peer を利用できる。保存済み設定と `resolved_urls` は保持しつつ、起動時・設定変更時・同意更新時に実効 connectivity を fail-closed で再構築する。

アプリ同意、Community Node の公開ポリシー取得、公開 manifest 取得は pre-consent allowlist のまま維持する。Node consent がない状態、撤回後、再同意待ちでは、認証 session、heartbeat、report、relay/seed/P2P assist を許可しない。

## 実装内容

- report 境界（`crates/desktop-runtime/src/community_node/report_routing_support.rs`）: same-origin と設定済み Node の検証に加え、Node ごとの active local consent と session の再同意待ち状態を送信前に検証する。同意なし・撤回後・再同意待ちは `CONSENT_REQUIRED` で停止し、HTTP POST が発生しない contract を追加した。匿名 report payload の既存仕様は変更していない。
- 実効 connectivity（`crates/desktop-runtime/src/community_node/config_support.rs`、`session_runtime_support.rs`、`runtime/mod.rs`）: 保存済み Node 設定から active local consent を持つ Node だけを relay/seed の実効設定へ射影する。topic rendezvous peer は Node URL ごとに保持し、同意撤回、Node 削除、設定 clear、再同意待ちへの遷移で該当 Node の relay、bootstrap seed、rendezvous peer を即時除外する。保存済み `resolved_urls` は破棄しないため、再同意後は明示的な再取得なしで再適用できる。
- API と harness（`crates/desktop-runtime/src/runtime/community_node_api.rs`、`crates/harness/src`）: consent accept 後の再適用と withdraw 後の無効化を runtime API 境界へ固定した。harness の接続再適用も active consent を条件にし、撤回後に Node assist が復活しないようにした。
- allowlist contract: pre-consent で許可されるのは公開 policies と公開 manifest だけであり、認証・heartbeat 等へ進まないことを request hit 数で検証する。report は未同意、撤回後、再同意待ちの全ケースで 0 hit を確認する。起動時の保存済み relay/seed と複数 Node の per-node filtering も unit test で固定した。
- 実通信 scenario（`community_node_public_connectivity`）: Node-assisted 接続を成立させた後に両クライアントの Node consent を撤回し、relay、bootstrap seed、rendezvous peer が空になることを確認する。その後に新しい manual ticket だけで Direct P2P を再確立し、投稿が複製されることを確認する。

## 検証

- `cargo test -p kukuri-desktop-runtime tests::community_node::report_submission -- --nocapture`: 8 件成功
- `cargo test -p kukuri-desktop-runtime tests::community_node::config -- --nocapture`: 12 件成功
- `cargo test -p kukuri-desktop-runtime tests::community_node::session -- --nocapture`: 6 件成功
- `cargo test -p kukuri-desktop-runtime --lib`: 158 件成功
- `cargo check -p kukuri-desktop-runtime -p kukuri-harness`: 成功
- `cargo xtask scenario community_node_public_connectivity`: 15 step 成功
- `cargo xtask doctor`: 成功
- `cargo xtask check`: 成功
- `cargo xtask cn-check`: 成功
- `cargo xtask cn-test`: Community Node 全crateの unit / contract / integration / doc test 成功
- `cargo xtask oversized-files`: 成功（Community Node scenario の 15 step 化を baseline へ記録）
- `cargo xtask test`: Rust 754 件、harness 22 件、frontend 1074 件すべて成功（Rust 3 件 skip）
- `cargo xtask e2e-smoke`: 成功

関連: #853（親）、#857

## 2026-09-04 Dome hosting 再監査追補

再監査で未充足だった Dome hosting の同意境界を実装した。Dome の status、health、delegate、rotate、revoke、resync は共通の GET / request helper を通り、保存済み Community Node、active local consent、公開 policy catalog の現行 required snapshot を順に確認してから token 読み込み、認証、Dome API 通信へ進む。同意なし、撤回後、policy 更新後、未設定 Node は typed error で fail-closed になり、認証および Dome API の request は 0 件となる。

UI は自由入力の Node ID / base URL を廃止し、設定済み Community Node の公開情報から得た `node_id` と base URL を一体の対象として選択する。未同意または再同意待ちでは既存の Community Node consent dialog を表示し、表示した policy slug / version / snapshot への同意完了後だけ、同じ Node を対象に delegate を再開する。公開情報が取得できない Node は選択対象にせず、Community Node 設定への導線を表示する。

追加 contract は次を固定する。

- runtime 6 件: 未同意 GET / POST、撤回後、policy 更新後、未設定 Node、現行同意での GET / POST 成功、401 後の再認証を検証する。
- UI 6 件: 現行同意での直接 delegate、未同意からの exact-policy accept、拒否、公開情報なし、policy 取得失敗、runtime race の `CONSENT_REQUIRED` を検証する。
- lock classification は新しい Dome hosting request contract 6 件を `CommunityNodeServer` class として明示する。

追補後の検証:

- `cargo xtask doctor`: 成功
- `cargo xtask check`: 成功
- `cargo xtask oversized-files`: 成功
- `cargo xtask test`: Rust 765 件、harness 22 件、frontend 1087 件すべて成功（Rust 3 件 skip）
- `cargo xtask desktop-ui-check`: lint、typecheck、unit 1087 件、Storybook build、browser 58 件、visual 14 件すべて成功
- `cargo xtask e2e-smoke`: `desktop_smoke_post_persist` 6 step 成功
