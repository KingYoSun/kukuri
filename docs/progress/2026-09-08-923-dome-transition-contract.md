# Issue #923: Dome遷移の取消・ack喪失・source cleanup contract

- Scope revision `2026-09-08-872-C-MV-v1`、区分C、before `36e3732edfb68e0637a60e7badf4c778d0530e4d`。
- 対象は既存session testへの追加と本記録のみ。製品hook/recovery helper、既存14 testsのassertion、IPC/authority、UI描画変更0。後続#924の前に現行挙動を固定する。
- 正本は#923 AC1〜4/INVAR1・2/INV1〜4/TR1〜4、ADR0042/0044/0045。commit後の新しい取消やretry上限は追加しない。

| 作業 | AC / invariant | 検証・証拠 | 依存 |
| --- | --- | --- | --- |
| T1 baselineとcaller | AC-4、INVAR-1/2 | session/recovery2 files14 tests before、prepare/commit/abort/last-visited/source input/presenceのcaller | なし |
| T2 取消・commit結果不明 | AC-1/2、TR-1/2/3 | 下記4case、既存helper4 tests。public session hook→実actions→API mockまで通す | T1 |
| T3 確定後cleanup | AC-3、TR-4 | 成功/全失敗2case、target選択とlast-visited、RPC入力/回数/sequence/presenceを観測 | T2 |
| T4 検証・独立監査・merge | AC-4、全INVAR | targeted→desktop-ui-check→固定head独立監査/CI/merge tree確認 | T3 |

## 固定集合とtestの対応

| INV / TR | 新規testと観測 |
| --- | --- |
| INV-1/4、TR-1 | `cancelling prepare aborts a late ticket ...`: 準備中に中心線から戻る→source abort、ticket遅着後target abort/同id source abort。commit/completeなし、source選択・last-visited維持。既存join/leave自身の副作用を禁止する新仕様は置かない |
| INV-2、TR-2 | `lost ack and hosting lookup failure ...`: 最初commit失敗+hosting取得失敗→同一ticket/transformで次commit待機。ack前に中心線から戻ってもabort/completeなし。ack後にtarget選択・last-visited・source complete |
| INV-2、TR-3 | `... rolls back without a successful handoff`: invalid-ticket確定拒否とtarget session置換の2case。commit1、target/source abort、error、source選択・last-visited維持、completeなし |
| INV-3、TR-4 | `source cleanup retries ...`: 1回目失敗→250ms後成功、または250/1000ms後も失敗。2/3回で停止しtarget/last-visited維持、abort0、同transition id・重複しないinput sequence。成功時のみsource presence_leave、全失敗時はerror表示 |

元のsession10 testsとrecovery4 testsを維持。特に既存Return Homeは別joinのauthoritative admission後の選択・last-visitedを保護する。TR-1は取消対象attemptの副作用だけを禁止する。

## 全caller・凍結境界

session hookの本番呼出はMetaverseRoomPanel。prepare/abort/commitはsession callbacks→`createMetaverseRoomActions`→DesktopApi。commit recoveryの本番callerはsessionのcommit callbackだけ。
`submitInputForRoom`はtransition以外のjoin/leave/keepalive/移動も共有し、sequence正本を増やさない。last-visitedはadmission成功とcommit成功だけ、source presence_leaveはcleanup成功だけ。
CodeGraphのroot sourceとworktree（indexなし）の同一sourceを確認。補完は `rg -n 'recoverDomeTransitionCommit|prepareTransition|commitTransition|abortTransition|submitInputForRoom|writeLastVisitedDome' apps/desktop/src/components/extended/metaverse`。製品inventory増減0、対象未分類0。

## 実行証拠

- before: `npx pnpm@10.16.1 --dir apps/desktop test src/components/extended/metaverse/useMetaverseRoomSession.test.tsx src/components/extended/metaverse/DomeTransitionCommitRecovery.test.ts` →2 files/14 tests PASS。
- 追加後同command: 2 files/20 tests PASS（4.83秒）。fake timer/deferred responseで非同期の観測点を固定する。製品private helperをmockせずAPIとstateを観測。
- 初回のpresence_leave検査はAPI引数のtopic位置を取り違えて失敗したため、実際の5引数契約（topic/room/peer/sequence/event）に訂正。publish mockの戻り値も既存mock実装を使い、型契約を維持する。
- 必須 `cargo xtask desktop-ui-check` は対象worktreeで実行。最終結果/CI/独立監査headはPR/Issueへ保存。Windowsのvisualはsmoke、Linuxのpixel比較はCIで確認する。
- test-onlyのためOS/WebView動作を変更していない。#924の製品抽出時に必要な実機確認を本結果で代替しない。

Rollbackはtest-only PRを単独revert。#924着手後は先にrefactorを戻し、contractだけを外さない。必要な検証・独立監査成功前にCompleteとはしない。
