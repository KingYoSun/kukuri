# Issue #872 Phase 1: harness・tests・docs・CI の監査

## 対象と判断範囲

- 基準: `ac9946d66aaf197204f669a10701fac8a13877e9`。親の `INV-6`、`TR-1`〜`TR-5`。
- 対象: `crates/harness`、`crates/test-support`、`harness/scenarios`、`xtask`、CI登録、現行規範と前回責務マップ。
- 製品コードとテストは変更しない。長時間テストの失敗原因を推測で断定せず、観測した失敗は別種別へ分ける。
- CodeGraph の `run_scenario` / `run_named_scenario` / `oversized_files` 探索、scenario module の symbol map と sourceを使用。module登録や同名symbolの曖昧さは登録点と完全名の参照で補った。

## 固定 inventory と双方向経路

| ID | 入口・全memberの列挙 | helper → sink | sinkからの逆引きとguard | 対応する証拠 |
| --- | --- | --- | --- | --- |
| H-1 | `xtask/src/main.rs`のscenario/e2e入口 → `xtask/src/scenario.rs` → `run_named_scenario`。YAMLは`git ls-tree -r --name-only ac9946d6 -- harness/scenarios`で19本 | `load_scenario` → `run_scenario`の10種の`ScenarioKind`分岐 → 個別runner → runtime/DB/network/result artifact | `crates/harness/src/scenarios/mod.rs`で全10分岐を確認。`run_named_scenario`の利用者はxtaskとharness tests、`run_scenario`の内部呼出は前者。再exportを維持する | `crates/harness/src/tests/*`、19 YAML、Fast/Nightlyのscenario登録 |
| H-2 | `CommunityNodePublicConnectivity` / `CommunityNodeMultiDeviceConnectivity`の2入口 | `run_community_node_connectivity` → `CommunityNodeStack::spawn` → 2 runtimeの設定・同意・通信・再接続 → `shutdown_runtime` / `stack.shutdown` | 2入口は`DistinctUsers` / `SharedIdentity`として列挙済み。未同意時DirectOnly/seed空/auth未確立をassert。scenario errorでもstack shutdownし、二重失敗は元エラーを保持する(112〜337、1194〜1227行) | `community_node_public_connectivity.yaml`、`community_node_multi_device_connectivity.yaml`。最新Fastで前者、9/6 Nightlyで両者PASS |
| H-3 | `DesktopSmoke`と`ScenarioStep`の列挙 | `run_desktop_smoke_scenario` → `ScenarioRuntime` / `AppService` → SQLite、MemoryDocsSync/MemoryBlobService/FakeNetwork、artifact | `scenarios/mod.rs`の単独dispatch先。`ScenarioStep`matchとYAMLの対応を維持。overall/step timeoutとstep順序・error文脈は中央runner所有 | post/bookmark/game/live/Dome系YAML、`tests/desktop_smoke.rs`。e2e-smokeはpost永続往復のみ |
| H-4 | 全runnerの完了 → `write_result_artifact` | `artifacts.rs` → `result.json`（optional metrics_snapshotを含む） | `rg -n 'write_result_artifact' crates/harness/src`で8 runnerの書込箇所とlib再exportを列挙。result schema、scenario名、step名、失敗の伝播を凍結 | `artifacts.rs`、個別scenario tests。書式変更は今回対象外 |
| H-5 | Fast/Nightly、`cargo xtask rust-test` / `app-api-slow-test` | `xtask` → nextest / cargo / frontend / docker → validation exit code、artifact | `.github/workflows/kukuri-{fast,nightly}.yml`、`.config/nextest.toml`を逆に辿る。harnessとiroh integrationのserial group、slow featureの別lane、必須CIを維持 | 下記既知失敗、親のvalidation表 |
| H-6 | `AGENTS.md` → docs/README → runbook / REFACTORING / PLANS → Issue/PR雛形 | 開始・検証・終了判断と追跡Issue | 現行runbookと#894の規範mergeを確認。古いprogressの判断で現行規範を上書きしない | `docs/runbooks/issue-lifecycle.md`、`REFACTORING.md`、`.github/ISSUE_TEMPLATE/engineering-change.yml` |
| H-7 | `crates/test-support/{Cargo.toml,src/lib.rs}`の13公開member → runtime/app-api/harness/CNのtest fixture | resource lock→mutex/guard解放、poll→callback/timeout/sleep、env gate→環境変数読取、snapshot→診断文字列 | 全13memberと全callerの列挙は下記。resource別排他、連続Ready、gate不成立時None、Operation/Timeout区別を維持 | `poll_requires_consecutive_ready_results` / `poll_reports_timeout` / `snapshot_format_keeps_diagnostics` / `env_gate_truth_values_are_normalized` / `resource_locks_exclude_only_the_same_resource` |
| H-8 | CN shared identity / direct-message / private-channel scenarioとfixture | `persist_runtime_identity`→identity file、`cleanup_runtime_artifacts`→DB sibling/iroh-data/secret prefix削除、`remove_sqlite_runtime_db`→DB/WAL/SHM削除 | identity seedはCN runner2site、cleanupはCN2/DM2/private3/tests-private3site、DB-only除去はprivate runner1site。artifact領域とDB-only/secret含むcleanupの区別を維持 | `runtime.rs`、各scenario/tests。今回fixtureの削除処理を変更しない |

`write_result_artifact`の全runnerはcommunity_node_index、desktop_smoke、community_node_trust_relation、community_node、direct_message、device_backup、dome_hosting、private_channel。
上表は登録単位の有限監査であり、全source行を逐一精査したとの主張ではない。変更する子Issueでは当該sinkと全callerを再固定する。

test-supportの公開memberは`TestResource`、`lock_test_resource`、`try_lock_test_resource`、`PollState`、`PollError`、`poll_until`、`constrained_timeout`、`env_flag_enabled`、`gated_env_url`、`TopicSyncSnapshot`、`SyncSnapshot`、`SyncStatusSource`、`format_sync_snapshot`。resourceはProcessEnvironment/IdentityStorage/IrohNetwork/DhtTestnet/CommunityNodeServerの5種。
全callerは`rg -n 'kukuri_test_support::|use kukuri_test_support' crates xtask --glob '*.rs'`と、これら公開名の完全symbol検索でre-exportとwrapper後も列挙する。主要ownerはdesktop-runtimeのtests/support、app-apiのtests/supportとsync、harnessのtests/waiters、CNのintegration fixtureである。
fixture sinkは`rg -n 'persist_runtime_identity|cleanup_runtime_artifacts|remove_sqlite_runtime_db' crates/harness/src --glob '*.rs'`でlib再exportと全callerを再生成する。

## 変更圧力と前回責務mapとの照合

`git log --since=2026-05-29 --format= --name-only -- crates/harness`を非空pathで集計した変更commit数は、runtime 14、scenario schema 12、CN index scenario 12、scenario_steps waiter 9、desktop_smoke runner 7、CN connectivity runner 5。rename追跡を加えない同一pathでの集計であり、不具合数ではない。

- 2026-05-28のwave 2で行ったwaiterのdomain分割は維持され、7/11の`303cc448`で手書きpollは`poll_until`へ共通化された。前回の未完候補として再起票しない。
- 前回のapp-api長時間test問題は`0fe05b90` (#560)で`iroh-integration-tests` feature / slow laneへ分離済み。現在の失敗は旧timeoutとは別の観測である。
- CN connectivity runnerは#861で同意UX、#869でpolicy文書、#877で送信境界修正が同じ関数へ波及している。しかしstack lifecycleは既に別ownerで、2 identity modeは意図した共有。現時点で抽出後の構造成果と負の経路保護を十分具体化できない。
- desktop_smoke runnerには#799/#800/#792/#821/#823のDome操作が追加されている。scenario DSL dispatchは意図的な集約であり、行数だけの責務分割は行わない。

## 候補分類

| 候補 | 観測と目標候補 | 優先順位 | 分類 | 理由・次の条件 |
| --- | --- | --- | --- | --- |
| H-01 | CN connectivityの同意・reconnect段階を別ownerへ抽出 | P2 | 延期 | #861/#869/#877の変更圧力はあるが、2 identity modeとerror/shutdownの保護を保った独立成果が未確定。次のCN scenario追加時に再評価 |
| H-02 | desktop_smokeのDome stepをdomain runnerへ分割 | P2 | 延期 | 7変更commit、1016行はsignal。現行DSLの単一dispatchが妨げた具体的変更漏れを未観測。Dome session製品側の候補を先に扱う |
| H-03 | waiter再分割、test helper全面統一、slow testの再分離 | — | 却下 | 既に責務分割・poll共通化・slow lane分離済み。現行成果をやり直さない |
| H-04 | Metaverse irohイベントtestのmanifest未取得失敗 | P1 | 別種別: fix | 下記の2 Nightlyで同一test・同一エラーを観測。原因の診断と失敗再現を独立Issueへ分ける。testのskip/期待値弱体化は不可 |

## 既知失敗・長時間検証

- [9/5 Nightly](https://github.com/KingYoSun/kukuri/actions/runs/33988557825) (`d2414e5a`) と [9/6 Nightly](https://github.com/KingYoSun/kukuri/actions/runs/34056551815) (`fe156251`) で `service::tests::game::metaverse_room_events_replicate_between_iroh_peers` が失敗。
- どちらも `crates/app-api/src/tests/game.rs:141` の `list_game_rooms(...).expect("list rooms")` が `Dome preset manifest is unavailable` でpanic。192 PASS / 1 FAIL、test実行時間は36.24秒 / 34.83秒。ジョブtimeoutではない。
- 現行sourceでは `create_metaverse_room` → receiverの`list_game_rooms`を60秒loop → `publish_metaverse_room_event`へ進む。manifest不足が最初の取得でエラーになると、イベント送信へ到達する前にexpectで終了する。blob取得競合かfixture不足か製品不具合かは未確定。
- 2026-09-08、変更前の `ac9946d6` / Windows / Rust 1.92で `cargo test -p kukuri-app-api --features iroh-integration-tests metaverse_room_events_replicate_between_iroh_peers -- --nocapture` を実行し、同じ141行・同じエラーを再現した。exit 101、0 PASS / 1 FAIL / 192 filtered、test 0.30秒、compile 2分06秒。修正は今回行わない。
- `git diff fe156251..ac9946d6 -- crates/app-api/src/tests/game.rs crates/core crates/transport crates/docs-sync crates/metaverse-host`は差分なし。全依存・OS・実行状態の同一性まで意味しない。
- 最新Fastの成功はslow featureの成功を含まない。関連refactorでこの既知failが再現した場合も、既知failと新規回帰を区別し、必要な境界test未確認をgreenに読み替えない。
- CN scenarioのYAML時間上限はpublic 480秒/step120秒、multi-device 360秒/step120秒。CIではrunnerがoverall最低600秒/step最低180秒へ引き上げる。実P2P/relay/DBを使うためFakeNetworkのe2e-smokeで代替しない。

## 今回の終了条件

8 inventory行と4候補の分類を記録した。候補未分類0。製品変更・scenario変更・CI変更は0。子Issueの実装・独立監査とcampaign完了判定はPhase 3/4へ残す。
