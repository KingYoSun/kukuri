# #943 独立監査

## 対象・判定

- 対象 commit: `fc532883232233986535dd4f21a4e456b03975fb`
- 差分基準: `d2610af4467f6a91fefa208952abc5728496dc29`
- Scope revision: `2026-09-08-column-activation-ci-v1`
- リスク区分: B。共有選択状態について、承認済みプランの独立監査工程として実施。
- 固定条件の取得元: `gh issue view 943 --json title,body,url` で取得した [Issue #943](https://github.com/KingYoSun/kukuri/issues/943)。Issue 本文の旧 CI 基準 `8d03a7ac8f5c22b98263e73f92bfc804dcd63965` と今回の差分基準を区別した。
- inventory: **3 group / 適合3 / 不適合0 / 未分類0**。共有 helper の実登録1、直接呼出し44か所を別途照合。
- blocker: **0**。
- 独立監査判定: **PASS**。これは固定コード・固定条件の監査判定であり、必須 CI の成功や merge 完了の代替ではない。

実装時の結論を前提にせず、Issue の条件、固定差分、source、既存と追加の test を先に評価し、その後に保存済み before/after と実行ログを照合した。製品コード、既存サーバー、native host、GitHub は変更していない。監査用の隔離ファイルと結果は `test-results/issue943-independent-audit/` に置いた。

読んだ規則は root `AGENTS.md` / `AGENTS.local.md`、`docs/README.md`、dev / issue-lifecycle runbook、`PLANS.md`、`REFACTORING.md`、`DESIGN.md`、ADR 0014 と ADR 0031 の active / visible / audio focus / URL / lifecycle 契約。今回の変更は不具合修正であり、UI 再設計や protocol / storage の変更ではない。

## 登録・入口・副作用の再構築

`.codegraph/` が存在するため `codegraph explore "useSyncRoute"`、`codegraph node` で hook、route同期、Canvas、runtime/context、hash parser を探索した。CodeGraph の nested callback の caller 集計だけに依存せず、次の検索で全 source の直接呼出しと pending の読書きを補完した。

```powershell
rg -n 'syncRoute\(|pendingRouteUrlRef|useSyncRoute\(|HashRouter' apps/desktop/src --glob '!*.test.*'
```

登録は `App.tsx:178` の `HashRouter` → `DesktopShellPage` → `useDesktopShellRouting.ts:196` の `useSyncRoute` 1か所。`useSyncRoute` の副作用は `pendingRouteUrlRef.current` と `navigate(nextUrl, { replace })`。URL組立てに使う store は呼出し時点で取得し、今回の変更で API、永続化、network 呼出しを追加していない。

直接呼出し44か所の再生成結果は次の通り。行番号は固定対象 commit のもの。各行の push/replace、nullable override、scope、呼出し前後の処理を確認し、[保存済み caller 一覧](2026-09-09-943-column-activation-evidence/sync-route-callers.txt)とも一致した。

| `apps/desktop/src/shell/` からの path | 件数・全呼出し行 | 所有する入口・契約 |
| --- | --- | --- |
| `DesktopShellPage.tsx` | 6: 336, 366, 395, 423, 475, 678 | content参照、Settings、Column activation。選択先の canonical URL を push |
| `useDesktopShellRouting.ts` | 16: 233, 296, 314, 393, 433, 465, 531, 573, 640, 710, 749, 766, 797, 826, 837, 850 | Settings、DM、thread、author、primary section、timeline view、pane close、profile mode。通常 push と error/close 時 replace |
| `routing/useRouteSynchronization.ts` | 2: 320, 915 | legacy DM と route 正規化の rAF。replace を維持 |
| `actions/composeInteractions.ts` | 1: 116 | 投稿時の thread解除と timeline URL |
| `actions/liveGame.ts` | 2: 120, 205 | Live/Game作成後の section replace |
| `actions/messageReactionSocial.ts` | 2: 134, 181 | 通知経由の author / timeline / scope遷移 |
| `actions/profileTopicChannel.ts` | 12: 263, 302, 337, 353, 370, 394, 459, 500, 566, 618, 638, 657 | topic/channel選択・作成・退出、profile保存、discovery/CN設定後の現状態同期 |
| `useDesktopShellActions.ts` | 2: 790, 849 | topic切替と local draft復帰 |
| `page/DesktopShellSettingsDrawer.tsx` | 1: 333 | 開いた Settings の section replace |

pending の生成 owner は `DesktopShellPage.tsx:190`。書込みは変更 hook、`useRouteSynchronization.ts:192,198,200` の消費/破棄、`useDesktopShellRouting.ts:672` の通知から直前URLへの復帰。最後の経路は helper を通らない直接 navigate だが、同じ pending consumer を使うため逆引きに含めた。pending は永続化されず、restart は初期 route / workspace 投影へ戻る。

| ID | 再生成した入口 → helper → sink | guard / 適合根拠 | 判定 |
| --- | --- | --- | --- |
| INV-1 | Canvas header pointerdown / focus、mobile settle / swipe / indicator、Control Center・workspace close → `activate` → `activateWorkspaceColumn` → `syncRoute` | workspace の実在確認、interactive/gesture/preserve-activation判定を維持。store activation の後、`loadTopics` の await より前に route を同期。新規 rapid / pointer-history test と既存 Canvas / routing tests | 適合 |
| INV-2 | 上表44呼出し、route observation、遅延focus / 正規化rAF、通知復帰 → hash / pending / workspace投影 | 実 hash が未反映の render に優先する。consumer の履歴追越し判定と投影条件を維持。focus rAF は active変更時 cleanup、unmount時も予約をcancel。旧pendingと次の利用者入力を unit で確認 | 適合 |
| INV-3 | IntersectionObserver → visible集合 → `projectColumnRuntime` → provider → Surface / MetaverseRoomView / Scene | 交差率 `>0.01`、observer再購読とcleanup、`immersive && !visible && !audioFocused`、Sceneの `frameloop='never'` と入力抑止を維持。providerはcolumn IDでkey付けされ、停止のためにsessionを破棄しない | 適合 |

公開入口、callback登録、network / 永続 sink の追加・削除は0。inventory は変更前後とも3 group、44 helper呼出しで、件数一致だけでなく各副作用の所有を確認した。

## 状態遷移・例外の評価

| 条件・transition | 確認した結果と証拠 |
| --- | --- |
| TR-1: fresh room → Metaverse | 元 offscreen test が room作成・canvas・activeを確認。保存済み browser log の同test成功、runtime source とunitを照合 |
| TR-2: Metaverse → Live | runtimeはvisibleからsuspendedを導出。元test、session継続test、native JSONLでMetaverseのcanvas保持・停止を確認 |
| TR-3: Live → Metaverse、router render前 | 旧renderがMetaverseでも実hashはLiveとなる。旧実装は最後のnavigateを省略し、変更後はMetaverseへpushしてpendingを設定。独自の基準コード実行で失敗、新コードで成功 |
| TR-4: 次の実pointer / history / focus | browser testはLive → Metaverse → back → forwardを実行し、LiveのLeave可能・viewer 1、同一Live card / stage / canvasの接続維持を検査。既存Canvas testsはkeyboard、mobile settle、wheelによる引継ぎ、indicator、reduced motionを保護 |
| 同じpending先の再要求 | 実hash一致ならnavigateを増やさず、render未観測ならpendingを保持。render一致で解除する新規unitが成功 |
| historyがpendingを追い越す | 新規unitは古いpending解除後のnotifications replaceを許可。既存consumer testはpending pushを破棄して履歴URLへworkspaceを再投影し、thread選択も解除する |
| 未観測pending / 観測済みroute | 既存consumer testは未観測中のnormalize、navigate、loadTopicsが0であることを確認。route変更後はpending解除と通常正規化へ戻る |
| hashなし / 不正route / missing state | hashなしは従来のrendered locationへfallback。parserは文字列の分離だけで例外を投げない。topic無しの早期returnは変更なし。不正path・未追跡topic・未加入channel・developer mode offの既存normalizeを変更していない |
| API失敗 / 遅着 / cancel | shared hook自身にAPI、retry、catchはない。callerのerror時replaceとroute consumerの`loadTopics(...).catch`を変更していない。Canvas focusのcleanupとroute rAFのunmount cancelも維持。新しいretryや要求の恒久ロックはない |

401 / account / nodeごとのglobal apply / rollback はこのローカルURL比較差分が所有する新しい状態ではない。既存API guardの変更として扱わない。あらゆる非同期処理へ世代管理を導入する一般化や、未知の将来routeは完了条件へ追加していない。

## AC / INVAR と独立証拠

| 固定条件 | 証拠・評価 |
| --- | --- |
| AC-1 | 同じ新規unitを基準commitのhookで独自実行し、A→B→Aが期待 `#/game` / 実際 `#/live` で失敗した。browserの[before JSON](2026-09-09-943-column-activation-evidence/before-sequence.json)も最後のMetaverse active時にhashはLive。保存画像は最終Live Activeを示す。Windows/Chromium 900×900、en-US/UTC、fresh mock、microtaskでstore更新を先行させるsequenceがtest本文に固定されている |
| AC-2 | 製品差分は `useSyncRoute.ts:98–115` の比較・pending更新だけ。対象headのunitは成功し、[after JSON](2026-09-09-943-column-activation-evidence/after-sequence.json)では最後のhashとactiveがMetaverseで一致。browser testはactive/hash/runtime/scene/canvasを同じDOM観測で確認し、単発のruntime復帰だけを成功条件としていない |
| AC-3 | 元offscreen assertionを保持し、Live作成・Join後のpointer/history/session継続testを追加。既存route/history/focus契約との照合、独自unit実行、保存済みbrowser log、native観測と残存制約の明記を確認 |
| INVAR-1 | URL builder、nullable override、scope、push/replace引数は不変。54 unit成功。native JSONLはLive選択でMetaverse停止、Metaverse再選択で復帰、canvas 1の保持を示す。native keyboard画像ではMetaverse drag gripにfocus ring / tooltipがあり、Metaverse Activeを保持 |
| INVAR-2 | 差分にsleep追加、既存assertion削除/緩和、skip、retry設定変更なし。新規testのmicrotaskは再現順序を固定し、cancel済みcallbackを強制実行しない。元browser test本文は無変更 |

before/after画像とnative keyboard画像を実際に開き、JSONと表示上のActive/focus状態を照合した。保存資料の確認を監査者自身のnative再操作とは呼ばない。過去CIにはtraceがなく、当時の全イベント列が今回と同一だったことまでは証明していない。現行登録経路で同じ利用者症状を再現する具体的原因と、その最小修正を監査した。

## 実行した validation

Windows、Node `v22.14.0`、pnpm `10.16.1`。独自の対象head実行:

```powershell
# apps/desktop で実行
npx pnpm@10.16.1 exec vitest run --project=unit src/shell/routing/useSyncRoute.test.tsx src/shell/routing/useRouteSynchronization.test.tsx src/shell/useDesktopShellRouting.test.tsx src/components/shell/ColumnCanvas.test.tsx src/shell/columnRuntime.test.ts src/components/shell/ColumnSurface.test.tsx --reporter=json --outputFile=../../test-results/issue943-independent-audit/vitest.json
```

結果は **5ファイル54件 PASS**（syncRoute 12、route synchronization 13、routing hook 13、Canvas 14、runtime 2）。指定した `ColumnSurface.test.tsx` は存在せず、実行件数に含めていない。Surfaceのruntime属性はsourceとbrowser testで確認した。

独自before検証はroot実装を編集せず、`git show d2610af4:apps/desktop/src/shell/routing/useSyncRoute.ts` の出力を監査用ディレクトリへ保存し、元Vite設定を継承した一時 `.mts` configで `@/shell/routing/useSyncRoute` だけをそのファイルへaliasした。Reactは同じ `apps/desktop/node_modules/react/index.js` に解決させた。

```powershell
# repo root で実行
npx pnpm@10.16.1 --dir apps/desktop exec vitest run --config ../../test-results/issue943-independent-audit/vite.before.config.mts --project=unit src/shell/routing/useSyncRoute.test.tsx --reporter=json --outputFile=../../test-results/issue943-independent-audit/vitest-before.json
```

結果は **12件中10 PASS / 2 FAIL**。失敗は以下で、対象headでは両方PASSだった。

- `keeps the latest Metaverse selection when Live navigation has not rendered yet`: 140行、期待game / 実際live。
- `does not push the same pending target twice and keeps it pending until observed`: 159行、navigate期待1回 / 実際2回。

一時configの初回はESM/CJS解決、次は隔離ファイルからのReact解決で起動失敗した。`.mts` とReact aliasで解決後に上記12件を実行した。起動失敗は製品testのbefore failureに数えていない。

`git diff --check <基準> <対象>` は成功。監査時の `apps/desktop` tree が対象headと一致することも `git diff --quiet <対象> -- apps/desktop` で確認した。

保存済み `.codex/plans/issue-943-desktop-ui-check.log` を独立に読み、162 files / 1305 unit、browser 139、Windows visual smoke 20の成功と、offscreen / rapid / session / hash-routing / column-scope各testの成功行を確認した。これは実装者の既存実行ログであり、監査者の再実行ではない。rapidの同時点assertion強化はこのfull logより後なので、その最終testを含む固定head全体はCIの確認対象とする。

今回の監査でbrowser / nativeサーバーは起動・停止・再利用していない。必須のLinux visual比較と最終head CIは並行する実装担当の工程で確認する。Windows smokeをLinux画像比較成功とは扱わず、監査PASSだけでMerge readyとしない。

## 残存事項・停止条件

- Existing-gap / Regression の concrete blockerは0。固定AC全5件が証拠に対応し、3 inventory groupとTR-1～4、pending/履歴/fallbackが分類済みなので、有限監査を終了する。
- 過去CIの全発火列同一性、screen reader実発話、Linux WebKitGTK実機、実network session、GPU frame rate、既知native fullscreen問題の解消は未確認。これらの解消を本差分の成果として主張しない。既知fullscreenの同時修正は固定Non-goal。
- 一般的な非同期世代管理や追加の全route競合網羅は、今回の具体的回帰証拠なしにblockerへ昇格させない（Optional-hardening）。
- 対象製品surfaceが変わらなければこのPASSを利用できる。変更された場合はdeltaと影響先だけを再監査する。必須CI成功、merge後の対象一致、Issueの現在判定更新は別途必要。
