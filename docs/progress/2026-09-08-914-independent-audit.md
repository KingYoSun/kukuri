# Issue #914 / PR #946 独立delta監査

- 対象 commit: `e0200108793f0dabcf7290ede797cb5fe1a9db46`
- delta base / 前回監査対象: `bf9df4a8e7feca1a758dbd8fdd848fe0468a3ac1`
- 実装の比較base: `916f5e0a1c2a81cfe0f88aa463fd77cc1e793c19`
- Scope revision: `2026-09-08-914-plan-v1`
- リスク区分: C
- 判定: **PASS**
- blocker: **0件**。前回F1〜F3は解消。
- inventory: 合計7群 / 適合7 / 不適合0 / 未分類0。入口群の追加・削除なし。INV-2/3/7の共通helperと影響callerを再監査。
- 本記録は固定headのコードdelta独立監査。最終CI・OS実機・browser/Storybook gateの実行完了は親担当が別途確認する。

## 監査方法と範囲

前回監査記録（`docs/progress/2026-09-08-914-independent-audit-initial.md`）のF1〜F3と固定AC/INVARを起点とし、修正担当の成功結論に依存せず差分を読んだ。CodeGraphの`useCommunityNodePolicyDialog runCommunityNodeOperation`探索を実施し、JSXとcallback配線をrgで補完した。独立に対象テストを実行した。

変更した製品surfaceは新しい規約controller、Settings/Dome caller、共有fetch/Node操作action、callback型・配線、共通Dialogのbusy時閉鎖表示、およびretry中の翻訳key。画像・証跡文書はコードsinkを増やさない。Rust/IPC payload/backend guardの変更なし。前回適合かつ未変更のsurfaceは再利用し、全監査を繰り返していない。

## 前回findingの確認

| finding | 修正と具体的evidence | 判定 |
| --- | --- | --- |
| F1: 古い規約応答・表示言語不一致 | `useCommunityNodePolicyDialog.ts:38-55`が対象・言語・attemptごとにgenerationを更新しcleanupで無効化。取得responseのviewをentryとして保持し、`:76-90`はそのviewのsnapshotとentry.languageを受諾へ渡す。言語が変わるrenderではcurrentEntryがnullとなり、旧文書の受諾ができない。`profileTopicChannel.ts:738-762`の共有cache書込みはpending entry同一性を確認。SettingsDrawerはlanguageを両callbackへ転送、Domeはlocaleをcontrollerへ渡す。旧応答後の再開と表示中言語変更のdeferred test、共有cacheの逆順testに対応 | 解消 |
| F2: 設定/Dome受諾失敗が背面へ隠れる | 新controllerの`:98-100`は受諾errorを保持し`:129-130`からDialogへ渡す。Dialogは既存のrole=alertとRetryをmodal内に表示する。Settings/Dome両方の実component testでmodal内alertを確認、Domeでは失敗時delegateHostingが0回。Retryはattempt更新で公開文書を再取得し、保存済みlocal consentの削除を追加しない | 解消 |
| F3: 遅延受諾statusによるevent巻戻し | `profileTopicChannel.ts:157-184`のrunCommunityNodeOperationはNode別baselineを保存、RPC後に現在authorとmembershipを確認。statusが変わっていた場合はmutation後のread-only getCommunityNodeStatusesを取得し、読取中にも変化した場合は現在store状態を維持する。status/configを一つのpatchで確定する。late retrying response対ready eventの元失敗条件をaction testで検証 | 解消 |

## delta入口 → helper → sink・逆引き

1. 設定drawer → CommunityNodePanel → useCommunityNodePolicyDialog → fetch/accept/withdraw callback。fetchは取得時language、acceptは表示snapshot/languageを渡す。Nodeはsavedかつ現在nodes在籍で限定。modal取消・再開・言語変更・二重操作はgeneration/acceptingで管理する。
2. DesktopShellPrimaryWorkspace → MetaverseRoomPanel → DomeHostingPanel → 同じcontroller。propsの中継でlanguageやresponseを破棄しない。受諾失敗では既存委譲sinkへ進まず、成功後だけ既存delegateToCommunityNodeへ進む。既存のruntime側同意guardは不変。
3. shared policy fetchのproduction callerをSettings/Domeまで追跡した。`void` view fallbackは既存fixture互換の分岐であり、productionのhandleFetchCommunityNodeConsentsはresponseから作ったviewを返すため、共有cacheの後続上書きを表示snapshotに使用しない。
4. runCommunityNodeOperationの全6 callerはauthenticate、invite code設定、token clear、metadata refresh、accept、withdraw。すべて同じ在籍・author・競合確認を通る。新しく増えるI/Oは競合時のread-only status IPCであり、authやNode HTTPを始める分岐ではない。
5. 新案内とExplore/topicのuseCommunityNodeConsentFlowは変更なしだが、共通accept actionを経由するためF3修正の影響先として再確認。既存の表示snapshot固定・二重受諾防止testとrecovery testを再実行した。
6. CommunityNodeConsentDialogは4製品callerすべてを確認。busy時は閉じるボタンを無効化しclose affordanceを隠す。各controllerのcloseもaccepting中は無効なので、mutation開始後だけ表示上の操作可能性と実処理を一致させる。policy読込中はbusyではないため閉じる/遅延応答破棄を維持。

## 固定AC / INVARとの対応

| 条件 | delta確認結果 |
| --- | --- |
| AC-1 / AC-6 | F3の逆順を修正し、status/configの同時patchから既存storeの選択導出へ進む。poll/manifestの既存世代保護は不変 |
| AC-2 | retry中表示を実在するshell翻訳keyへ修正。原因分類、manual停止、成功空の条件は不変 |
| AC-3 / AC-4 | shell初回案内、index 0、全Node状態判定、session再表示抑止、startup gateにdeltaなし |
| AC-5 | Settings/Domeに世代・言語・表示snapshot固定とmodal error回復が適用。新案内/Exploreの元controllerも局所testで維持 |
| INVAR-1 | 新しいNode送信や同意mutation入口なし。競合後のstatus読取はread-only。既存runtime preflightやtoken/relay/seed guardは不変 |
| INVAR-2 | Node別結果反映、現在membershipを確認。Node configの順序/空一覧/auto/manualの既存導出は不変 |
| INVAR-3 | app/年齢/restore gate、Direct P2P、private query境界は不変 |
| INVAR-4 | 全同意UI callerの表示文書・言語対応を確認。取消時の同意呼出しなし、保存済み同意のrollback削除なし |
| INVAR-5 | controllerのfocus復帰と世代無効化、Dialog閉鎖、Column/query contextの未変更を確認。視覚実機の最終gateは親担当 |

TR-4/5/10の前回不適合を今回のaction/controller/component testと制御フローに対応付けた。TR-1〜3/6〜9は未変更部分の前回証拠を維持する。新要件は追加していない。

## 独立実行した検証

`apps/desktop`で次を実行:

```text
npx pnpm@10.16.1 exec vitest run src/shell/useDesktopShellActions.test.tsx src/shell/actions/useCommunityNodePolicyDialog.test.tsx src/components/settings/CommunityNodePanel.consent.test.tsx src/components/extended/metaverse/DomeHostingPanel.test.tsx src/components/settings/SettingsPanels.test.tsx src/shell/actions/useCommunityNodeConsentFlow.test.tsx src/shell/actions/useCommunityNodeRecovery.test.tsx
```

- **7 files / 53 tests PASS**、7.57秒。
- `git diff --check bf9df4a8 e0200108`: PASS。
- Rust全suite、CI、browser/Storybook、OS実機は重複実行していない。本監査でそれらの完了を代替しない。

## Non-blockerと停止条件

一般的な全アプリmodal coordinatorや別の同意store導入、未変更機能の拡張は求めない。固定AC/INVARに対応しない新しい製品条件は追加していない。

F1〜F3の解消を確認し、入口群の未分類0・不適合0・blocker0となったためdelta監査を終了する。同じheadの全監査反復は不要。以降に対象コードが変わる場合だけ、そのdeltaと影響callerを再監査する。
