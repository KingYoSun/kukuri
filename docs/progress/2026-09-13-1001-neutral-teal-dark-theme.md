# #1001 Neutral / Teal dark theme

## Scopeと判断

- Issue: [#1001](https://github.com/KingYoSun/kukuri/issues/1001)
- PR: [#1002](https://github.com/KingYoSun/kukuri/pull/1002)
- Scope revision: 2026-09-13-r1。リスクB、親Issue・reopen・shared guard変更なし。独立監査必須条件には該当しない。
- 基準: `7d74328ef1be13bd35d8a97d473e5802b8ec4276`。ユーザーは配色案、WCAGに必要な小幅調整、実装・Issue・commit・PR・CI後merge、Windows/Ubuntu24の実機確認を承認済み。
- 完了条件・invariant・transitionの固定内容はIssue本文を正とする。製品契約はDESIGN.md、実行値はtokens.css。

darkの背景`#121212`、パネル`#292929`、primary`#03dac5`、本文`#ffffff`をそのまま採用した。primary内文字は`#00332e`、補助文字は`#b3b3b3`、入力面は`#202020`、装飾線は`#3d3d3d`、操作識別境界は`#858585`。primary内の白文字は1.77:1のため使わない。暗色文字なら7.82:1。warning/destructive等の意味色は維持する。

選択肢は、A:今回のNeutral / Teal、B:現行Graphite / Orange、C:初回deep-teal。ユーザーがAを選択し、本文・primary・補助文字のcontrastと無彩色の大面積surfaceを両立できるため採用した。生成画像の構造や画素値は仕様・実装証拠として流用しない。

## Surface inventoryと変更境界

CodeGraphでtheme.ts、Foundations、contrast.test.ts、design-contract.test.ts、theme-palette.spec.tsの現行sourceと参照を確認した。CSSはindex対象外なので直接読み、旧orange値とtoken consumerを検索した。

| 固定ID | member / 列挙方法 | 読み書き・副作用・invariant | 確認面 |
| --- | --- | --- | --- |
| INV-1 | `lib/theme.ts`のread/write、App、表示設定、theme-palette tests | 既存localStorageのみ。keyとdark fallbackは不変 | TR-1/2: 1600/390px切替・reload・不正値、native保存復元 |
| INV-2 | `visual.spec.ts`のTimeline/Thread/Author/Profile/Notifications/Messages/Explore、Timelineのbookmark tab | 表示色のみ。Column id/pin/span/order/draft/focus/scrollを保持 | before/after、theme-palette、既存visual/localization tests |
| INV-3 | Composer、Settings、Control Center、Dialog/Popover/Tooltip。`styles/index.css`の全local importからprimary/accent/border/ring consumerを列挙 | portalとshell双方にroot tokenを適用。handler・DOM・cascadeの順序は不変 | browser実効色、component/Foundations、native pointer/keyboard |
| INV-4 | Appの初回同意・startup error。`base.css`のshell外surface | guard・同意内容・保存処理は不変 | visualのconsent disabled/ready、関連既存tests |
| INV-5 | live/game/feedback/connection/developer visual cases、Notice/Badge | 既存状態labelと意味色を保持。3D/media内容とnetwork処理は対象外 | 既存visualとcontrastのsemantic pairs |

group追加・削除は0。既存参照先の色更新のみ。画像crop用storyのSVG内orangeは画像コンテンツ、warning系badgeは意味色であり置換対象外。未分類0。新規tokenは追加せず、dark/global/lightのparser契約を維持した。

## AC / INVAR → 実装・証拠

| 条件 | 実装・test | 証拠 |
| --- | --- | --- |
| AC-1 | tokens.css darkブロック、theme-paletteのColumn/portal/primary実効色 | [dark before](assets/1001/before-columns-dark-1600.png) / [after](assets/1001/after-columns-dark-1600.png) |
| AC-2 | contrast.test.tsの実用途ペア・RGBA ring、primary内文字、強い境界 | [browser contrast](assets/1001/contrast-browser.json)、native focus画像 |
| AC-3 / INVAR-1,2,3 | theme-paletteの切替・draft・focus復元・Column一致・reload・不正値、lightの期待値維持 | [light before](assets/1001/before-columns-light-1600.png) / [after](assets/1001/after-columns-light-1600.png)、[before実効値](assets/1001/before-computed.json) / [after](assets/1001/after-computed.json) |
| AC-4 | DESIGN §2/§11/token表、Foundations Tokens、旧reviewから新reviewへの参照 | [UI review](../ui-reviews/2026-09-13-issue-1001-neutral-teal-dark-theme.md) |
| AC-5 | path必須desktop-ui-check、Linux/Chromium視覚回帰、Windows/Ubuntu24実機 | 下記validation。未実施をPASSと扱わない |

## Accessibilityと画像確認

通常文字4.5:1、必要な非テキスト情報3:1を満たす実用途ペアを検証する。focusは既存2px outline/2px offsetを維持し、primary本体と輪郭を分離。disabled controlの規格上の例外と、読ませる状態説明を区別する。色だけによる選択・意味状態の伝達は追加しない。

主要4画面（Columns / Appearance / Composer / Feedback）×dark/lightのaxe color-contrast違反は0。Feedbackの文字カウンターのみ両themeで自動判定がincompleteとなり、実効色を確認した。darkは白/`#292929`で14.55:1、lightは`#242424`/白で15.52:1。祖先opacityは1であり、保留を自動PASS扱いせずこの確認で補完した。その他のWCAG要件を含む全製品への適合宣言ではない。

Linux/Chromium baselineは[run 34734032623](https://github.com/KingYoSun/kukuri/actions/runs/34734032623)で生成した。darkの13画像だけが変更され、light baselineはbyte単位で不変。Windows生成物でbaselineを更新していない。

## 実機条件

WindowsローカルのWebView2と、Remote Desktopで接続中のUbuntu24 / WebKitGTKをComputer Useで操作。外部投稿や本番profile変更を避けるため、現行frontendをmock APIでビルドし、Tauri WebviewWindowの独立profileへ読み込んだ。これはnative WebView描画・pointer/keyboard・theme保存の確認であり、P2P通信や本番アカウントのE2E検証ではない。検証hostのタイトルは再利用元の「#996 Graphite / Orange」のままだが、読込bundleは今回のbefore/afterであり、計測したColumn色でも区別した。

- Windows: WebView2 Edge 152、1280×840 CSS px（100%）。新配色、Composerへの日本語下書き入力、Escapeで閉じた後のprimary focus、Appearanceでdark/light切替を確認。[before](assets/1001/windows-before.jpg) / [after](assets/1001/windows-after.jpg) / [Composer focus](assets/1001/windows-composer-focus.jpg) / [dark設定](assets/1001/windows-appearance-dark.jpg) / [light設定](assets/1001/windows-appearance-light.jpg)。独立profileを再利用して検証hostを終了・再起動し、light選択の保持と200%表示を確認した。[再起動・200%](assets/1001/windows-restart-light-zoom200.jpg)。
- Ubuntu24: WebKitGTK、初期1280×793 CSS px。新配色のColumnとAppearance、dark→light→dark、Tabによるfocus表示を確認。[before](assets/1001/linux-before.jpg) / [after](assets/1001/linux-after.jpg) / [dark設定](assets/1001/linux-appearance-dark.jpg) / [light設定](assets/1001/linux-appearance-light.jpg) / [keyboard focus](assets/1001/linux-keyboard-focus.jpg)。検証hostを終了・同じprofileで再起動し、dark保存と200%表示を確認。[再起動・200%](assets/1001/linux-restart-dark-zoom200.jpg)。
- shell外の初回同意は、既存consent fixtureと同じstartup statusを返す専用検証hostで未選択状態を表示した。[Windows](assets/1001/windows-consent-disabled.jpg) / [Ubuntu24](assets/1001/linux-consent-disabled.jpg)。理由文、無効ボタンの破線と状態label、独立した本文scroll領域を確認。年齢申告・同意の送信は行っていない。
- native計測: [Windows](assets/1001/windows-native-observations.jsonl) / [Ubuntu24](assets/1001/linux-native-observations.jsonl)。テーマと保存値、Column id/実効背景、viewport、focusの変化を記録した。

## Validation

- 変更前のstyles targeted: 3 files / 121 tests PASS。
- theme-palette browser: 3 tests PASS（1600/390px・不正保存値）。primary文字/背景・focus outlineとoffset・portal本文色の期待値を追加。
- `cargo xtask check`: PASS（fmt、workspace clippy、Tauri check、frontend lint/typecheck）。
- `cargo xtask test`: Rust 911 PASS / 4 skipped、harness 22 PASS、doctest PASS。後段のfrontend suiteは181 files / 1610 tests PASS、2 files / 3 tests timeoutでFAIL。ローカル全体成功とは扱わない。
- `cargo xtask desktop-ui-check`: lint/typecheck成功後、frontend suiteは180 files / 1608 tests PASS、3 files / 5 tests timeoutでFAIL。後段のStorybook/browser/visualは別実行とCIで補完した。
- ローカルtimeoutはRust・UI検証の同時実行時に観測。初回失敗したAccountKeyPanel / workspaceResilience / channelsは、対象を分けた再実行で全件成功（前2 filesは11件、channels単独11件）。次の全体実行で別ケースにもtimeoutが出たため、全体結果の補完は同じ製品差分のCIを用いる。test timeout・assertion・本番の操作コードは変更していない。
- `npx pnpm@10.16.1 storybook:build`: PASS。`cargo xtask desktop-storybook`はWindowsで実行中のxtask.exeを再リンクできず起動前に失敗したため、同じpackage scriptを直接実行した。
- `npx pnpm@10.16.1 test:e2e:browser`: 270 tests PASS。既存ja/en/zh-CN、狭幅・200% effective viewport、各種状態と操作を含む。
- Storybook 12状態（Tokens、Button、pressed/disabled IconButton、StatusBadge、更新checking/success/error、接続read-error/connecting/retrying、cached feedback）でaxe color-contrast違反0。[結果](assets/1001/story-contrast.json)。addon-a11yと別実行のaxeの競合を避けて描画後に検査した。Tokensの折り返されたcode文字2箇所が背景推定incompleteだったが、画像で重なりがないことと既存semantic pairの値を確認した。[Tokens描画](assets/1001/foundations-tokens--desktop-width.png)。
- Chromium forced-colors / reduced-motionでdark/light×ja/en/zh-CNの設定radio focusを確認。初回同意の未選択状態も3 localeで確認した。[追加結果](assets/1001/additional-validation.json)。Windows OS全体のHigh Contrast設定を切り替えた検証ではない。
- [不変条件チェック](assets/1001/invariant-checks.json): light tokenブロックとTailwind aliasは変更前と一致、before/afterのColumn id・幅・高さは一致。手動取得画像には一部レンダリング時点の差があるため、lightのpixel回帰判定は決定的なLinux baselineとCIで行う。
- 製品差分 `6f8835f5` の[CI run 34734288810](https://github.com/KingYoSun/kukuri/actions/runs/34734288810): Linux UI全suite・browser/visual・Rust・static・CN・smoke・Windowsの全job PASS。最終headの全CIとAppImage buildの判定はPR checksを正とし、すべて成功後にmergeする。
- 性能: tokenと描画確認面のみ、DOM構造・handler・motion・取得データの追加なし。新たな性能計測は対象外。

未実施の補助的確認: 実タッチデバイス、screen readerによる全読み上げ、Windows OSのHigh Contrast設定切替、本番アカウント/P2PのE2E。この変更は色tokenと確認面のみであり、browser/keyboard/forced-colorsとnative WebView描画で変更範囲を検証した。製品全体のWCAG適合宣言は行わない。

AC-1〜4とINVAR-1〜3は上記証拠に対応。AC-5のマージ条件は最終PR headの全CI成功。Scope追加・guard変更・未分類surface・重大な配色不適合は0。今回の検証用host/scriptは製品差分に含めない。
