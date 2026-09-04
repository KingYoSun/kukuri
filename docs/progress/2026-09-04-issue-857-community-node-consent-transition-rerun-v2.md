# #857 Community Node同意transition再実行 v2

## 対象

- Scope revision: `2026-09-04-issue-857-transition-rerun-v2`
- 基準 commit: `db3585c5`
- リスク区分: `C`
- Goal: Community Node sessionが`Ready`でない状態から、保護対象HTTP通信またはNode由来relay／seed適用へ進めないようにする。
- Non-goals: server側#860、report経路の直接preflight方式の変更、将来route、legacy互換、一般的なsecurity hardening、広範なrefactor。

## failing-before

基準 commit上で回帰testを先に追加し、次の3経路が失敗することを確認した。

| transition | test | 修正前の観測 |
| --- | --- | --- |
| `TR-1` retry deadline内 | `retrying_session_preflights_and_stops_index_query_before_http` | index queryが保護対象HTTPまで到達した |
| `TR-2` `AwaitingAdmission`中のpolicy変更 | `awaiting_admission_manual_refresh_rechecks_policy_and_blocks_protected_http` | manual refresh後もindex queryが保護対象HTTPまで到達した |
| `TR-3` local consentのみ | `connectivity_apply_ignores_local_consent_without_verified_ready_session` | 保存済みrelayがruntimeへ適用された |

いずれも修正前はexit code 101、修正後は成功した。

## 実装と受入条件

| 条件 | 実装 | 実行可能な根拠 |
| --- | --- | --- |
| `AC-1` | `ensure_community_node_session*`が`Ready`／`Deferred`／`ConsentRequired`を返す。保護対象callerは`Ready`だけでtoken読込とHTTP処理を続行する | retry中のindex query、indexing request、trust／relation、tester feedbackのnegative test |
| `AC-2` | session guard取得後、retry deadlineと`AwaitingAdmission`のearly returnより前に公開current policy preflightを行う | `retrying_session_preflights_and_stops_index_query_before_http`、`awaiting_admission_manual_refresh_rechecks_policy_and_blocks_protected_http` |
| `AC-3` | `Ready`到達時のlocal consentをruntime-only evidenceとしてNodeごとに保持し、現在のlocal consentとの完全一致を要求する | `connectivity_apply_ignores_local_consent_without_verified_ready_session` |
| `AC-4` | `active_community_node_connectivity_config`が検証済みevidenceを持つNodeだけを残す。初回`Ready`後にrelay／seedを再適用する | `connectivity_apply_keeps_only_the_node_with_verified_ready_session`、既存startup／metadata／connectivity test |
| `AC-5` | 拒否経路は保護対象HTTP hitを0件にし、未検証Nodeのrelay／seedを適用しない。正常経路とheartbeat再queueも維持する | 追加negative test 7件、既存Community Node test、実connectivity scenario |
| `INVAR-1` | 公開manifest／policy取得以外は、active local consentとcurrent policy照合済みevidenceの成立後だけ開始する | caller別negative test、startup／mixed-node test、既存report／Dome test |

## 固定surface inventoryの再構築

### `INV-1`: manual refreshとmaintenance

- `refresh_community_node_metadata`は`ensure_community_node_session_with_mode(..., true)`を呼ぶ。結果にかかわらずstatus生成以外の保護処理を続行しない。
- `run_community_node_session_maintenance_once`から到達する`refresh_community_node_registration_if_due`は型付きoutcomeを受け取り、session helper外の保護処理を続行しない。
- preflightはretry／`AwaitingAdmission`判定より前に実行される。

### `INV-2`: 登録済みcommand 13件

`apps/desktop/src-tauri/src/commands/community_node.rs`と`apps/desktop/src-tauri/src/lib.rs`の登録点から次を再列挙した。

1. `refresh_community_node_metadata`
2. `submit_community_node_report`
3. `submit_community_node_tester_feedback`
4. `submit_community_node_indexing_request`
5. `search_community_node_index`
6. `discover_community_node_index`
7. `recommend_community_node_index`
8. `read_community_node_trust_user`
9. `read_community_node_relation_user`
10. `list_community_node_relation_neighbors`
11. `get_community_node_relation_optout`
12. `set_community_node_relation_optout`
13. `clear_community_node_relation_optout`

reportは既存の直接preflightを維持する。残る12 commandはmanual refreshまたは4つのshared caller groupを通じて型付きsession outcomeへ到達する。未分類は0件。

### `INV-3`: connectivityのglobal apply

`apply_runtime_connectivity_assist*`、`apply_effective_seed_peers*`、`force_apply_effective_seed_peers`の全callerを逆引きした。config／seed変更、startup、reconnect、metadata／rendezvous更新、session `Ready` transitionは、いずれも`active_community_node_connectivity_config`のNode単位filterを経由する。未分類は0件。

## 状態遷移の結果

| transition | 結果 |
| --- | --- |
| `TR-1` | retry deadline中でも公開policyを取得し、`Deferred`または`ConsentRequired`で保護対象callerを停止する |
| `TR-2` | `AwaitingAdmission`中のmanual refreshでもpolicy変更を検出し、sessionを`Idle`へ戻してconnectivityを無効化する |
| `TR-3` | restart／config変更／mixed-node global applyでは、検証済みNodeだけのrelay／seedを反映する |
| `TR-4` | 厳密一致同意と`Ready` evidenceがある正常経路、metadata refresh、heartbeat再queue、実peer接続を維持する |

## validation

- `cargo fmt --all -- --check`: 成功
- `cargo xtask check`: 成功
- `cargo xtask oversized-files`: 成功（追加testで一時的に1002行となった既存ファイルを999行へ収め、baselineは変更していない）
- `cargo test -p kukuri-desktop-runtime`: 176件成功
- `cargo xtask rust-test`: 本体776件、harness 22件、doc test成功
- `cargo xtask test`: 本体776件、harness 22件、frontend 1089件、doc test成功
- `cargo xtask tauri-check`: 成功
- `cargo xtask e2e-smoke`: 6 step成功
- `cargo xtask scenario community_node_public_connectivity`: 15 step成功
- `git diff --check`: 成功（Windows checkoutのLF→CRLF警告のみ）

## merge gate

区分Cのため、この記録だけでは完了にしない。固定したPR headを別コンテキストで独立監査し、`PASS`かつ必須CI成功となった場合だけmergeする。監査結果とmerge後tree比較はIssue commentへ記録する。
