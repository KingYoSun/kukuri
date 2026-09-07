# Issue #922: CN Dome transfer完了処理の所有者を集約

- Scope revision `2026-09-08-872-R-RT-v1`、区分C。before `d5c7769a9c73c76ccfeabb8b0b0453d495b1e1b6`。
- 先行#921はPR#932で全CI・独立監査・merge後照合PASSを経て完了。固定INV1〜3/TR1〜5のcontractを保持して着手。
- 意図: runtimeの2入口が二重所有していたassignment→owner activation保存→CN activateを1 private methodへ抽出。

| 作業 | AC / INVAR | 対象・証拠 | 依存 |
| --- | --- | --- | --- |
| T1 before contract | AC-2、INVAR-1/2 | runtime11 testsとAppService2 tests、#921 merge/head一致 | #921 |
| T2 private method抽出 | AC-1/2、INVAR-1/2 | `private_channels_game_api.rs`だけ。prepare/no-op/view組立は各入口に残す | T1 |
| T3 変更後検証・測定 | AC-3、全INVAR | 同runtime11 tests、AppService tests、caller2/実行site1の逆引き、rust-test CI | T2 |
| T4 監査・CI・merge | AC-3、全INVAR | fixed head独立監査・必須CI・merge tree照合、結果はPR/Issue | T3 |

## 構造上の成果

`complete_community_node_dome_transfer`の本番callerはdelegate/commit layoutの2箇所のみ。
このmethodだけが`build_dome_hosting_assignment_request`、`assign_dome_hosting_to_community_node`、`activate_dome_hosting_on_community_node`を順に利用し、間のAppService activationを所有する。各実行siteは2→1、public入口は2のまま。
元のmodule内にprivate methodを置くため、他moduleへhelperの公開範囲を広げない。単なるファイル分割を成果にしない。

## 維持した制御と境界

- delegateは自分でprepareしてからhelperへ進み、その結果viewを返す。
- layoutはcandidate取得、AppService commit、owner target/no-op/active operation retryの早期returnを維持。CN Transferring時だけhelperへ進み、committed.hostingを置換する。
- 元の4つのmissing lease/activation error contextはstatic tupleで渡し、delegate/layoutの文言を維持する。新しいdomain policy、retry、transaction、rollbackは追加しない。
- 既存CN adapterは各送信の直前に同意を再確認する。owner activation保存後の送信失敗で永続状態を巻き戻さない。
- instance/context/base URL、署名済みJSONの取出し・serialize、asset bundle、public request/response/IPC/HTTP/wire/schema/authorityは不変。

CodeGraphで元の各sequenceを確認し、抽出後は `rg -n 'complete_community_node_dome_transfer|assign_dome_hosting_to_community_node\(|activate_dome_hosting_on_community_node\(|build_dome_hosting_assignment_request\(' crates/desktop-runtime/src/runtime/private_channels_game_api.rs` で全siteを再列挙。Tauri/CLI callerに変更なし、INV/TR増減0、未分類0。

## 検証

- before runtime: `cargo test -p kukuri-desktop-runtime tests::community_node::dome_hosting` →11 PASS（4.65秒）。
- before AppService: `cargo test -p kukuri-app-api tests::dome_hosting` →2 PASS（0.05秒）。
- after runtime: 同11 tests PASS（3.76秒）。返却view、各失敗後の署名済み状態、同operation、追加HTTP禁止、assignment後のpolicy更新を同じassertionで検証。
- test/fixture変更0、UI/Rust以外の境界変更0。必須rust-testはCIの対象SHA/jobとともに記録し、local targetedを全suite成功としない。
- `git diff --check`とoversizedを確認。root専用CARGO_TARGET_DIRを使用する。

Rollbackはこの抽出PRのみrevertし#921 contractを保持。data/schema migrationなし。公開契約変更が必要と判明した場合は抽出へ混ぜない。
