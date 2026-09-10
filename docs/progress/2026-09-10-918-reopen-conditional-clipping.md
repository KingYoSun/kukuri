# Issue #918 再開 — 幅変更後のExploreの見切れ

## 現在判定

- 実装・ローカル検証・独立監査の記録。独立監査はPASS。CI・merge・Closeの最新判定は [PR #970](https://github.com/KingYoSun/kukuri/pull/970) とIssueのCurrent statusを参照する。2026-09-10に計画実行、commit・PR・必須CI成功後のmergeまで承認済み。
- リスク区分B、Scope revision `918-r2`。基準commit `5db2aeb0853329ac12558a5fbc503553fd6618d0`。PR識別子 `codex/issue-918-reopen-conditional-clipping`。
- [Issue #918](https://github.com/KingYoSun/kukuri/issues/918)の再現／非再現が混在する報告を受けた再開。[前回作業記録](2026-09-09-918-explore-overflow-and-content-density.md)は`918-r1`当時の実装・検証証拠であり、今回の完了根拠ではない。
- 正本は[DESIGN](../../DESIGN.md)、[ADR 0031](../adr/0031-variable-span-column-workspace.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[Issue lifecycle](../runbooks/issue-lifecycle.md)。現在のfreshは5カラム。3本の同時表示やCanvas横scrollの廃止は今回の要件にしない。

## 再現・原因

1. 保存3カラム（Timeline / Notifications / Explore）で1600×800の画面を開く。
2. Control CenterからExploreを選択する。browser試験ではqueryへ文字を入力する。
3. 他の操作を行わず、viewportを1280×800へ縮める。

修正前はExploreが `x=952, width=440, right=1392` に残り、Canvasの右端1264を128px超える。Workspace内部の幅は正しくても、発見button・説明文・Node案内がCanvasで切れる。最初から1280pxでExploreへ移動した場合は既存のactive変更effectがscrollするため、同じ症状にならない。Ubuntu 24のv0.2.1-preview.1配布AppImageでも同じ1600→1280の見切れを確認した。

続けて759px以下へ縮めると、CSS scroll snapによる位置変更を既存120msのscroll-settleが利用者操作と扱い、別カラムがactiveになる場合がある。queryのDOM focusはExploreに残るため、選択・表示・focusが分離する。

原因のownerは `apps/desktop/src/components/shell/ColumnCanvas.tsx`。active IDの変更時には表示位置を合わせるが、Canvas幅が変わってactive IDが変わらない場合には追従しなかった。これは固定AC-1/4・INVAR-2/3、INV-7、R-TR-4のExisting-gapであり、前回の文字密度変更によるRegressionと断定しない。

前回 `community-index-layout-fixture.ts` はinit scriptで毎回同じlayoutとExplore activeを投入し、reloadでも再適用していた。既存helperのWorkspace内部containmentと、操作時の自動scrollだけでは外側Canvasの欠落を検出できない。今回のtestは初回だけページからlayoutを設定し、reload前の保存値を再投入しない。

## 固定AC / INVAR

全文は[918-r1の固定条件](2026-09-09-918-explore-overflow-and-content-density.md#固定ac--invar)を継承し、以下の検証対応を補う。条件の削除・免除はない。

| ID | 条件・実装と検証の対応 |
| --- | --- |
| AC-1 | 1280×800・日本語・3カラムのタブ・query・実行buttonがカラム内に収まり操作できる。新規`column-resize-context.spec.ts`が通常起動→移動→resize後のCanvas内表示を操作前に検査し、既存`community-index-layout.spec.ts`が3操作を検証 |
| AC-2 | Node案内・長URL・回復actionへの全文到達と対象の一致。既存layout、consentGate、availability／onboardingのtest・Storyを維持 |
| AC-3 | 本文14px相当、投稿15px、補助12px、header16pxとportal基準を維持。CSS／token／fontは変更せず、既存computed styleとvisualで確認 |
| AC-4 | 3言語・2theme・代表幅・200%で内部clip・overlap・document overflowなし。新規resize matrixと既存localization／reflow、製品WebViewの実機確認で対応 |
| INVAR-1 | 検索先・scope・同意・認証・retry・pending・callback不変。resize／scrollだけでqueryや同意保存が発生しない。新規testのcall記録と既存境界testで確認 |
| INVAR-2 | query／draft／Column配列・span・設定復帰文脈を維持。新規testは入力focus、active、保存layoutの同一性を検査。手動でactiveから離れた閲覧位置をresizeで引き戻さない |
| INVAR-3 | accessible name、順序、target、色、root font、portal基準不変。操作前のbounds、実入力、既存a11y・focus／mobile検査で確認 |

## 固定surface inventory

基準は上記commit。INV-1〜6の全memberは前回固定inventoryと適用先一覧を継承し、INV-7/8を明示する。コードはCodeGraph、CSSはselectorから確認。実差分はCanvasのresize／scroll観測のみ。認証・検索・保存APIの入口／sink追加削除は0。独立監査でも8 group適合、不適合0・未分類0を確認した。

| ID | 入口・trigger／member | shared helper | 読み書き・副作用 | guard／invariant | transition／検査 |
| --- | --- | --- | --- | --- | --- |
| INV-1 | CommunityIndexWorkspaceの3タブ・query・submit、PrimaryWorkspace | 既存query実行・結果更新 | 明示query・local state | INVAR-1/2 | R-TR-1/3、query／layout test |
| INV-2 | Workspaceの未同意・ノードなし・選択先不可・retry・error | 規約／設定／回復callback | 既存明示I/O | INVAR-1/3 | R-TR-3、consentGate／layout |
| INV-3 | AvailabilityNotice、CommunityNodeOnboarding、既存Story | availability・retry timer | 既存timerと回復のみ | INVAR-1/2/3 | R-TR-3、既存unit／Story |
| INV-4 | 全10 Column kind、ColumnSurface／PrimaryWorkspace／AuxiliaryPanels、post-body利用先 | 本文container・typography | 描画・既存取得 | AC-3/4、INVAR-2/3 | R-TR-4/5、typography／visual |
| INV-5 | Control Center、FilterableTopicNavList、SavedWorkspaceLayouts | focus／設定／layout action | 明示focus・既存保存 | INVAR-1/2/3 | R-TR-1/2/5、shell／layout |
| INV-6 | header、設定／規約portal、fixed dock／indicator | CSS cascade・primitive | 描画 | AC-3、INVAR-3 | R-TR-4/5、bounds・focus・visual |
| INV-7 | DesktopShellColumnWorkspace→ColumnCanvas→ColumnSurface、resize／activate／scroll／observer。Story・testもcaller | scrollIntoView・可視集合・paging geometry | DOM scrollと既存active更新。新resize callback自身はroute／store／APIを書かない | INVAR-1/2/3、可視だったactiveだけを補正 | R-TR-1/4、新規browserと既存Canvas／mobile |
| INV-8 | fresh・保存layoutの通常起動、名前付きlayout適用、reload／再起動 | workspacePersistence・workspace slice・routing | 既存読込／明示保存 | INVAR-2、schema不変 | R-TR-2、新規再投入なしreload・既存layout・製品再起動 |

呼出先は `rg -l 'ColumnCanvas' apps/desktop/src -g '*.tsx'` で `DesktopShellColumnWorkspace.tsx`、`ColumnCanvas.stories.tsx`、`ColumnCanvas.test.tsx`、`ColumnCanvas.swipe.test.tsx` と定義自身を確認。本文のmemberは `rg -l 'shell-column-body|post-body' apps/desktop/src -g '*.tsx'` で前回一覧と突合した。

R-TR-3の非同期境界は `CommunityIndexWorkspace.test.tsx` の `a pending response is discarded when its node or scope is no longer active`、`responses that complete in reverse order keep only the current request context`、`a selected node that is no longer eligible does not receive queries until it is eligible again` と既存consentGate試験を含む。長文と回復表示は19状態Storyの全言語／themeと長URL browserで確認する。

## 状態遷移

| ID | 事前状態→event sequence | 期待状態 | 許可I/O | 禁止副作用／検証 |
| --- | --- | --- | --- | --- |
| R-TR-1 | Timeline activeの3カラム起動→Control Center→Explore→3操作 | 選択対象に到達し、内部clipなし | 明示query | 自動query・重複submitなし。新規起動／移動＋既存query test |
| R-TR-2 | fresh 5、保存3、順序／span変更済み→保存→reload／再起動 | 保存構成・span・targetを復元 | 既存読込／明示保存 | 再seed・schema変更なし。新規reload＋既存workspace／実機 |
| R-TR-3 | Node状態遅延→長文案内→error／retry待ち→回復 | 全文・理由・対象・期限が読める | 既存取得・timer・回復 | 自動同意・期限内retryなし。既存unit／Story／browser |
| R-TR-4 | query／draft保持→1600→1280→900→760→759→390→desktop、native zoom往復 | 可視だった選択対象・focus・入力を保持 | 既存可視集合反映 | resize由来の誤activeなし。手動scrollで離れたactiveへ引き戻さない。新規matrix／実機 |
| R-TR-5 | Control Center→設定→戻る、locale／theme変更 | 文脈・focus・文字階層保持 | 既存設定のみ | portal縮小なし。既存localization／layout／visual |

## 修正内容と境界

Canvasの幅変更をwindow resizeとResizeObserverで検出し、直前に全幅が表示されていたactive Columnだけを `scrollIntoView` で即時に合わせる。幅の変わらないscrollでは可視状態の記録だけを更新し、利用者のscroll位置を変更しない。幅変更に先行するscroll snapイベントでは古い幅での可視判定を保持する。

mobile補正は既存のprogrammatic scroll targetを設定し、保留中のsettleを取消す。これによりresizeのscrollを利用者のpage移動と誤認しない。listenerとobserverはeffect cleanupで解除する。CSS、共有primitive、query処理、layout保存形式は変更しない。

## 検証記録

| 項目 | 現在の結果 |
| --- | --- |
| 修正前の新規browser | desktop縮小とmobile境界の2件がbounds assertionで失敗、手動scroll保持の1件は成功 |
| 修正後の同3件 | 3件成功 |
| 新規3言語×2theme＋既存layout／localization | 48件成功（新規13件を含む） |
| TypeScript | `tsc --noEmit`成功 |
| Linux `cargo xtask check` | 成功。Rust／Tauri compile、frontend lint／typecheck |
| Linux `desktop-ui-check`（CI=1） | 初回は既存`DesktopShellPage.profile.test.tsx`の1件が10秒timeout、1313件成功。timeout値・test内容を変更せず、下記の全suiteと後続の構成stepを成功させた |
| Linux `cargo xtask test` | 成功。Rust合計948件、既存ignored4件、doctest成功。frontend165ファイル・1314件成功（初回timeoutしたprofile試験も含む） |
| Linux `desktop-storybook` | 成功。Canvas3 StoryとWorkspace／Availability19 Storyの計22×3言語×2theme＝132条件でaddon-a11y違反0 |
| Linux `desktop-browser-test`（CI=1） | 165件成功。新規13件、既存Canvas／mobile／scope／query／layout／localizationを含む |
| Linux `desktop-visual-test`（CI=1） | 20件のLinux／Chromium比較成功。baseline更新0。最初の起動はbrowser testとport4176が競合しtest開始前に失敗したため、browser終了後に単独実行して成功 |
| `cargo xtask oversized-files`／`git diff --check` | 成功。baseline上限の変更0 |
| 独立監査 | `3f3327dba4f343d4a739ba849f4399d3a0b93ce9`でPASS。新規13件を独立再実行して成功、blocker0。[監査記録](2026-09-10-918-resize-independent-audit.md) |
| 必須CI・merge後照合 | PR #970の最終headとmerge commitに対する結果をPR／IssueのCurrent statusへ記録する |

## 実機の区別と再実行条件

- `ssh local2`、`~/kukuri`、Ubuntu 24.04.5、GNOME Wayland sessionのXWayland、WebKitGTK 2.52.6。リモートデスクトップで実際の製品画面を確認。
- 配布版 `kukuri_0.2.1_amd64.AppImage`、tag `v0.2.1-preview.1`、SHA256 `9f4990ae0913a9682a23d28779ea0791f7e53c2c8eab1f64ec7c33e52995df3b`。tag commit `f2cdb5cacf02218b34291a35754b55a15c2f564e` は前回mergeを祖先に含む。
- UI試験用の合成済みapp同意fixtureと新規試験accountを使用。ユーザーの年齢申告／同意記録ではない。既存profile・秘密鍵・Node同意・DMをコピーしない。Nodeは未同意のままで、規約の自動受諾は行わない。
- runtimeは `KUKURI_APP_DATA_DIR`、WebView storageは `XDG_DATA_HOME` と `XDG_CACHE_HOME` を専用directoryへ分離。`KUKURI_DISABLE_KEYRING=1`、`KUKURI_DISCOVERY_MODE=static_peer`。保存3カラムのlocalStorage fixtureはプロセス停止中に一度だけ設定し、以後の再起動では再投入しない。
- 初期化した専用profileでNode説明を閉じ、Exploreを選択してから、アプリ内容領域を1600×800→1280×800へ変更する。`xwininfo`でアプリ領域を確認し、画像はGdkでそのウィンドウだけを採取。RDP解像度や装飾込み外寸とは区別する。
- 修正候補は同じrepositoryの製品Tauriを `pnpm tauri build --debug --no-bundle` で生成。通常capabilityのLinux実機で1600→1280縮小後のExploreが `x=800, width=440, right=1240` となり、Canvas内へ収まることを確認。WorkspaceのscrollWidth372px、本文14px／header16pxは不変。
- 両OSのnative zoom試験に限り、非追跡configで `core:webview:allow-set-webview-zoom` を許可した製品buildを用いた。通常の製品capabilityでは当該IPCが拒否される。productionのcapability／CSPは変更せず、native zoom 2→1を確認するための検証用設定である。
- Linuxはnative zoom 2で実効640×400・devicePixelRatio 2、active Explore・3カラムを保持。最後のtabから実キーTabで規約確認へ移り、focus矩形とhit testが一致。Windows 11 build26200、WebView2では1280×840→実効640×420、初期DPR1.5→3を確認。実pointerの縦scroll、Node設定への回復、Escapeで復帰、実キーTabのfocus到達を確認した。
- 両OSでzoomを1へ戻した後、layoutを再投入せず製品processを終了・再起動し、Explore active、3カラム、Canvas内のboundsを確認。LinuxはSIGTERM、Windowsは試験processをterminateして再起動した。Windowsで正常Quitの永続化を新たに検証したという扱いにはしない。
- [同条件画像とUI採用記録](../ui-reviews/2026-09-10-918-resize-context.md)を参照。Windowsのzoom画像はCDP screenshotではnative DPIによるcropが起きたため、実際に観測したComputer Useのwindow captureを保存した。cropした画像をUI不具合や成功証拠と混同しない。

## 終了条件と確認範囲

今回見つけたresize条件は修正前後の検査で解消を確認したが、報告時の操作履歴を断定しない。nativeはNode未同意の表示・回復と実機のresize／zoom／保存復元を検証し、検索成功／pending／retryと送信先の契約は既存のcomponent／browser試験で確認した。実Nodeへの検索成功を今回再検証したという扱いにはしない。

新規observerの影響はCanvas内に閉じ、loopback以外の新しい試験serverや実データ投稿は作成していない。色・label・target・CSSが不変のため、screen reader／High Contrast全体とGPU／巨大一覧の性能再評価は非該当。全条件の証跡と独立監査・CIを揃えるまで再Closeしない。
