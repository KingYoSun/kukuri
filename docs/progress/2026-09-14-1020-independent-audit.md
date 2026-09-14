# #1020 独立監査

## 現在判定: 9382357a delta監査

- 対象commit: `9382357a`（比較元 `e5ced6b0`）、Scope revision: `2026-09-14-r1`、区分C。
- 判定: **FAIL**。B-1のassign/activate/release競合はコード上解消したが、runtime removalの逆引きで同じ境界を迂回するstatus入口が残る。
- inventory: 8群 / 適合7 / 不適合1 / 未分類0。群3へstatus poll/GETのruntime削除を追加し、不適合理由を下記B-1残存入口へ更新する。

### 修正されたB-1経路

`lock_hosting_lifecycle`のcallerはassign/activate/releaseの3件。各handlerが認証・同意後、DB read/write、pin mutation、runtime insert/removeより前にprocess MutexとInstance IDのPostgres advisory transaction lockを取り、全区間保持する。トランザクション値を関数末尾まで保持するため、DB closeとruntime removalの間で新assignment/activationが割り込まない。advisory lockにより別server stateでも同DBの同Instance pin操作を直列化する。SQLパラメータ化とlock取得順も確認した。

`dome_hosting_delete_tests.rs`は実handlerのDB close後にNotify barrierを置き、generation/epoch 2のassign→activateを交差させ、最終runtime epoch=2と新manifestのactive_lease pin=1をassertする。同processと別DomeHostingNodeStateの2caseがあり、単なるエラー値検証ではない。実装担当から、修正前runtime Noneの失敗とprocess Mutex修正後の1case成功を受領。9382357aの2case最終実行はこの時点では未確認。300ms timeoutは競合実行の猶予であり、全件実行成功の証拠とは区別する。

### B-1残存入口: 旧status readによる新runtime removal

- 固定条件: INVAR-1/2、INV-3のpollと群3のsensitive sink逆引き、Existing-gap。
- `crates/cn-user-api/src/dome_hosting.rs:474`でassignmentをDB readし、485行でsessions lockを後から取得する。486～487行はread済みassignmentの期限だけを根拠にInstance IDでruntimeをremoveする。新lifecycle lockにもruntime epoch照合にも参加していない。
- sequence: 期限切れgeneration G/epoch Eをstatusがread → statusをsessions取得前で停止 → 削除後再作成されたG+1/E+1のassign/activateが新runtimeを格納 → status再開 → 旧assignment.expires_atを使い新runtimeを削除。
- 影響: DB上activeの新Domeがauthoritative runtimeを失い、新入室/inputが失敗する。期限切れ旧readから新世代へのmutationであり、B-1と同じ保護境界。
- 根拠: 到達可能な制御フローとinterleaving。動的barrier testは未実行。修正はstatus readからremoveまで共通lifecycleに参加させるか、remove対象を現在runtimeのlease/epochで検証する。新世代runtimeが保持される回帰testが必要。
- 他のruntime remove callerも分類済み。submit_dome_hosting_inputの期限処理はsessions lock内で現runtime自身のlease.expires_atを検証するため、この旧DB snapshot競合に該当しない。restoreはstartupの復元であり旧releaseの後処理ではない。

### その他deltaとvalidation

Managerのstop表示条件はleaseにactive/admittedを追加しただけで、backend owner guardは維持。Storybook3状態はmockのみ。scenario21件の台帳とnightly配線は一致し、新製品入口を増やさない。oversized baselineは理由付きの変更6pathに限定されている。

実装担当からCN release retry contract、browserの管理→停止→cancel→削除→再作成1test、新harness delete 7steps成功を受領した。監査者はこのdeltaでbuild競合を避けるため重いtestを重複実行していない。全CI、Windows/Linux実機、全必須gateは別ゲートとして未確認。コード上の残存B-1を解消した固定headでdelta監査を続ける。

## 初回監査: e5ced6b0（当時の判定を保持）

- 対象: PR #1026、commit `e5ced6b0`、base `64a1b4729a197e4131703bf724e574f657716e27`
- Scope revision: `2026-09-14-r1`
- リスク区分: C
- 監査者: 実装担当と別コンテキストの独立監査agent
- 判定: **FAIL**。下記B-1の修正とdelta監査が必要。
- inventory: 8群 / 適合7 / 不適合1 / 未分類0。実行中の全体validationと実機判定をこの件数に含めない。

## 方法と対象

Issue本文の固定AC-1～4、INVAR-1/2と登録点から再構築した。実装progressは主張として扱い、成功の前提にしなかった。CodeGraphのexplore/nodeを先に使用し、macro、文字列IPC、method callerの欠落をrgで補完した。製品コード、git index、commit、公開状態は変更していない。

| 群 | 入口 → helper → sink、分類 |
| --- | --- |
| 1 / INV-1 | Discovery作成・管理・カード、Panel、再起動後一覧 → managedScope/managedSnapshot、list_game_rooms_scoped → 管理選択とcanonical Instance read。適合。管理選択でstart/Join/deleteを呼ばない |
| 2 / INV-2 | Management確認、shell deleteRoom、runtimeApi、Tauri live_game/delete_dome、CLI Destructive delete_dome → DesktopRuntime/AppService delete_dome → journal、close、Connection terminal、Instance/game manifest、cache、hint。owner/Context/generation/operation guardはwriteより前。適合 |
| 3 / INV-2追加逆引き | DesktopRuntime delete_domeと既存close → release_dome_hosting_on_community_node → send_dome_hosting_request → CN release → assignment/pins/runtime removal。不適合、B-1 |
| 4 / INV-2共通helper | create_metaverse_room_scoped、update_metaverse_room、import_metaverse_room_asset、move_dome、start/prepare/activate/close hosting、commit_dome_layout → dome_mutations。内部unlockedはlock所有caller限定。適合 |
| 5 / INV-2関係 | delete → list_dome_connection_topology/terminate_dome_connection_with_reason → 対象generation/owner endpointのterminal record、active topology。空topologyと未取得Instanceを扱う。適合 |
| 6 / INV-1・3 | 明示Join、初回auto entry、Return Home、session input → admissionAttempt/authoritative snapshot → admittedRoom/presence/audio。managementActiveでauto entryを抑え、authoritative admission前にsceneを開始しない。適合 |
| 7 / INV-2 restart/retry | PendingDomeDeletions、list_pending_dome_deletions、再起動 → Context journal署名/identity照合 → exact request retry。未完了tombstoneでも回復対象を保持。適合 |
| 8 / INV-3 | getHosting、refresh/poll、Discovery/Management/Controls → hosting kind/admission/recovery/外部接続表示。稼働と外部接続、停止と削除を分ける。適合 |

## B-1: 旧CN releaseが新generationのruntimeを削除できる

- 分類: Existing-gap、INVAR-1/INVAR-2、INV-2のCN sink逆引き、TR-2/3。merge blocker。
- 入口: 新規delete APIはlocal完了後もCN releaseが進行する。`crates/desktop-runtime/src/runtime/private_channels_game_api.rs:17`でAppService lockが解放され、19～31行のnetwork呼出しへ進む。`crates/app-api/src/game.rs:305`はlocal journalがcompletedなら同IDの次generation作成を許す。
- 問題箇所: `crates/cn-user-api/src/dome_hosting.rs:399`～417。epoch付きDB closeの後、pins解除をawaitし、最後にInstance IDだけで`sessions.remove`する。`activate_dome_hosting`の318～330行は別の並行要求から同IDの新runtimeを挿入できる。
- 具体的sequence: G/Eのdeleteがlocal completedとなる → CN releaseがEのDB closeを成功させる → 旧releaseをpins解除の前/途中で停止 → 同ContextにG+1を作成しE+1を同CNへassign/activateして新runtimeを格納 → 旧releaseを再開 → 417行が新runtimeを削除する。
- 利用者影響: 新DomeはCN上のDB assignmentがactiveのままauthoritative runtimeを失い、入室/inputが失敗する。旧retryが新世代へ作用しないという固定境界を満たさない。
- retention側も`crates/cn-core/src/dome_hosting.rs:209`の解除がInstance/epochを照合せずreference IDで削除する。CN reference IDは`instance_id:preset.revision`であり、新generationのrevisionが同じ場合に旧releaseが新active_lease pinを落とせる。current pinまで消すと断定はしない。
- 現在の証拠: 制御フローと実行可能なinterleaving。barrier付き動的再現は未実行。既存のepoch比較（CN 359～366行）とDB WHERE epoch（cn-core 393行）は、DB close成功後の競合を防がない。
- 必要な最小検証: DB close後をbarrierにし、新generation assign/activateを交差させて旧releaseを再開するcontract。新runtimeのepoch/session、assignmentとactive lease pinsが不変であることを確認する。対策はinstance単位の共有排他または全mutationのepoch条件付け。単なる逐次stale release拒否testでは代替できない。

## AC / INVARと状態遷移の根拠

| 条件 | コード・test evidenceと判定 |
| --- | --- |
| AC-1 | 停止ownerのManage Dome、独立管理対象、明示startからjoin。DiscoveryとManagementの独立実行17 tests成功。最終headのWindows再起動実機は実装担当の確認待ち |
| AC-2 | confirmation/cancel、stable operation、local tombstone後新generation、未完了journal/retry。app-api削除4 testsを読み、実装担当からDome関連22 tests成功を受領。CN並行cleanupはB-1 |
| AC-3 | Management.enterはstart失敗/Join失敗を区別し、Join再試行でhostingを再開始しない。PanelはadmittedRoom成立後だけstage scroll/focus。独立Management test成功。実機focus確認は別ゲート |
| AC-4 | host kind、開始中、入室済み、recoveryと外部peerを別表示。Discoveryの参加済みbuttonは無効化。所有制限は管理導線付き。独立Discovery test成功 |
| INVAR-1 | 管理readのみ、owner/context guard、historical lease署名とidentity検証、単調epoch、active generation session check、authoritative Joinを確認。ただしCN runtimeのB-1により不適合 |
| INVAR-2 | AppServiceの旧completed retryは新Instanceをwriteしない。Context replicaを固定、private write authorityを使用し、他owner journalを一覧から除外する。Presetを一括削除せずremote copy消去も約束しない。ただしCN cleanupのB-1により不適合 |

TR-1のfresh→create→管理→start→admission、TR-2のrestart→管理/削除→再作成、TR-3のowner/Context/generation拒否・cancel・部分write失敗・同operation retryはコードと対応testへ照合した。docs書込み失敗testは停止時5点/稼働時7点を注入してAppService再構築後にretryする。CN release failureの新testは署名済みcloseの同一性、local完了とcleanup pendingを確認するが、B-1の並行状態を覆っていない。

CN releaseのconsent gateはauth token取得・POSTの前。401では既存再認証経路を通る。別nodeへのfallbackはない。拒否Contextは新規topic購読を作らず、削除guardのnegative testはdocs全体不変と購読不増を比較する。backend拒否とUI cancelを同じ証拠として混同しない。

## 実行validationと限界

- 監査者独立実行: `npx pnpm@10.16.1 exec vitest run src/components/extended/metaverse/DomeManagementPanel.test.tsx src/components/extended/metaverse/MetaverseRoomDiscovery.test.tsx`。2 files / 17 tests PASS（2026-09-14 15:28 JST）。
- 実装担当から受領: `cargo test -p kukuri-app-api dome_ --lib` 22 tests PASS、frontend targeted 18 files / 127 tests PASS。監査者による重複実行はしていない。
- CN新contract、harness新scenario、全CI、Windows最終head実機はこの監査時点で実行中/未確認。未確認をPASSとみなさない。
- 対象commit後の作業tree差分はこの判定に含めない。修正後は固定headのdelta監査が必要。

## Non-blocker

- local削除済み・CN cleanup失敗後に同CNへ新epochを割り当てると、旧releaseはepoch拒否されpending表示が残る。旧世代pendingの自動解消は固定要件ではなく、新generation保護を優先する現行挙動として記録する。local削除・新規作成自体は妨げない。
- Dome削除で独立owner資産のPresetまで破壊しないこと、remote peer copyの消去を保証しないことはADR 0036/0040の対象境界に適合する。

判定をPASSへ進めるにはB-1の再現・修正・回帰testとdelta監査が必要。全体CIと実機ゲートは別途完了させる。
