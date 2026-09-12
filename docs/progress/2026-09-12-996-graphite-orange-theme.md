# Issue #996 Graphite / Orange 配色変更

- 対象: [#996](https://github.com/KingYoSun/kukuri/issues/996) / [PR #997](https://github.com/KingYoSun/kukuri/pull/997)
- Scope revision: `2026-09-12-theme-a-v1`
- 基準commit: `3776c24080a142e80bfc7a3140e2e34473b5dc0e`
- リスク区分: B（frontend表示責務）。親Issue集約・Reopen・shared guardの変更はなく、独立監査は必須対象外。
- 実装・ローカルUI gate・実機確認: 完了。最終CI／mergeの現在判定はPR #997とIssue #996を参照する。

## 変更内容・境界

採用済みA案を`tokens.css`のdark／lightへ適用し、`DESIGN.md`の方針と全token契約表、Foundationsを同期した。カラム本文・基本panelはdark `#212121`／light `#FFFFFF`、header／footerは`#262626`／`#ECECEA`、post cardの起点は`#292929`／`#F7F7F5`。主ボタンは既存オレンジを維持し、文字を濃色inkへ合わせる。背景・文字・scrollbar・shadowは中立色系列に揃え、selected／active／focusは局所的な橙系とした。

新しい弱い装飾境界をinput、select、textareaの唯一の識別に使わないよう、共通primitiveとsemantic CSSの該当箇所を既存`--border-subtle-strong`へ変更した。状態のwarning／error／接続色は保持・必要なcontrast調整を行い、media／3D本体の色は変更しない。寸法、書体、layout、保存形式、同意・権限、Rust／IPC／networkは変更していない。

## inventoryと逆引き

CodeGraphで`readDesktopTheme`／`writeDesktopTheme`→`App`を確認し、CSSはindex対象外のため直接確認した。production入口は`styles/index.css`の9 local stylesheet（part1〜4を含む）で、import順は変更なし。`@theme inline`は同じsemantic tokenへのaliasを維持する。

変更色tokenの直接参照は[利用先一覧](assets/996/token-consumers.json)に記録する。`var()`に加え、`bg-background`／`text-foreground`／`bg-card`／`text-muted-foreground`／`border-border`／`bg-input`／`bg-primary`／`text-primary-foreground`／`text-accent`／`text-accent-foreground`／`text-destructive`等のTailwind aliasは`@theme inline`→同じroot tokenを使う。新規local import・入口・副作用・shared helperは0。以下の全memberを配色適用先として分類した。

| ID | 入口／member | 適用経路・副作用 | guard／維持条件 | TR／証拠 |
| --- | --- | --- | --- | --- |
| INV-1 | 初回起動、保存済み起動、表示設定切替 | `lib/theme.ts`→`App`のroot `data-theme`。既存localStorageのみ | startup readyでの保存、dark既定、不正値fallback。処理変更なし | TR-1〜3、shellChrome／shell smoke／theme-palette、native記録 |
| INV-2 | Timeline、Bookmarks、Thread、Profile、Notifications、Messages／Conversation、Explore、Stream／Metaverse chrome、header／footer、dock、controls | production CSS／共通primitive→root token。描画のみ | scope、active、pin、幅、順序、draft、scroll、戻る文脈。media／3D本体は対象外 | TR-2／4、既存visual／localization／browser、比較画像 |
| INV-3 | Control Center、Settings、Composer、Dialog、Popover、DropdownMenu、Tooltip、初回同意、起動エラー | base／part*／primitiveとbody portal→root token。描画のみ | 同意・操作権限gate、意味色、label、keyboardを維持 | TR-2／4、portal実効色・draft・focus、意味pair |
| INV-4 | Foundations、component stories、Linux Chromium visual fixture | productionと共通のstyle入口 | theme・roleと全状態の意味を維持 | TR-4、Storybook、contrast、visual baseline |

inventoryは4group。分類外0、追加・削除0。token値の適合と描画は以下の証跡で別に判定し、分類済みであることだけを完了根拠にしない。

## AC／INVAR・状態遷移と証跡

| 条件 | 実装・証拠 |
| --- | --- |
| AC-1 | `tokens.css`、DESIGN token契約、Foundationsと比較画像。無彩色の主要面とオレンジ操作 |
| AC-2 | `theme-palette.spec.ts`のproduction computed background、[変更前](assets/996/before-computed.json)／[変更後](assets/996/after-computed.json)、[本文中央の画像採色](assets/996/column-pixels.json)。矩形(240,730)〜(280,770)の1600画素が`#212121`で一致 |
| AC-3 | `contrast.test.ts`の両theme semantic pair＋合成ring、`theme-palette.spec.ts`のportal／入力境界／focus。disabledは既存非操作stateを維持 |
| AC-4 | `design-contract.test.ts`で全runtime値の一致、Foundations、[採用記録](../ui-reviews/2026-09-12-issue-996-graphite-orange-theme.md)と旧recordの後継参照 |
| AC-5 | 下記before／after。同一fixtureと日本語、両theme、desktop／narrow。生成画像を実装証拠として使わない |
| AC-6 | 下記validation。TR-1〜4を既存・追加テストとnativeへ対応 |
| INVAR-1 | layout寸法・Column順序／pin／span・見出し、theme切替時draft、portal開閉後focusをbrowserで比較。色以外のproduction値・挙動は差分で無変更確認 |
| INVAR-2 | theme key・既定値・startup gate・書き込みcallerは無変更。初回dark／不正値dark／切替保存／reload／native再起動を確認 |
| INVAR-3 | label／icon／gate／handlerは無変更。light／dark両方の状態contrast・実keyboard／pointerを確認 |

- TR-1: 保存値なしはdark、無効値も既存dark fallback。`theme-palette.spec.ts`と既存shellChromeを使用。
- TR-2: dark→light→darkの設定操作でdraftとColumn snapshotを比較。既存portal close後のfocus復元も確認。
- TR-3: 初期化scriptで毎回themeを書き直さず、実際の保存値をreloadで復元。lightは既存`shell.smoke.spec.ts`、darkは追加test。native再起動は別記録。
- TR-4: 既存component／browser suiteに加え、post・補助文字、selected、hover、error／warning、input境界と合成focusを両themeで検証。意味のないdisabled contrast閾値は追加しない。

追加browser testの初回実行は、reload後の既存inline composer復元を見落としたfixture操作で失敗した。製品コードは変更せず、切替往復を終えてからreloadを検証するよう修正し、3件PASSを確認した。

## 同条件の描画比較

browserはChromium、ja-JP、UTC、device scale factor 1、desktop 1600×1000／narrow 390×844。基準commitを先にbuildしたbundleを保持し、同じデータfixture・操作scriptでbefore／afterを取得した。narrowは同じColumn構成を既存ページ移動で確認する。

| 面 | dark desktop | light desktop | dark narrow | light narrow |
| --- | --- | --- | --- | --- |
| Columns | [前](assets/996/before-columns-dark-1600.png) / [後](assets/996/after-columns-dark-1600.png) | [前](assets/996/before-columns-light-1600.png) / [後](assets/996/after-columns-light-1600.png) | [前](assets/996/before-columns-dark-390.png) / [後](assets/996/after-columns-dark-390.png) | [前](assets/996/before-columns-light-390.png) / [後](assets/996/after-columns-light-390.png) |
| 表示設定 | [前](assets/996/before-appearance-dark-1600.png) / [後](assets/996/after-appearance-dark-1600.png) | [前](assets/996/before-appearance-light-1600.png) / [後](assets/996/after-appearance-light-1600.png) | [前](assets/996/before-appearance-dark-390.png) / [後](assets/996/after-appearance-dark-390.png) | [前](assets/996/before-appearance-light-390.png) / [後](assets/996/after-appearance-light-390.png) |
| 投稿作成 | [前](assets/996/before-composer-dark-1600.png) / [後](assets/996/after-composer-dark-1600.png) | [前](assets/996/before-composer-light-1600.png) / [後](assets/996/after-composer-light-1600.png) | [前](assets/996/before-composer-dark-390.png) / [後](assets/996/after-composer-dark-390.png) | [前](assets/996/before-composer-light-390.png) / [後](assets/996/after-composer-light-390.png) |
| フィードバックDialog | [前](assets/996/before-feedback-dialog-dark-1600.png) / [後](assets/996/after-feedback-dialog-dark-1600.png) | [前](assets/996/before-feedback-dialog-light-1600.png) / [後](assets/996/after-feedback-dialog-light-1600.png) | [前](assets/996/before-feedback-dialog-dark-390.png) / [後](assets/996/after-feedback-dialog-dark-390.png) | [前](assets/996/before-feedback-dialog-light-390.png) / [後](assets/996/after-feedback-dialog-light-390.png) |

現行の投稿作成はinline表示も使うため、AC-5が許容する「同等のportal面」としてフィードバックDialogを固定し、投稿作成と併せて比較した。送信操作は行っていない。

## validationと実機

- targeted Vitest: 4ファイル137件PASS（styles contrast／design-contract／css-vars、shellChrome）。
- targeted Playwright: `theme-palette.spec.ts`の3件PASS。
- `cargo xtask desktop-ui-check`: PASS。lint／typecheck、Vitest 183ファイル1597件、Storybook build、browser 267件、visual到達31件。
- Linux／Chromium baseline: Ubuntu24で全再生成後、比較31件PASS。既定の更新では許容差内の色差が残るため`--update-snapshots=all`を使用し、本文画素も確認した。Windows非CIのvisual成功は到達smokeのみ。
- CI初回の`linux-desktop-browser`: browser操作は成功、visualは30件PASS／1件FAIL。`developer-enabled-ja-dark.png`の差分はログ末尾のmonospace文字の字幅・折り返しのみだった。font指定とlayoutのproduction変更はなく、CIの実画像を確認して当該baselineを採用した。視覚比較の閾値・testは変更しない。
- CIの`linux-desktop-ui`は、同一製品コードで成功した後、socialGraphの複数画面操作が既定の合計実行枠5秒を超過した。再実行ではcolumnScopeとreactionsの長い操作も同じ理由で超過した。成功時にもsocialGraphは4928ms、columnScopeは4550msで上限に近かった（[成功ログ](https://github.com/KingYoSun/kukuri/actions/runs/34693062640/job/103551658827)、[再現ログ](https://github.com/KingYoSun/kukuri/actions/runs/34693673014/job/103558222387)）。既存notificationFocus testと同じ15秒の実行枠をこの3ケースだけに明示した。これはテスト全体のwatchdogであり製品の応答時間契約ではない。全操作、assertion、negative check、個々の`waitFor`／`findBy`の期限、global設定は維持する。配色以外の製品コードは変更しない。変更後は対象3ファイル11テストがPASS。
- [実画面contrast](assets/996/contrast-browser.json): axe-core 4.13.0で両theme×Columns／表示設定／投稿作成／フィードバックの8条件、違反0。フィードバック文字数表示のみ両themeで自動判定保留となり、実効foregroundと透明祖先・不透明panel背景を取得してdark 13.25:1／light 15.52:1を確認した。disabledの例外と区別する。
- native: 隔離したTauri 2確認用hostでproduction frontendを読み、browser mockデータを使用。OS・WebViewの実描画と実入力を確認するための環境であり、実P2Pやbackend同意処理の検証ではない。今回backend処理は無変更。
- Windows: ローカルWebView2（Chromium 152）、1280×840、ja。Computer Useでdark→light→darkを切り替え、light／darkそれぞれで終了・再起動後の選択復元を確認。日本語draft入力とEscape、focus復元、dark再起動時の200%表示（CSS viewport 640×420）も確認した。[light設定](assets/996/windows-appearance-light.png)、[dark設定](assets/996/windows-appearance-dark.png)、[入力とfocus](assets/996/windows-composer-focus-light.png)、[dark復元・200%](assets/996/windows-restart-dark-zoom200.png)。初回確認hostのCommon Controls manifest不足はhostだけを修正した。製品の変更ではない。
- Ubuntu24: `ssh local2`で確認用hostを準備し、ローカルRemote Desktop越しにComputer Useで操作。WebKitGTK 2.52.6、1280×840、ja。dark→light→dark、選択ラベル、Tabのfocus、Escape、light／dark各々の終了・再起動と保存復元を確認。dark再起動は200%（CSS viewport 640×420）で既存1 Column表示へのreflowと文字・操作の可読性を確認。[light設定](assets/996/linux-appearance-light.png)、[darkとkeyboard focus](assets/996/linux-keyboard-dark.png)、[light復元](assets/996/linux-restart-light.png)、[dark復元・200%](assets/996/linux-restart-dark-zoom200.png)。
- 両OSの[実効色・保存値・viewport](assets/996/native-observations.json)を記録した。記録中の接続表示はmockデータであり、ネットワーク経路の成功証拠ではない。画像中の青いpointer強調はComputer Use側の表示で、製品の光彩ではない。
- 未実施: screen readerの読上げ実測、Windows OS設定自体のHigh Contrast切替。accessible name／状態・DOM、forced-colors専用規則は変更せず、今回の確認は実contrast・既存a11y／keyboard回帰と対象OSの実描画へ限定した。Rust／CNの追加ローカルsuiteは変更path外のため実行せず、PRの既存必須CIで確認する。

## 終了・追加発見

固定AC／INVARを満たし、必須CIが成功したら承認済みの運用でmergeする。merge対象と検証差分を照合し、Issueの現在判定を更新してCloseする。任意の配色再探索や無関係なUI再設計は行わない。
