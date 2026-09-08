# #914 コミュニティノード初回案内・同意後の検索復旧

## 対象・固定条件

- Issue: [#914](https://github.com/KingYoSun/kukuri/issues/914)
- Scope revision: `2026-09-08-914-plan-v1`。2026-09-08に実装・commit・PR・CI成功後のmergeまで承認。
- 基準commit: `916f5e0a1c2a81cfe0f88aa463fd77cc1e793c19`。
- 区分: C。固定AC-1〜6、INVAR-1〜5、INV-1〜7、TR-1〜10の全文はIssue本文。計画原本は`.codex/plans/2026-09-08-issue-914-community-node-onboarding-and-index-recovery.md`。
- 現在判定: 実装・ローカル統合検証済み。初回独立監査の3件を修正し、差分監査と最終CIを待つ。
- 非目標: relay優先度変更、自動同意、明示選択先の無断fallback、既定Node復活、規約本文改定、本番Node設定変更、関連のないrefactor。

## 原因と変更

見つけるの受諾処理は`acceptCommunityNodeConsents`の返却statusを捨てていた。runtime eventはstatusを更新するが検索先を再解決せず、Tauriの60秒poll等まで検索先がnullに残った。また、manifest loaderは取得開始時の古いstatusで選択を決め、先に始めた古いmanifest取得が新しい成功結果を上書きできた。

以下の3つを、修正前の現行HEADに先行追加したtestの期待assertion失敗で再現した。

| test | 修正前 | 修正後 |
| --- | --- | --- |
| `a consent-ready runtime event selects the index node without waiting for polling` | Nodeがreadyでも選択がnull | timerを進めず選択が更新される |
| `manifest completion resolves the index using consent received after the request started` | 古い未同意statusでnullに戻る | 最新statusで選択される |
| `an older manifest response cannot overwrite a newer capability result` | 新しいokが古いabsentに戻る | 新しいokを維持する |

設定・受諾・event・poll・manifest更新時の検索先をstoreの同じ更新で導出する。初回configの読込前は保存済みmanual選択を空一覧で消さず、explicitな削除後のauto復帰契約を維持する。manifestの世代を管理し、statusの読込中に新しい受諾/撤回/eventが来たNodeを古いsnapshotで巻き戻さない。

初回案内は全設定Nodeのローカル同意状態を取得した後だけ判定する。active consentが1つでもあれば接続失敗中でも初回説明を繰り返さない。説明から規約へ進む対象は設定一覧のindex 0。「あとで」はアカウントごとの起動session中だけ記録し、永続同意にはしない。app/年齢/restore gateの後に動くshellへ配置した。

規約取得と受諾のUI flowは、対象Nodeと言語・表示snapshotを固定する。取得失敗/受諾失敗の再試行、Node削除、古いresponse、重複受諾を制御し、既存のshell受諾actionへ委譲する。Nodeの状態説明では同意、接続準備、retry期限、入場制限、manifest、検索機能非提供を区別する。query失敗の403をすべて同意不足扱いせず、既知の429 retry期限を同じNodeの操作で維持する。

初回独立監査では設定/Domeの旧policy応答、modal内受諾error欠落、共通受諾actionの遅延status反映をExisting-gapとして検出した。いずれも先行失敗testを確認して補完した。設定/Domeは`useCommunityNodePolicyDialog`で取得結果のviewと言語を保持し、共有cacheの後続更新から表示内容を分離する。shared fetchはpending entryの同一性を確認し、Node操作のRPCとevent/pollが競合した場合はmutation後のread-only statusで確定する。同じ反映を使うauth/invite/token/refresh/withdrawにも適用する。

ブラウザ全体検証で検出したモックの設定保存時の同意消失も修正した。runtimeの`set_community_node_config`は残存Nodeのlocal consentを消さないため、mockも同じNodeの状態を保持する。既存の狭幅smokeはassertionを変更せず成功した。新規recoveryの処理中に撤回された状態を古いmetadata応答で戻さないtestも追加した。

これは現行コードとtestで再現したclient側の原因であり、報告版`v0.2.0-preview.2`のDebian端末や本番Nodeの当時の状態を再現したものではない。本番の障害や検索提供の有無を推定で確定しない。

## Surface inventoryと逆引き

CodeGraphを先行使用し、動的なJSX/IPC登録は登録箇所の参照検索で補完した。CodeGraphのcaller結果だけではJSXやTauri commandの全利用者を列挙できないため、以下のnamed inventoryで扱う。

| ID | 入口と所有者 | sink/更新 | guardと確認先 |
| --- | --- | --- | --- |
| INV-1 | `App` ready→`DesktopShellPage`→`CommunityNodeOnboarding` / `useCommunityNodeOnboarding` | 一時表示state、既存規約flow | 全local状態確認、一覧index 0、他modalの終了待ち。onboarding unit/shell/browser |
| INV-2 | `CommunityIndexWorkspace`、`CommunityNodePanel`、初回案内→`handleAcceptCommunityNodeConsents` | public policy、受諾IPC、status/config | `useCommunityNodeConsentFlow`と設定/Domeの`useCommunityNodePolicyDialog`のtarget/language/generation、shared fetchのpending entry、RPC競合時のread-only status。consentFlow/SettingsPanels/actions |
| INV-3 | `useRuntimeEventBridge`→`useDesktopShellDataEffects`、`useConnectivityStatusRefresh` | status merge→`store.withCommunityIndexSelection` | 最新のconfig/status/manifest/preferenceで解決。event-only/deferred loader |
| INV-4 | `loadCommunityIndexCapability`、設定save/clear/auth/refresh/invite/token/withdrawの既存actions、新規`useCommunityNodeRecovery` | config/manifest/status、既存metadata IPC | loader世代、Node在籍、active local consent、retry期限。loader/actions/runtime |
| INV-5 | `DesktopShellColumnWorkspace` header、設定のauto/manual、Index検索/発見/おすすめ | 選択保存、既存query IPCと結果詳細 | `eligibleCommunityIndexNodes`を維持、manual無断fallback禁止、request/contextによる旧結果の無効化。preference/workspace/shell |
| INV-6 | Tauri `commands/community_node.rs` / CLI `commands/community_node.rs`→`runtime/community_node_api.rs`、session scheduler | local暗号化consent/token、auth/consent/heartbeat/bootstrap、relay/seed適用 | Rust実装は変更なし。既存preflightが支配するsinkへの禁止HTTP/変更0をruntime93 testsで確認 |
| INV-7 | `CommunityNodeConsentDialog`: Index/Settings/DomeHosting/新規Onboarding。`eligibleCommunityNodes`: Index/Trust/DistanceOptout/TesterFeedback | 各既存機能の固有操作 | 同意Dialogはerror・focus・空文書guardを補い、eligible helperの条件を緩めない。既存利用側testも実行 |

変更前の入口6群に初回案内1群を追加。既存入口は削除しない。APIの追加、保存schemaの変更、Rustの送信sink変更は0。actor/token/同意をNode間で共有する変更もない。

逆引き起点:

- `accept_community_node_consents`: Tauri command登録/CLI handlerとruntime実装。runtimeのlocal保存→session 1 tick→status返却は不変。
- `persist_community_node_local_consents`: runtimeのaccept/withdrawのみ（module importを除く）。既存暗号化storeが正本。
- token/auth、`request_accept_community_node_consents`、query HTTP、bootstrap/relay/seed: 既存`community_node` moduleのpreflight/session/requestsとconfig適用をruntimeのpositive/negative testで確認。初回説明からこれらへ直接の新callerを追加しない。
- `eligibleCommunityNodes`: `eligibleTesterFeedbackNodes`、`eligibleCommunityIndexNodes`、`eligibleTrustRelationNodes`、`eligibleDistanceOptoutNodes`の4群。既存条件は不変。

## AC/INVARと証跡

| 条件 | 実装とtest | TR |
| --- | --- | --- |
| AC-1 / AC-6 | `store.ts`、loader世代、受諾action配線。`useDesktopShellData.test.tsx`のevent-only再現、section loaderのdeferred再現、`DesktopShellPage.communityNodeOnboarding.test.tsx`のtimerなし受諾→3操作 | TR-1,4,8,10 |
| AC-2 | `communityNodeAvailability.test.ts`の理由別判定、`CommunityIndexWorkspace.test.tsx`、既存preference test、onboardingのoffline/retry表示 | TR-4〜8 |
| AC-3 / AC-4 | `firstUnconsentedCommunityNode`の順序/欠落/未取得/混在/撤回/token-only、shellの「あとで」/再起動/手動再開、browserの12条件 | TR-1〜3,9 |
| AC-5 | `useCommunityNodeConsentFlow.test.tsx`、`useCommunityNodePolicyDialog.test.tsx`のpublic GET失敗、言語/Node変更、snapshot、二重受諾、失敗、削除。`CommunityNodePanel.consent.test.tsx`とDomeのmodal内失敗、既存consentGate | TR-4,5,10 |
| INVAR-1 / INVAR-4 | shell/browserで説明表示中のpolicy/auth/accept 0、先頭のみ受諾。runtime `node_without_local_consent_is_never_contacted`、`policy_update_is_not_silently_reaccepted`、`saved_token_does_not_bypass_same_version_snapshot_preflight`ほか93件 | 全TRのNode境界 |
| INVAR-2 / INVAR-3 | auto/manual既存test、複数Nodeの非対象未同意維持、runtime config/session/admission/index_query。app/restore gateは既存コードを維持 | TR-2,3,8,9 |
| INVAR-5 | browserのDialog単一性・focus・keyboard/Escape/200% zoom、既存Index context無効化test。3 locale×2 theme×2 widthのgeometry | TR-3,8,10 |

## 検証記録

| 検証 | 現在の結果 |
| --- | --- |
| 修正前targeted Vitest | 3件が期待assertionで失敗（event選択null、古いstatus選択null、古いmanifest上書き） |
| state/loader修正後 | 22 tests PASS |
| 既存Index/設定/同意導線のtargeted | 42 tests PASS |
| 理由判定・規約flow・既存query/data targeted | 61 tests PASS |
| 新規shell onboarding | 3 tests PASS（timerを無効化して受諾→検索/発見/おすすめ） |
| runtime `tests::community_node::` | 93 tests PASS。禁止HTTP/同意更新/入場制限/複数Node/config/検索等。Rust source変更なし |
| `cargo xtask doctor` | PASS |
| `cargo xtask oversized-files` / `git diff --check` | PASS。既存の大型ファイル警告は維持、ratchet増加なし |
| `cargo xtask desktop-ui-check` | `bf9df4a8`でPASS。lint/typecheck、157 files / 1258 tests、Storybook、browser 81件、visual到達16件。Windowsのpixel比較はskip |
| 監査修正の先行test | event後の遅延受諾、設定の古いpolicy、modal内受諾errorの3件で期待assertion失敗を確認 |
| 監査修正targeted | settings/Dome、共通policy controller、shell actionsの43 tests PASS。表示言語をcallbackの期待引数へ追加し、既存assertionを維持 |
| `cargo xtask check` | PASS（Rust clippy、Tauri check、frontend lint/typecheck） |
| `cargo xtask rust-test` | PASS（non-CN、serial harness、doctests） |
| UI実機・画像 | Windows Tauri/WebViewの隔離mock fixtureで日本語の説明→先頭Node規約→同意→検索の成功した空結果を確認。画像はUI採用記録へ集約 |
| 独立監査・CI | 初回FAILの3件を修正済み、delta判定/最終CI待ち |

初回browser実行は日本語/英語とerror/zoomの10件が成功し、中国語4件はテスト側のbutton名が既存訳`接受`ではなく`同意`だったため失敗。既存訳に修正して再実行する。製品の訳をtestに合わせて変更していない。

修正後、3 locale×2 theme×2 width、error/zoom、既存狭幅smokeの15件が成功し、後続のbrowser全体81件も成功した。Linux visual baselineは[run 34202969424](https://github.com/KingYoSun/kukuri/actions/runs/34202969424)で生成し、追加した2枚だけを採用。CI runnerには日本語glyphのfontがなく初回日本語baselineが豆腐表示だったため、その画像は採用せず、既存運用と同じ英語baselineに揃えた。日本語の実描画はWindows WebView画像で確認した。

## UI確認・残る限界

- 採用記録: [UI review](../ui-reviews/2026-09-08-914-community-node-onboarding.md)。brief、文言、データ境界は`DESIGN.md` §4.3とNode法務データ分類へ反映。
- Debian 13の実機と本番Nodeを使った報告sequenceは未確認。Windows/Chromiumの成功で置き換えない。
- Windowsのvisual comparisonは設定上skip。Linux/Chromium baselineは専用workflowで作り、CIで比較する。
- 完了条件を満たすまでIssueをCloseしない。区分Cの独立監査はPR headを固定して別工程で行い、監査後のcode deltaは再監査する。
