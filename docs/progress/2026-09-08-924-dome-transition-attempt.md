# Issue #924: Dome transition attemptの所有者を抽出

- Scope revision `2026-09-08-872-R-MV-v1`、区分C、before `4e833af8cbdb7263a75ad17f54ee64be6e263471`。
- 先行#923はPR#934、merge `d9ea79be`。24 targeted・独立delta監査・最終CI・対象tree一致PASSを経て着手。
- 対象利用者はMetaverse参加者。移動/取消/退出の観測結果を保ち、非同期attemptの所有者を一つにする。HTML/CSS・描画・audio/scene/admission・input sequence・IPCを変更しない。

| 作業 | AC / INVAR / INV・TR | 証拠 | 依存 |
| --- | --- | --- | --- |
| T1 before baseline | AC3/4、全INVAR、INV1〜4/TR1〜4 | session/recovery/transition/entry/panelの5 files57 testsを製品変更前に実行 | #923 |
| T2 attempt owner抽出 | AC1/2、全INVAR | 同directoryのuseDomeTransitionAttemptのみ。入力・取消要求と確定handoff通知でsessionに接続 | T1 |
| T3 同contract・path validation | AC3/4、全INVAR | 同57 tests、typecheck、desktop-ui-check、Windows/WebView操作 | T2 |
| T4 独立監査/CI/merge | AC4、全INVAR | fixed head監査、最終CI、merge対象一致。結果はPR/Issueへ | T3 |

## 構造と凍結境界

- `attempt.(phase|cancelled|ticket)`と`transitionAttemptRef.current`の直接代入はsession9→0、専用owner9。mutable attemptを戻り値に含めない。
- prepare/commit/abort APIはsession各1→0、owner各1。source prepare/abort/completeとcleanup retryも同owner内。recoverDomeTransitionCommitの判断は変更しない。
- sessionからはtransform（直前値を含む）とjoin/leaveの取消要求を渡す。選択・handoff・joined setは同期callbackが既存順で反映し、その後ownerがlast-visited/onError/cleanupを続ける。
- 入力sequenceはsessionの既存MapとsubmitInputForRoomをそのまま共有。別の正本・追加state copy・公開API/依存追加なし。
- abort callbackの依存はactions/submitInputForRoomのまま。取消要求callbackもそれに連動し、joinRoomのeffect安定性を保持。非同期途中の古いclosureを最新参照へ勝手に変更しない。
- 全caller: session handleLocalTransform→handleTransitionTransform、joinRoom/leaveRoom→requestTransitionAbort、owner commit→既存recovery。source inputは既存submitInputForRoom経由。admission等の他caller無変更。
- before/afterは代入regex、prepareTransition/commitTransition/abortTransition/recover/submitInputForRoom/writeLastVisitedDome全参照とdiffで照合する。固定inventory4・transition4の増減0、未分類0。

## 維持する制御

precommit取消、遅着ticket abort、commit適用不明でabort禁止、同ticket/transform再送、確定拒否/置換後だけrollback、ack後handoff、source cleanup0/250/1000msの最大3回、成功時だけsource presence leaveを保持。join/leave自身のadmission/selection/audio副作用もそのまま。

既存Rust `desktop_smoke_metaverse_dome_transition` はsource prepare/target commit/source complete、entryはauthoritative entry/safe spawn、recoveryはtopologyの保護として対応する。hookの非同期競合とは区別し、Rust/scenario/DTO変更0のため追加scenario編集なし。

## 検証とplatform

- before: 5 files57 tests PASS（11.33秒）。after: 同57 PASS（10.14秒）。既存test/fixture/assertion差分0。typecheck PASS。
- 必須desktop-ui-check、Windows/WebViewのpointer/keyboard移動・退出・fullscreen/input ownership・resource縮退、CI・独立監査は残gateとしてPR/Issueへ記録する。nativeの成功をbrowser/hook結果で代替しない。
- 見た目・theme/locale/token/layoutは不変。UI実機はWindows、既存seed/隔離profile、同条件のbefore/afterで確認する。実音声/他peer接続の成否をmockや単一端末操作から推定しない。

Rollbackは抽出PRだけrevertし#923 contractを保持。retry/authority/取消の新仕様が必要になれば別fixへ分離する。
