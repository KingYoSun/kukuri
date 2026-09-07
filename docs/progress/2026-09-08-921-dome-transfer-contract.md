# Issue #921: runtimeのCN Dome transfer境界を固定

- Scope revision `2026-09-08-872-C-RT-v1`、区分C。before `36e3732edfb68e0637a60e7badf4c778d0530e4d`（#928の文書同期後、製品は監査開始ac9946d6と同じ）。
- 対象はtest fixtureと新規contract、記録のみ。後続#922の製品抽出前に実行する。全10IssueについてユーザーのCI/独立監査後merge承認あり。
- `dome_runtime`の既存fixtureへ追加routeを渡せるtest-only入口を用意し、既存6 testsは従来と同じ空の追加routeで実行する。
- 新規`tests/community_node/dome_hosting/transfer_contract.rs`はpublic runtime `delegate_dome_hosting` / `commit_dome_layout`を呼び、local HTTP受信、署名済みlease/acceptance/activation、保存済みhosting/manifest/layout operationを観測する。AppService直呼出は初期Dome/owner/inputの準備と保存状態の読取だけ。

| 作業 | AC / INVAR | 対象・検証 | 依存 |
| --- | --- | --- | --- |
| T1 基準・全caller | AC-1/3、INVAR-1 | public runtime2入口、Tauri/CLI登録、3 HTTP adapterとAppService authorityを確認、既存6 tests before実行 | なし |
| T2 途中失敗・retry保護 | AC-1/2、INVAR-1/2 | 下記failure matrix、HTTP回数・signed state・保存operation数 | T1 |
| T3 guard・必要な検証 | AC-3、INVAR-1/2 | missing/withdrawn/current policy/configured node、existing AppService contract、rust-test CI | T2 |
| T4 独立監査/CI/merge | 全AC/INVAR | 固定head、別担当監査、CI、merge tree照合。結果はPR/Issue | T3 |

## Inventory / transition / test対応

| 固定集合 | 実行証拠 |
| --- | --- |
| INV-1、TR-1/2/3 | `public_delegation_preserves_each_transfer_failure_boundary_and_retry`: success/assignment503/tampered acceptance/activation503。新epoch2の保存、activation送信回数、前段失敗でactivation未保存、後段失敗では保存済みactivation維持。新delegation retryはepoch3へ進む |
| INV-2、TR-1/2/3/4 | `public_layout_restart_preserves_transfer_failure_and_operation_retry`: 同4条件をlayout再起動で確認。revision2/operation1/epoch3の保存、同operation retryではrevision/operationを増やさない。未activation時は同leaseでtransfer再送、既にactivation保存済みなら再送しない |
| INV-2、TR-4 | `cn_noop_and_owner_layout_changes_do_not_send_transfer_requests`: owner no-opはoperation0、owner新規変更はrevision/operation/lease更新を許可しCN送信0、retryで追加保存0。CN no-opはcandidate取得だけでassignment/activationを増やさない |
| INV-3、TR-5 | `public_transfer_entries_cannot_bypass_current_consent_or_configured_node`: public delegationの未同意と、両入口の撤回/現行版変更/未登録拒否。新規assignment/activation/candidate0、challenge/verify不変。policy preflightの許可された取得は禁じない |
| INV-1/2/3、TR-3/5 | `activation_rechecks_current_consent_after_assignment_for_both_entries`: assignment応答を返すmockがpolicyを更新し、両入口でactivation自身のpreflightがCONSENT_REQUIREDを返す。owner activationは保存済み、activation HTTPは増えない |
| INV-3、TR-5 | 既存6 adapter testsは未同意、撤回、版変更、未登録、同意済み、401再認証を保持。authorityの署名/manifest整合は既存AppService2 testsとmock受信時のcore verifier・acceptance ID一致でも確認 |

後段activation HTTP失敗でもownerの署名済みactivationは保存済み。同operationのlayout retryは前段candidateを取得するがtransferを再送しない、という現行挙動をそのまま記録する。このcontractへrollback/retry policy修正を混ぜない。

## 全callerと凍結境界

CodeGraphでruntime2入口・共通CN adapter・core署名/候補検証・AppService layoutを確認。補完検索:

```powershell
rg -n 'delegate_dome_hosting|commit_dome_layout|assign_dome_hosting_to_community_node|activate_dome_hosting_on_community_node|build_dome_hosting_assignment_request' crates apps/desktop/src-tauri -g '*.rs'
```

Tauri `commands/live_game.rs`とCLI `commands/live_metaverse.rs`の2command登録→runtime→CN assignment/activationとAppService保存、CN layout candidateの経路を固定。request/response shape、error context、owner authority、同意guard、signed record、schema/public APIは無変更。製品inventory差分0、対象未分類0。

## Validation

- before `cargo test -p kukuri-desktop-runtime tests::community_node::dome_hosting`: 6 PASS（0.54秒）。
- 初回after: 既存6+新規4 tests PASS（5.93秒）。新規4 tests内で両入口の4failure条件と7guard条件等を実行。保存operation数のassert追加後の最終結果とCI/監査headはPR/Issueへ記録する。
- 独立監査で返却viewの直接比較とassignment後のactivate再guardの証拠補強を指摘。成功時の返却lease/session/署名recordと保存状態の一致を追加し、policy更新を跨ぐ両入口のtestを追加。既存6+新規5の11 tests PASS（8.28秒）、最終headでdelta監査する。
- 必須: 同targeted、`cargo test -p kukuri-app-api tests::dome_hosting`、`cargo xtask rust-test`。CIが全Rustの結果を担う場合は対象SHAとjobを記録し、local targetedを全suite成功と呼ばない。
- UI/IPC/製品source変更なし。静的確認は`git diff --check`と製品diff0。
- CI `34146718415`で`tests::support::lock_contract::lock_acquisitions_match_declared_classification`がFAIL。新testのCommunityNodeServer lock取得5箇所を分類表に反映していなかったため、明示entryを追加し総数123→128へ同期した。scan/全件一致のassertは保持し、未分類追加の検出を弱めない。該当testと全CIを修正headで再実行する。

## Rollback

このtest-only PRを単独revert可能。#922開始後は先にrefactorを戻す。新規contractと現行仕様の不一致が見つかった場合はfixへ分け、期待値を弱めて抽出を始めない。
