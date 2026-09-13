# Issue #1003 primary塗りボタンのオレンジ化とカラム上辺強調の削除

## 現在判定・承認範囲

- 状態: In progress。ユーザーは実装、Issue作業、コミット、PR作成、CI成功後のマージを承認済み。
- リスク区分: B（frontendの表示）。Scope revision: 2026-09-13-v2。基準commit: `143333d82dd7998e34c790ed75fc339c0981ed87`。
- [Issue #1003](https://github.com/KingYoSun/kukuri/issues/1003)。計画の現在の詳細と証跡は本書に集約する。
- v1のオレンジ上辺・未読数・未読枠はユーザーの追加指示で撤回。v2はprimary塗りボタンだけオレンジを維持し、カラム上辺グローは選択・固定を問わず削除、通知は元のtheme accentへ戻す。
- UI分類: 既存画面の改善。対象はカラムを閲覧・操作する利用者。目的は主操作の配色を整え、上辺装飾を除くこと。
- 非目標: 全体accent・focus・warning・dangerの変更、レイアウト再設計、通知集計・既読化・カラム選択・永続化・backendの変更。
- 正本: [DESIGN](../../DESIGN.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[Issue運用](../runbooks/issue-lifecycle.md)、[検証マトリクス](../../REFACTORING.md)。

## 固定AC／INVAR

| ID | 受入条件・維持する契約 | 対応する実装・検証 |
| --- | --- | --- |
| AC-1 | 「投稿」「結果を表示」等のprimary塗りボタンだけ両themeでオレンジ。hover・実行中・disabledの操作条件と文字の視認性を保つ | button用surface／foreground token、共通button CSS、orange-emphasis、theme-palette、既存consent・composer tests |
| AC-2 | active×pinnedの4組合せに上辺グローがない。選択外周枠・ラベル・aria-currentを維持し、固定解除でheader高さが変わらない | scoped CSSの固定上borderとactive inset shadowを削除、orange-emphasis実効CSS／geometry |
| AC-3 | 未読数と未読枠を基準commitのtheme accentへ戻す。darkは`#03dac5`、lightの数字は`#713717`、枠は`#a44a21` | Badgeとnotification CSSは基準と同一、orange-emphasisの通常／hover／focus |
| AC-4 | 実行token、DESIGN、Foundationsと採用記録が整合し、必要な検証・画像が揃う | design-contract、css-vars、contrast、Storybook、Linux visual、native確認 |
| INVAR-1 | 非primaryボタン、link、Badge、focus ring、warning／dangerは既存配色と意味を維持 | 共通consumer逆引き、theme-palette、contrast、visual |
| INVAR-2 | active、固定、span、順序、draft、scroll、focus復元、狭幅page移動・復元を維持 | DOM／handler／store変更なし、既存browser suiteとtheme-palette |
| INVAR-3 | 投稿・検索・通知件数・既読化・取得失敗時のstate保持を維持。I/O追加なし | 既存component／browser flow、新規testは既読化失敗fixtureで未読状態を観測 |

## 固定surface inventory・状態遷移

| ID | 入口・shared helper・consumerの列挙方法 | I/O・guard | transition・test |
| --- | --- | --- | --- |
| INV-1 | Button／buttonVariantsの全参照、`.button`付与、`surface-button-primary`・`primary-foreground`参照をCodeGraph→rgで逆引き。shell内／portalの共通button CSS | 表示値のみ、既存実行条件を維持 | TR-1: 通常→hover→focus→実行、disabled。orange-emphasis、consent、composer |
| INV-2 | ColumnSurfaceのactive／pinned属性、ColumnCanvas、scoped CSS | 既存stateを読むのみ | TR-2: 選択切替・固定解除の4状態、TR-3: reload／狭幅移動。orange-emphasis、theme-palette、workspace既存test |
| INV-3 | DesktopShellControlCenterの件数Badge、DesktopShellAuxiliaryPanelsのnotification-item | 既存件数とread状態を読む。API変更なし | TR-4: 未読0→1→複数→既読、TR-5: 未読hover／focus・自動既読失敗。notifications既存testとorange-emphasis |
| INV-4 | tokens／styles/index.css／Foundations／skip linkとreview CSS | themeの既存切替のみ | TR-6: dark→light→dark、portal、復元。theme-palette、contract、visual |

consumer分類: 共通`.button`はprimary塗り、secondary／ghostは既存上書き。shell skip linkはボタンでないため背景を同値の`--primary-start`へ付け替え、文字は`--primary-foreground`を維持。review用skip linkとStreamのspanも同様に配色を保持。Foundations Tokensは新しいbutton文字tokenへ同期し、Spacingの見本は実行surface色を描画する。一般primary tokenとTailwind aliasは変更しない。新規のconsumer group／I/O／guardは0。

## 作業・依存・証跡

| ID | 作業・path | 受入条件・証跡 | 依存 |
| --- | --- | --- | --- |
| T1 | 上記inventory、変更前の実効CSSと再現を固定 | v1→v2の変更前にorange-emphasisの上辺`none`期待が両themeで失敗、既存inset shadowを検出。投稿ケース2件はPASS | なし |
| T2 | `tokens.css`、`shell-phase1-part1.css`、`shell-phase1-part2.css`、`shell-scoped-overrides.css`、review CSS | AC-1～3／全INVAR。v2のorange-emphasis 4件PASS。通知CSSは基準へ復元 | T1 |
| T3 | DESIGN、Foundations Tokens、contrast、theme-palette、採用記録 | AC-4、実contrast・token同期・既存theme切替 | T2 |
| T4 | browser／Storybook／Linux視覚baseline／WindowsとUbuntu24の実機確認、PRとCI | 全AC／INVARの証跡、最終headの全CI成功後にマージ | T3 |

## 検証結果

- ボタン文字`#20160e`対背景`#d77d45`: 5.84:1、hover背景`#c86f38`: 4.89:1。背景からの境界・focusとその他の実用途ペアはcontrast.testで確認する。
- v2 targeted browser: `orange-emphasis.spec.ts` 4件PASS（dark／light、上辺なし、pin切替の高さ、未読数・枠、hover／focus、投稿ボタン）。
- v1のローカル`cargo xtask desktop-ui-check`はlint/typecheck成功後、変更前の文字tokenを参照するcontrast testが1件失敗。実際のbutton専用文字tokenとskip linkのペアに同期し、検証を継続中。全体成功とは扱わない。
- v1のWindows／Ubuntu24実機画像は撤回済み配色の証拠であり、v2の検証済み証拠として流用しない。Computer Useは前ターンにユーザーのEscapeで停止した。
- Linux baselineはGitHub ActionsのKukuri Visual Baselineで最終製品差分から生成する。Windowsのvisualはsmokeで、pixel比較成功の代替としない。
- 実機条件・最終コマンド結果・CIは作業完了時に追記する。

## 終了条件・未確認

対象viewport×theme×stateで描画・操作を確認し、必須validationと最終PR headのCIが成功したら終了する。性能はCSS値と描画規則のみで、新たなDOM・handler・animation・購読・取得なし。native確認はmock APIによるWebView描画・入力の確認とし、本番アカウントやP2Pの検証とは区別する。実タッチデバイス・screen reader全読み上げは対象外。失敗・中断・未実施は成功と区別して記録する。
