# Issue #916: 同意画面の年齢申告案内と操作到達性

- Issue: [#916](https://github.com/KingYoSun/kukuri/issues/916)
- Scope revision: `916-r1`（2026-09-09承認・固定）
- 基準commit: `74016c04677e3b4032733213b8fa0f9f050a1819`
- リスク区分: C
- 状態: 実装・ローカル検証・独立監査完了。mergeの現在判定は [PR #948](https://github.com/KingYoSun/kukuri/pull/948) とIssueを参照。

## 変更結果

初回同意画面の文書全文と年齢申告・同意操作を別領域にし、未チェックの理由を操作前に表示する。
native disabledを維持し、専用の境界・背景・影と理由文で有効状態と区別する。
低い画面ではpage scrollに退避し、文書や操作をclipしない。日本語・英語・簡体字中国語を揃える。

同意条件、法務本文・版番号、IPC・保存形式、runtime/network開始条件は変更しない。
`ConsentGate` がstateと送信を所有し、`ConsentGateView` が純粋な表示とStorybook確認面を所有する。
保存中の同一操作はrefで抑止し、保存失敗後のチェックを維持する。

工程: [Issue lifecycle](../runbooks/issue-lifecycle.md)、[PLANS.md](../../PLANS.md)、[REFACTORING.md](../../REFACTORING.md)。
製品契約: [DESIGN.md](../../DESIGN.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[ADR 0046](../adr/0046-age-attestation-adult-content-gating.md)、[app同意分類](../legal/app-consent-data-classification.md)。

## 受入条件と維持する契約

| ID | Yes / Noで判定する条件 |
| --- | --- |
| AC-1 | 年齢申告が必要で未チェックの間、checkboxと同意ボタンの近くに「続行するには、18歳以上であることをチェックしてください。」相当の案内が常時見える。hover・click不要で読め、checkboxとボタンのaccessible descriptionにも対応する |
| AC-2 | 未チェック時の同意ボタンはnative disabledを保ち、有効時のprimaryと背景・境界・影で区別できる。理由の文字を併用し、pendingは既存の処理中文言で別状態と分かる |
| AC-3 | 1280×800の初期表示でcheckbox・理由・同意／拒否が見える。規約全文を独立して末尾まで読め、footerが本文やfocusを隠さない。狭幅・低い高さ・200% zoomでも全文と操作へ到達でき、意図しない横overflowがない |
| AC-4 | pointer、Tab / Shift+Tab、Spaceによるcheckbox操作、Enter / Spaceによる有効ボタン操作で同じ結果になる。チェック解除で再び無効になり、未チェック・拒否では同意IPCが0回。pending中の追加操作も送信を増やさない |
| AC-5 | `ja` / `en` / `zh-CN`、dark / lightで理由・状態が読め、切替／保存済みlocaleの起動で翻訳漏れがない。保存失敗時は同じ操作領域に案内が残り、選択済みcheckboxを保持して再試行できる |
| INVAR-1 | 未申告または旧版申告では明示checkboxが必要。現行版申告済みの文書更新では再チェックを要求せず、不要な年齢未選択案内を出さない。初回申告と文書更新を混同しない |
| INVAR-2 | 明示的な同意操作前に同意保存・runtime構築・network開始・復元activationを起こさない。チェックのみ・文書scroll・拒否ではread-onlyの表示に留まり、再起動で未保存のチェックは引き継がない |
| INVAR-3 | 文書全文、metadata、参考訳、更新通知、拒否案内、表示版と言語を送るIPC契約、ready / initializing / failedおよび復元後frontend state適用の分岐を維持する。成人向け表示を有効にしない |
| INVAR-4 | 共有Button・startup error画面・AboutPanel・Community Nodeの画面に新しいdisabledやscroll規則を波及させない。global tokenの値や既存testのassertionを弱めない |

## 固定surface inventory

| ID | 入口・trigger | shared helper | 読み書き・外部副作用 | 必要なguard / invariant | 対象transition | test / evidence |
| --- | --- | --- | --- | --- | --- | --- |
| INV-1 | `App` cold start / reload / initializingのstatus再取得 | `getDesktopStartupStatus` → `ConsentGate` | 起動状態のlocal読取り・gate描画 | INVAR-1〜3。ready前のshell構築を増やさない | TR-1, TR-6, TR-7 | `App.test.tsx`、host起動gate test |
| INV-2 | `ConsentGate` 文書scroll・checkbox・拒否・locale/theme描画 | `ConsentGateView` → `LegalDocumentView`、Button、i18n | React内stateとDOMのみ | AC-1〜5、INVAR-2,4 | TR-1〜3, TR-5 | component / browser / visual / 実機 |
| INV-3 | 同意ボタンpointer／keyboard → `handleAccept` | `acceptAppConsents` → `accept_app_consents` | 文書版・言語・年齢boolをIPC送信 | 年齢条件とpending。INVAR-1〜3 | TR-2〜4, TR-6 | IPC引数・0回／回数test |
| INV-4 | 同意IPC成功／失敗 | `onAccepted`、`applyPendingDeviceRestoreFrontendState` | 起動状態変更、ready時の復元frontend state適用／reload | INVAR-3。失敗を成功扱いしない | TR-4, TR-7 | App統合test、既存restore回帰 |
| INV-5 | Tauri `accept_app_consents`（逆引き対象） | `validate_app_consent_documents`、`require_consent_acceptance_state`、`record_app_consents` | local同意保存→初期化またはrestore activation | INVAR-1〜3。拒否時は保存・構築なし | TR-1, TR-6, TR-7 | Rust境界testとcall path確認 |
| INV-6 | 表示／保存helperの別consumer | `LegalDocumentView` → AboutPanel・component test、Button全利用先、`record_app_consents` → CLI `Session::accept_consents` | 既存の設定表示／CLI同意保存 | INVAR-4。変更しない共有経路として分類 | TR-8 | caller逆引きとdiff、既存test |

入口の機械的確認は `ConsentGate` / `LegalDocumentView` / `acceptAppConsents` / `record_app_consents` のCodeGraph callerと、`rg -n 'ConsentGate|LegalDocumentView|acceptAppConsents|record_app_consents|accept_app_consents' apps/desktop/src apps/desktop/src-tauri/src crates/desktop-runtime/src/host crates/kukuri-cli/src` を併用する。Buttonはimportの別名も含めて利用先を確認する。runtime全体や全CN routeを今回の変更scopeへ拡張しない。

確定したinventory差分は、INV-2内の理由表示・本文scroll領域・footer、純粋な表示consumer `ConsentGateView` と試験fixtureの追加。productionのIPC・保存sinkの追加・削除は0。共有法務表示のgate側callerはAppからViewへ移り、AboutPanelは不変。各memberとguardを独立監査で確認し、未分類0・不適合0と判定した。

### Sensitive sink

- SINK-1: `host/consent_acceptance.rs::record_app_consents` → `save_app_consent_store` → `<db_path>.app-consent.json`。現行callerはTauri同意commandとCLI session。未チェック操作ではIPC 0回、Rust年齢拒否ではファイル未作成／既存bytes不変を確認する。
- SINK-2: `commands/app_consent.rs` の `spawn_desktop_initialization` / `build_desktop_state` → runtime・DB・identity・transport構築。未同意時は既存host gateとinvoke guardが支配する。UI試験のmock成功を実network未開始の証明にしない。
- SINK-3: 同commandの `orchestrate_restore_activation` / `activate_pending_restore` とfrontendの `applyPendingDeviceRestoreFrontendState`。本変更では呼出順・条件・payloadを維持し、チェック・拒否・IPC rejectionだけで進まないことを確認する。

## 状態遷移

| ID | 事前状態 | event / sequence | 期待状態 | 許可するI/O | 禁止する副作用 | test / evidence |
| --- | --- | --- | --- | --- | --- | --- |
| TR-1 | 新規／年齢未申告・未チェック | 起動→文書scroll→無効ボタン操作 | 理由可視、gate継続 | 起動状態のlocal読取り | 同意IPC・保存・runtime構築 | 初回component test、browser座標／実入力、host gate |
| TR-2 | 年齢未申告 | checkbox選択→同意 | 理由解除、有効→pending→返されたstatus | 明示同意IPC 1回、その後の既存処理 | checkboxだけで保存、pendingで重複送信 | App IPC引数test、browser |
| TR-3 | checkbox選択済み | チェック解除→同意を試みる | 理由再表示、disabled | DOM更新のみ | 同意IPC | component / keyboard test |
| TR-4 | checkbox選択済み | 同意→遅延／rejection→再試行 | pendingで操作抑止、errorとチェックを保持→明示retry | 初回と明示retryのIPC | 自動retry、errorをready扱い | deferred/rejected PromiseによるApp test |
| TR-5 | 未同意 | 拒否→再表示／再起動 | 拒否notice、gate継続。未保存チェックはfalse | 状態読取り、DOM更新 | 同意保存・activation | App test、実機 |
| TR-6 | 文書更新、年齢が現行版／旧版 | gate表示→必要な操作→同意 | 現行版はcheckbox不要・案内なし、旧版は再申告必要 | 表示版・localeに対応するIPC | 現行版への再申告強制、旧版の迂回 | 既存renewed consent test＋旧版case、Rust年齢test |
| TR-7 | 同意記録欠落／破損、または復元のAwaitingConsent | 起動／再起動→gate→拒否または同意 | fail-closed維持。承諾成功後のみ既存status／restore分岐 | local読取り、明示成功後の既存復元I/O | 未同意で構築・frontend restore apply | host破損記録test、App restore test、既存Tauri restore test |
| TR-8 | 起動error画面／AboutPanel／通常の同意済みshell | 閲覧・既存操作 | style・本文・入力を維持 | 既存I/Oのみ | gate専用styleの漏出 | 既存App / LegalDocumentView test、browser / visual |

401、CN再認証、複数nodeのglobal apply、background consent retryはこのgateの制御フローに存在せず対象外。送信先はlocal IPCであり、offlineを解消するための新規network操作は設けない。


## 実装・test・証跡の対応

| 条件 | 実装 | test / evidence |
| --- | --- | --- |
| AC-1,2 / INVAR-1 | `ConsentGateView` の理由・description・disabled、`base.css` のgate専用style、3 localeのgateキー | App「consent explains the missing age confirmation before any action」、browser layout matrix、Story Unchecked / Checked / RenewedConsent |
| AC-3 / INVAR-4 | 文書scroll領域、通常flowのfooter、低い高さのpage scroll | browser「consent actions are reachable on first paint without scrolling the terms」、3 viewport matrix、visualのwide/narrow、UI review画像 |
| AC-4,5 / INVAR-2,3 | `ConsentGate.handleAccept` のref/年齢guardと既存送信・status分岐 | App pending/error/retry・旧版/remount・ready/restore test、browser keyboard retry・touch input |
| INVAR-1〜3 / SINK-1,2 | Rust productionは不変。拒否時の保存不変をtest追加 | `missing_age_attestation_does_not_create_or_mutate_consent_file`、既存host起動gate・破損記録test、Tauri app_consent / invoke gate / restore test |
| INVAR-3,4 / SINK-3 | 法務全文・metadata・restore API・共有Buttonとstartup errorは不変 | 既存App / LegalDocumentView / deviceBackup test、shared caller逆引きとdiff確認 |

inventory差分: INV-2に理由、footer、文書scroll領域を追加。表示を `ConsentGateView` に抽出し、production callerは `App::ConsentGate` の1箇所、確認面は `ConsentGateView.stories.tsx`。
新規production IPC・保存sinkは0。`LegalDocumentView` のgate側callerだけがAppからViewへ移る。全callerの再確認と独立監査は最終headで記録する。

## 修正前の再現

- `App.test.tsx`: 理由文を要求する追加testが失敗、既存7testは成功。
- browser: 1280×800 / en / dark / 初回未申告で同意ボタン下端が4961pxとなり、viewport内の操作到達性assertが失敗。自動scrollで問題を隠さず、操作前の座標を確認した。
- 変更後: 対象browser 26testと関連Vitest 82testが成功。browserにはtouch操作と独立監査で判明した低い画面の回帰testも含む。
- 画像・実機条件: [UI review](../ui-reviews/2026-09-09-consent-gate-affordance.md)。報告OS / WebView版はIssue本文からは確定できず、当時の配布版そのものの再試験とは区別する。

## 検証結果

| 検証 | 結果 |
| --- | --- |
| `cargo xtask check` | 成功。Rust追加testのformat差分を修正して完走 |
| `cargo xtask test` | 成功。non-CN Rust 887 passed / 既存設定の4 skipped、harness 22 passed、doctest、Vitest 160files / 1275 passed |
| 同意関連targeted Vitest | App / LegalDocumentView / locale parity / deviceBackupの82件成功 |
| `cargo xtask desktop-ui-check` の各構成gate | 初回Vitestは既存messages/socialGraphの3件がtimeout（1272件成功）。対象2fileの12件と、その後の全Vitest1275件が成功。最終 `desktop-lint`（lint/typecheck）、`desktop-storybook`、`desktop-browser-test` 114件、`desktop-visual-test` 18件も成功。失敗したaggregate command自体を成功と呼び替えず、構成gateの補完結果として記録 |
| 同意browser | 26件成功。3locale×2theme×3viewport、pointer/keyboard/touch、pending/error/retry、再同意、低い画面のnotice累積、forced-colors/reduced-motionを含む |
| Storybook addon-a11y | 7state×3locale×2themeの42条件で違反0。ボタン文字contrast最小4.79:1、未選択理由13.54:1、disabled境界4.69:1 |
| Tauri境界unit | 同意4件、同意前invoke禁止1件、restore関連allowlist/終了待機2件、旧bundle再同意1件が成功。restore activation全体の件数とは扱わず、host backup/restoreはRust全suiteでも確認 |
| Linux visual | [生成run 34247865615](https://github.com/KingYoSun/kukuri/actions/runs/34247865615)の同意画面2枚を追加。既存16枚はbyte一致し変更なし。[head aef944c0のCI](https://github.com/KingYoSun/kukuri/actions/runs/34248845019)でlinux-desktop-browser（比較を含む）とlinux-desktop-ui成功。Windowsの18件は比較skipのsmokeとして区別 |
| 実機・描画 | Windows WebView2、Linux WebKitGTK2.50.4の同条件before/after・実入力、WebKit実zoom 2.0での本文末尾・操作到達・focusを確認。[UI review](../ui-reviews/2026-09-09-consent-gate-affordance.md)に画像と範囲を記録 |

### 環境補完と再実行

Windowsの `cargo xtask test` 初回はSDKの `ucrt.lib` 探索で失敗した。実在するSDK 10.0.22621.0のucrt/um x64 pathを検証processのLIBに加えて再実行し、suiteを完走した。製品のbuild設定は変更していない。

Tauri unit exeと隔離確認用exeにはCommon Controls v6 manifestがなく `STATUS_ENTRYPOINT_NOT_FOUND` となった。生成済みの同じunit exeへmanifest resourceを付け、上記filterを直接実行した。cargo runnerの成功とは混同せず、test名と実行結果で判定した。product sourceやtest assertionを回避する変更はない。

最初のnative確認用ホストのIPC接続、Linux fixtureのcache/入力dispatch間隔、cairo snapshot依存は確認環境側で補正した。成功した最終観測だけを実機の証拠にし、これらの準備失敗を製品不具合の再現には数えない。

## 独立監査と修正

初回監査はFAIL（blocker 1件）。390×541 / jaで拒否→チェック→保存失敗→チェック解除すると文書clientHeightが16pxになるExisting-gap（AC-3/5、INV-2、TR-4/5）を再現した。可読な本文の最小高さを持つgridとpanelのintrinsic minimumへ修正し、notice増加時はpanelが伸びてpage scrollへ退避する。

追加browser testは修正前16pxで失敗、修正後に文書末尾・focus・操作到達を含め成功。独立したdelta確認でも390×541の本文190px・末尾remaining0、5viewportのdisabled pointer IPC0、Tab/Shift+Tab、focus可視、横overflow0を確認した。

- 最終監査対象code commit: `aef944c05a9b6a18b6f31a0053b011bb1c5a9042`
- Scope revision: `916-r1`
- inventory: 合計6 / 適合6 / 不適合0 / 未分類0
- blocker: 0、総合判定: **PASS**
- 詳細: [独立監査記録](2026-09-09-916-independent-audit.md)。初回FAILと後続の補完経緯を維持する。

## 未確認範囲とmerge条件

音声screen readerの実発話、OS設定としてのWindows High Contrast、利用者の報告版・OSそのものは未確認。semantic tree、ARIA description、addon-a11y、forced-colorsや実engine描画を、それらの全面的な代替とは呼ばない。公開する文書・画像はsynthetic fixtureで、利用者の同意記録やアカウントを使わない。

実装を含む監査対象surfaceが同じであることと、最終PR headの必須CI成功を確認してからmergeする。後続の文書・証跡だけのcommitは対象surfaceの差分を確認して監査PASSを再利用する。merge後のtree一致とIssueのCloseはPR/Issueに記録する。
