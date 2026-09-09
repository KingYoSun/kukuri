# Issue #918 — 見つけるの見切れとカラム内の文字密度

## 現在判定

- 実装・ローカル検証の記録。リスク区分B、Scope revision `918-r1`。CI・merge・Closeの最終状態は対象PRとIssueのCurrent statusを参照する。
- 基準commit: `e241d304e44cf656707f19e4ffa710a3a9a94528`。
- 2026-09-09に計画・実装・commit・PR・必須CI成功後のmergeを承認済み。
- [Issue #918](https://github.com/KingYoSun/kukuri/issues/918)。検索操作・ノード案内の見切れと、追加コメントのカラム本文・Control Centerの文字密度が対象。ヘッダー・カラム構成・同意／認証／検索契約は維持する。
- PR識別子: `codex/issue-918-explore-overflow-density`。独立監査はBの適用条件（親Issue・Reopen・shared guard）を含まないため非該当。

## 固定AC / INVAR

| ID | Yes / Noで判定する条件 |
| --- | --- |
| AC-1 | 1280×800・日本語・3カラムの見つけるで、3タブ、検索input、実行buttonの文字とhit areaがカラム内に収まり、各操作をpointer / keyboardで実行できる。発見・おすすめの説明文も全文読める |
| AC-2 | 未同意、利用可能ノードなし、明示選択先が利用不可、接続準備中、retry待ち、問い合わせerrorで、理由・長いURL・回復buttonを折り返しまたは縦scrollで全文確認して操作できる。複数ノードでも対象を識別できる |
| AC-3 | カラム本文とControl Centerの文字階層がT1で固定する役割別token・行高に一致し、過大だった本文・操作ラベルが適正化される。変更前後のcomputed styleと比較画像があり、カラムヘッダーの文字サイズ・階層は維持される |
| AC-4 | 3言語、dark / light、下記の代表viewport、200% zoomでclipping・overlapがなく、必要な情報と操作に到達できる。document-level横overflowはなく、既存Canvas scroll / mobile pagingは利用できる |
| INVAR-1 | 検索先・scope・検索条件・選択tabと、同意・認証・機能利用可否・retry期限・pendingによる既存制限を維持する。表示・resizeだけでAPI呼出しや同意保存を増やさない |
| INVAR-2 | レイアウト変更でquery / draft / 選択Columnを消さず、Control Centerの開閉・設定からの復帰で既存のfocus・文脈を維持する |
| INVAR-3 | accessible name・DOM順・focus可視性、Desktopの最小target条件、Mobileの44px目標、既存の色とcontrastを維持する。文字を隠す・対象外の同意Dialogを縮小する方法でACを満たしたことにしない |


## 固定surface inventory

基準は上記commit。CodeGraphでcomponent / callerを確認し、CSS適用先は下記の機械的な列挙で補完した。差分は表示class・styleと検証面であり、API入口・guard・sinkの追加削除は0。6 groupすべて適合、未分類0、不適合0。

| ID | 入口・trigger / member | shared helper | 読み書き・副作用 | guard / invariant | transition / 証拠 |
| --- | --- | --- | --- | --- | --- |
| INV-1 | `CommunityIndexWorkspace`の3タブ、query入力、submit。呼出元 `DesktopShellPrimaryWorkspace` | `runQuery` / `operationMethod` / `invalidateResults` | 既存検索API、query・結果state | INVAR-1/2、既存disabled・retry・context guard | TR-1/2/5、component test・`community-index.spec.ts` |
| INV-2 | Workspace内の未同意、ノードなし、選択先不可、allJoined、retry・error分岐 | consent flow、既存設定・retry callback | 明示操作で既存Dialog・設定へ進む、既存再試行 | INVAR-1/3、同意・検索制限を変更しない | TR-3/4、既存consentGate test・長文fixture |
| INV-3 | `CommunityIndexAvailabilityNotice`。Workspace / `CommunityNodeOnboarding` / Story | 渡されたavailabilityとcallback | 既存retry timer、明示再試行・設定・auto操作 | INVAR-1/2/3 | TR-3/4、既存onboarding試験とStory |
| INV-4 | カラム本文。Timeline / Explore / Notifications / Messages / Conversation / Profile / Thread / Stream / Game / Metaverse | 本文コンテナ・semantic typography、PostCard等の読書本文 | 描画のみ。既存data取得・保存は変更しない | AC-3/4、INVAR-2/3。適用selectorとcomponent対応は下記の確認済み一覧を参照 | TR-5、localization / visual / 代表Story |
| INV-5 | `DesktopShellControlCenter`の本文、カラム・参加先一覧、追加・設定・保存layout等の既存action | `FilterableTopicNavList` / `SavedWorkspaceLayouts`、既存callback | 既存操作の副作用のみ。文字変更自体は描画のみ | INVAR-1/2/3 | TR-5/6、localization / shell smoke |
| INV-6 | 共有styleから影響し得るヘッダー、portal内設定・規約Dialog | CSS cascade / primitive | 変更禁止領域への漏れを確認 | AC-3、INVAR-3 | TR-5/6、変更前後比較・既存consent系試験 |


## 状態遷移

| ID | 事前状態 | event / sequence | 期待状態 | 許可するI/O | 禁止する副作用 | 証拠 |
| --- | --- | --- | --- | --- | --- | --- |
| TR-1 | 日本語、1280×800、3カラム、ready | 見つけるを表示→query入力→検索 | 文字・操作が収まり結果表示 | 既存検索request | resizeによる追加query、重複submit | 変更前失敗・変更後成功のbrowser test |
| TR-2 | ready、queryあり | 発見→おすすめ→検索、各実行→loading→成功または空 | 各ラベル・説明・pendingが読める。結果は選択文脈に一致 | 各明示操作の既存API | pending制限の解除、異なる文脈の結果表示 | 既存component / browser test＋幅確認 |
| TR-3 | 未同意またはノードなし・選択先不可、長いURL、複数ノード | 案内表示→規約確認／設定／autoへの既存回復 | 全文と対象が分かり、既存の回復先を開く | 明示回復による既存I/O | 表示だけの同意保存・未同意query | consentGate test、長文browser / Story |
| TR-4 | connecting、offline、429待ち、query error | 状態更新→待機→期限後retry→失敗または回復 | 文・button・残秒が収まり、errorと空結果を区別 | 既存timerと許可された再試行 | deadline中のretry、表示変更に伴う再認証・新しいpolling | component回帰・availability Story・browser |
| TR-5 | query / draft / 結果を保持、本文とヘッダー表示 | 幅変更・200% zoom・locale / theme切替・カラム移動 | 本文の階層と全文到達を維持、headerサイズ不変、入力保持 | 既存の設定反映のみ | rem基準やlayout保存形式の変更、表示変更によるstate初期化 | localization / visual、実操作・computed style |
| TR-6 | Control Centerを閉じた通常画面、既存layoutあり | 開く→縦scroll→設定へ進む→戻る→閉じる。reload後にも表示確認 | 本文の密度と全actionへの到達、既存focus復元、保存layout保持 | 既存action・既存保存値の読込 | 文字変更のためのlayout書換え、portal縮小 | shell smoke / localization、同条件画像 |

認証・復元・破損DB・複数accountへのglobal applyの制御は変更しないため、新しい全状態直積は作らない。表示対象となる未取得・失敗・未同意・mixed node stateはTR-3/4で確認し、既存境界testを継続する。


## 修正前の再現・採用判断

1280×800、日本語、dark、3カラム（Timeline / Notifications / Explore）と、現行のfresh 5カラムを確認した。報告時の3ペインを新しい既定値にはせず、カラム幅・構成・保存形式は維持した。

| 観測対象 | 変更前 | 変更後・採用理由 |
| --- | --- | --- |
| 未同意Nodeの長いURL入り規約button | 幅374pxのWorkspaceのclientWidthが372pxに対しscrollWidthが839px。暗黙grid trackの最小幅がURLで膨らみ、tab・本文・検索actionも右端へ押し出される | `minmax(0, 1fr)`と局所的な折り返しでscrollWidthが372pxへ収まる。URLの値・callbackの送信先は保持 |
| 通常readyの3タブ | 現行では既に2列＋最終タブ全幅で表示できた | 既存構成を保持。フォームはviewport依存の横並びをやめ、実幅に応じて縦積み／2列を選ぶ |
| 本文・操作ラベル | カラム本文とControl Center本文は16px / 24pxを継承。入力要素のunlayered `font: inherit`が`text-sm`より優先される | 共通の本文コンテナへ既存`--text-body`を適用し14px / 21px。root fontを変えず、ヘッダーは16px / 24pxを保持 |
| 投稿本文・補助文 | 投稿本文は16px / 24px、Control Centerのカラム補助文は12.8px / 19.2px | 投稿本文は15px / 22.5px、カラム・参加先一覧の補助情報は12px / 18px。通常本文を補助サイズへ落とさない |
| 200%拡大時の検索action | keyboardでfocusできても固定page indicator / dockに覆われる。640×400の再現testで`occluded: true` | Mobileの見つける本文だけ固定UI用の下部領域を確保。任意のscroll位置とfocus移動で本文の表示領域が固定UIと重ならず、test・WebView実機で`clipped: false / occluded: false` |

修正前の失敗は、本文サイズ1件と長いURLの1280 / 760 / 390の3件、追加で固定UIの遮蔽1件。後者はAC-4 / INVAR-3のExisting-gapとして同じ作業に含めた。失敗testのassertionや既存suiteを削除・skipして成功扱いにはしていない。

## 確認済みの適用先と変更境界

- `.shell-column-body`: `ColumnSurface.tsx`、`DesktopShellPrimaryWorkspace.tsx`、`DesktopShellAuxiliaryPanels.tsx`。`ColumnKind`の10種（上表INV-4）へ同じ本文基準を適用する。header / footer / portalの基準サイズは変更しない。
- `.post-body`: `PostCard.tsx`、`LiveSessionPanel.tsx`、`GameRoomPanel.tsx`、`ProfileConnectionsPanel.tsx`。既存の読書本文containerへ15pxを適用する。独自の見出し・caption指定は保持する。
- `.shell-control-center-grid`と限定した`small`: `DesktopShellControlCenter.tsx`の本文・action・カラム一覧と、配下の参加先一覧。rootのfont-size、共有Button / Input / Notice primitiveは変更しない。
- `CommunityIndexAvailabilityNotice`: Workspace、`CommunityNodeOnboarding.tsx`、Storyを確認。修正はclass付与だけであり、callback・retry timer・同意条件は不変。
- `.shell-community-index-workspace / -form / -notice`は`styles/shell-phase1-part3.css`、本文サイズは既存`styles/shell-scoped-overrides.css`、Mobileの固定UI避けは`styles/mobile-column-workspace.css`が所有する。import順・token値・IPC・状態管理・URL保存schemaは不変。
- 列挙は`rg -l 'shell-column-body|post-body' apps/desktop/src -g '*.tsx'`と、CodeGraphのcomponent / caller確認で再生成できる。共有styleの呼出先を含め、入口や副作用の増減はない。

## AC / INVARと証拠の対応

| 条件 | 実装・維持箇所 | test / 証拠 |
| --- | --- | --- |
| AC-1 / INVAR-1 | Workspaceのgrid、フォームとタブ。`runQuery`等の処理は不変 | `community-index-layout.spec.ts`の3言語×2theme×2幅。3操作、pending制限、empty、APIの送信先・呼出回数、pointer / Enter / Spaceを確認。既存`community-index.spec.ts`も成功 |
| AC-2 / INVAR-1 | Workspace各Notice、AvailabilityNoticeと規約button | 長いURL2件の全表示と2番目Nodeへの規約取得、Escapeで未受諾を確認。既存consentGate / onboarding / retry関連testと、19 Story×3言語×2themeの114条件を確認 |
| AC-3 / INVAR-3 | scoped typographyと既存token、DESIGNの文字階層 | browserのcomputed style、header16px保持、投稿15px、caption12px、Windows / Linux WebViewの同条件比較。設定Selectの16px保持、同意・設定の既存baseline不変 |
| AC-4 / INVAR-2/3 | カラム実幅とMobile表示領域 | 1280 / 900 / 760 / 390幅、3言語・2theme、640×400のreflowで遮蔽とclipを検査。Windows WebView2 / Linux WebKitGTKのnative 200% zoomでscroll、入力、focus、検索を確認 |
| INVAR-2 | UIのquery・layout・focus、既存callback | resize中の入力とColumn配列保持、Control Center→設定→Escapeのfocus / active Column / query保持、reload後3カラム復元を確認 |

`TR-1/2`はquery操作matrix、`TR-3`は長URLの規約操作と既存consent試験、`TR-4`は既存unit testとAvailability Story全状態、`TR-5/6`はreflowとControl Centerの復帰testに対応する。high-level件数だけを受入根拠にせず、内部文字Range・hit area・既存callbackへの対応を確認した。

## 検証結果

| 検証 | 結果 |
| --- | --- |
| `cargo xtask check` | 成功（Rust / Tauri compile、frontend lint・typecheck） |
| `cargo xtask test`のRust部分 | 887＋22件成功、既存skip4件、doctest成功 |
| `cargo xtask test`の初回Vitest | 1300件成功、既存shell integration 2件が5秒／10秒でtimeout。timeout値・test内容は変更していない |
| timeoutした2ファイルの単独rerun | `DesktopShellPage.channels.test.tsx` / `.profile.test.tsx`の19件成功。後続の全Vitestも162ファイル・1302件成功 |
| `desktop-ui-check` | lint、typecheck、Vitest1302件、Storybook build、browser137件、visual smoke20件成功。最初のCargo起動は実行中xtask.exeの置換がWindowsで拒否されたため、同じ未変更sourceからbuild済みの`target/debug/xtask.exe desktop-ui-check`で完走 |
| 追加layout test | 19件成功。通常のdocument overflowだけでなく、内部文字Range、button bounds、hit test、固定UIと本文領域の分離を検査 |
| Linux / Chromium | CI同等のNoto CJKでvisual20件、layout / localization35件成功。本文・フォーム変更の既存7枚を更新し、日本語Explore・Control Centerの2枚を追加 |
| Storybook addon-a11y | Workspace / Availabilityの19 Story×3言語×2theme、114条件すべてWCAG 2 A / AA・2.1 AA・2.2 AAの違反0、横overflow0 |
| Native WebView | Windows 11 Pro 10.0.26200 / WebView2 152.0.4191.66、WSL Ubuntu 22.04 / WebKitGTK 2.50.4。1280×800の前後、200% zoom（実際のinnerWidth640 / innerHeight400）、入力・検索・Control Centerを確認 |
| `cargo xtask oversized-files` / `git diff --check` | 成功。baseline上限は変更しない。scoped stylesheetは999行 |

Native確認は製品のfrontend bundleを隔離したTauri hostへ読み込み、in-memory APIで行った。製品profile・ネットワーク・同意記録は操作していない。WebKitGTKはXvfb上の実エンジン描画・xdotool入力、Windowsは実WebViewウィンドウと入力で確認した。

## 確認中に区別した事項

- DockerのPlaywright imageにはCI runnerにないWenQuanYi / IPA fallbackがあり、当初は未変更の日本語同意画面にも差分が出た。Noto CJKを導入して余分なfallbackを除き、CIと同じ条件で比較し直した。未変更の同意画面baselineを上書きして吸収せず、最終的にbyte一致を確認した。
- 高速なviewport / CSS zoom往復の直後にactive Columnが通知へ更新される挙動は基準commitでも同じだった。既存Mobile pagingのactive更新と今回の入力・Column削除を混同せず、resize時の配列／query維持と、通常Control Centerのactive／focus復帰を独立したsequenceとして検査した。
- 小さいviewportでは全内容を同時表示することを条件にせず、縦scrollで全文へ到達することを確認した。固定UIで操作の一部が隠れる状態は許容せず、上記の専用検査で保護した。
- screen reader、Windows High Contrastの専用手動操作、実データを伴う検索serverの再検証は実施していない。semantic name・role・DOM順・色・検索／同意処理は変更せず、addon-a11y・既存boundary test・実操作と差分確認で保護した。大きな一覧やGPU resourceの性能は今回変更していない。

## 視覚証拠・終了条件

同条件の画像と採用判断は[UI review record](../ui-reviews/2026-09-09-918-content-density.md)へ集約した。内部layoutの横overflowは839→372px、本文fontは16→14px、headerは16px維持。新たなdata取得・timer・event listenerは製品へ追加していない。

全AC / INVARを固定inventory・transition・証拠へ対応付けた。実装差分に関するblockerは0。PR headの必須CI成功後に承認済みの運用でmergeし、merge treeと検証対象の一致を確認してIssueのCurrent statusとCloseを更新する。
