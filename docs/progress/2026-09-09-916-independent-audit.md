# Issue #916 独立監査

- 現在判定: **PASS**（code commit `aef944c05a9b6a18b6f31a0053b011bb1c5a9042`、Scope revision `916-r1`）
- inventory: 合計6 / 適合6 / 不適合0 / 未分類0、blocker 0。
- 公開証跡: [作業記録](2026-09-09-916-consent-gate-affordance.md)、[UI reviewと画像・計測](../ui-reviews/2026-09-09-consent-gate-affordance.md)、[PR #948](https://github.com/KingYoSun/kukuri/pull/948)。
- 以下は初回FAILから最終PASSまでの監査履歴。ローカルlog/一時script名は監査時の取得元で、公開可能な結果は上記recordとCIへ集約する。

# Issue #916 PR head 独立監査

- 対象 commit: `6819c14fdd8525db5e63d0a8b9227ad86aa7b448`（git rev-parse HEADで確認）
- 基準: `74016c04677e3b4032733213b8fa0f9f050a1819`
- Scope revision: `916-r1`
- PR: #948
- リスク区分: C
- 監査担当: 実装者とは別のコンテキスト independent_audit
- 暫定判定: **INCONCLUSIVE**。具体的blockerは現在0件。実機・addon-a11y・Linux visual比較と最終必須validationの証拠を待つ。成功済みの総件数だけからPASSにはしない。

## 構造とinventory

CodeGraph exploreを最初に利用し、表示された実ソースとgit diffを確認。続いて固定のrg式で全memberを逆引きした。CodeGraphのhost/tests.rsはindex該当なしだったためhost/mod.rsへ通常検索で到達。新規indexは作成していない。

| ID | 静的分類 | 再構築した経路と判断 |
| --- | --- | --- |
| INV-1 | 適合 | App:94-149 status読取。consent_requiredだけConsentGateへ。readyだけshellと3 persistence effectへ進む。初回initializingの100ms再取得、非active return、bridge unavailable既存fallbackも差分不変 |
| INV-2 | 適合（描画追加確認待ち） | App -> ConsentGateView。文書・checkbox・拒否はDOM/React stateだけ。Viewのproduction callerはAppの1箇所、Storybookは確認専用。固定height境界のnotice追加時は追加確認候補 |
| INV-3 | 適合 | handleAccept:204 refと年齢guard -> acceptAppConsents:59 -> invokeDesktop。production callerはAppのみ（api.tsはre-export）。native disabledとrefの二重抑止。文書slug/currentVersion、resolvedLanguage、age boolは従来契約を維持 |
| INV-4 | 適合 | handleAcceptのreadyのみapplyPendingDeviceRestoreFrontendState、適用trueのみreload。それ以外はonAccepted。catchはerror state、finallyはpending解除。backup helperのsnapshot/rollback/ackも不変。startup effectにもreadyのみ同helperという2caller |
| INV-5 | 適合 | lib.rs:339 command登録 + invoke_gate.rs allowlist -> accept_app_consents。validate -> operation lock -> running -> ConsentRequired -> restore phase確認 -> record -> 初期化/restore。年齢拒否はrecord内saveより前。既存DesktopState早期ready returnもrecord後 |
| INV-6 | 適合 | LegalDocumentViewのproduction consumerはConsentGateViewとAboutPanel、追加で既存component test。record_app_consentsはTauri commandとCLI ClientSession::accept_consents。CLIもvalidate/state/restore phase -> record -> host/activationで、全てproduction差分なし。Buttonはui/button import全fileを列挙（別名もimport pathで包含）、icon-button wrapperを含む。共有Buttonとglobal token/startup CSSに差分なし、新規CSSはapp-consentクラスのみ |

合計6 / 静的適合6 / 不適合0 / 未分類0。描画検証待ちは適合の最終確定と区別する。production IPC/sink追加・削除0、LegalDocumentViewのgate側caller移動と純粋なView抽出だけで、固定inventory想定と一致。

### sensitive sinkの逆引き

1. record_app_consents -> save_app_consent_store -> app-consent.json。全production callerはTauri/CLI。save自体の追加逆引きでreset_app_consent_at_path、CLIの明示reset、およびtest fixture writerを確認した。いずれも同意付与のUI入口ではなく、既存の失効/fixture経路として分類、差分なし。
2. command内spawn_desktop_initialization / build_desktop_stateはrecord成功に支配される。host::ClientHost::start_if_consented:183-194は未同意return後にのみstartへ入り、start内でaccount/runtimeを構築。invoke gateはready前のnetwork commandとrestore frontend state取得を拒否する。UIのmock結果を実network不使用の証明として用いていない。
3. orchestrate_restore_activation/activate_pending_restoreはAwaitingConsentとrecord成功後だけ。frontend applyはready成功のみ、snapshot適用後ack失敗なら前状態に戻る。上記既存Rust/TSソースは変更していない。

## 固定条件の証拠

| 条件 | ソース・test・結果 |
| --- | --- |
| AC-1 | View:29/55-69/87でreasonと双方aria-describedby。App `consent explains the missing age confirmation before any action` はvisible/description/チェック解除/拒否IPC0をassert。browserの3locale×2theme×3viewport matrixもdescriptionを検証。提供logで成功確認 |
| AC-2 | View:86/88/91はdisabled/busy/pending文言。base.css:220付近の限定selectorはopacity1、背景、inset境界を付与。有効primaryのshadowとの差分。画像/contrastとaddon-a11y最終観測待ち |
| AC-3 | View:38の独立focusable region、footer通常flow。base.css min-height:0/overflow-y:auto、低高さpage scroll。browser first-paintはクリック前に座標検証、matrixは操作viewport内と横overflowなし、Control+Endでscroll増加を検証。最後の文書内容の完全到達やnative/実200%zoomはこのassertだけでは証明しないため追加観測待ち |
| AC-4 | handleAcceptのguard/ref、Viewのnative controls。App二重clickでIPC1、browser Enter重複とSpace retry、checkbox Space/revoke/decline0、touch正のIPC1。Shift+Tab、無効button座標clickの直接観測は実機追加証拠待ち |
| AC-5 | 3 legal.jsonへ同じ2キーだけ追加、全文・metadataの変更なし。browserは保存済みlocale/theme起動、3localeのpending/error/retryとexact IPC languageを検証。App rejection後checkbox維持を検証。i18n parity成功。locale切替/実機の観測待ち |
| INVAR-1 | App:199-201の既存比較不変。App outdated/remount test、既存renewed consent test、browser現行申告済みcheckbox0/reason0/ageAttested false。Rust recordは保存済み現行申告なら再申告不要 |
| INVAR-2 | Appチェック/拒否はローカルstateのみ。App remount testは未保存選択false/IPC0。record testはmissing falseでファイル未作成、corrupt falseで既存bytes不変。host gate testはaccounts.json未生成。静的にruntime構築まで到達不能。非ready invoke gateの境界testは最終Tauri実行結果待ち |
| INVAR-3 | LegalDocumentView本体/API payload/restore helper/Rust production差分0。既存App testの全文title・metadata・参考訳・更新notice・failed分岐のassertを維持。新App ready restore testはapply完了前shellなし。既存deviceBackup testはactivated marker、ack失敗rollback、部分write rollback。adult表示setter追加0 |
| INVAR-4 | 既存testは追加のみ、削除/弱体化0。新CSS selectorはapp-consentに限定。Button/LegalDocumentView/AboutPanel/CN/startup error既存ソース不変。DESIGNのapp gate新節以外は節番号移動のみ。既存visual比較の最終結果待ち |

## transition照合

- TR-1: 初回・欠落・scroll・disabled -> gate。component/browser、Rust missing/corrupt/host gateに対応。
- TR-2: 明示checkbox -> accept1 -> pending -> status。App/keyboard/touch payload test。
- TR-3: checkbox解除でreason再表示/disabled/IPC0。componentおよびlocale matrix。
- TR-4: pendingでcheckbox/両button無効、ref guard、rejection後errorと選択保持、retry2。App deferred promiseとbrowser 250ms+failOnce。
- TR-5: declineはnotice stateのみ、reload/remountで選択消去。browser reloadとApp0call。native再起動追加証拠待ち。
- TR-6: 現行申告は不要、旧版は必要。既存renewed test、新outdated test、Rust current保存維持test。
- TR-7: load missing/invalidはdefault -> fail-closed。restore commandのphase/record guard、frontend ready guard、既存rollback tests。Tauri最終test証拠待ち。
- TR-8: shared styles/consumer diff不変、App startup errorとLegalDocumentView既存test成功。既存shell visual比較待ち。
- 401、複数node/global apply、background consent retryは登録点からこの変更フローに存在せず、固定Non-goal扱い。

## 検証

監査者は重いsuiteを追加実行せず、ソース・test assertionと以下の実ログを照合した。

- issue-916-targeted.log: App/LegalDocumentView/deviceBackup/i18n parity、4file 82test成功。
- issue-916-browser.log: 対象Playwright 25test成功。
- issue-916-runtime-consent.log: consent filter 29test成功。特にhost gate、missing/invalid、追加non-mutation testのtest名と成功行を個別照合。
- 監査者自身のgit diff --check（基準..対象head）: 成功。
- 全体desktop-ui-check、rust-test、Linux visual baseline比較、addon-a11y、native WebViewの最終結果は未確認。

## findings / 停止条件

- Existing-gap / Regressionの確定blocker: 0。
- 検証候補（未再現、まだblockerではない）: 390×541 / jaでerrorまたはdecline表示後、header/footer合計によりdocumentsが0近くまで縮む可能性。固定heightに対しmax-height:540でのみpage-scrollへ退避するため。親担当へ限定再現を依頼済み。再現すればAC-3/5、INV-2、TR-4/5のExisting-gapとして分類する。
- Optional-hardening: native disabledのpointer-events-noneによりcursor:not-allowedが見えないことは、ACがcursor変更を要求せず文字/背景/境界で説明するためblockerにしない。
- New-requirement追加なし。未知bugの不存在を条件にしない。

最終判定は証拠待ちのINCONCLUSIVE。未実施を成功扱いせず、対象head変更があればdeltaと影響先だけを再監査する。

## 追加再現・判定更新

上の暫定結果に対し、独立再現でblockerを確認したため、対象 `6819c14fdd8525db5e63d0a8b9227ad86aa7b448` の最終判定を **FAIL** とする。

- 分類: Existing-gap（今回のlayout差分で発生し、固定ACにも違反）
- 条件: AC-3/AC-5、INV-2、TR-4/TR-5。
- Sequence: Chromium、390×541、ja、dark、fresh consent -> Decline -> checkbox選択 -> accept -> 保存rejection。fixtureはproduction Appを起動する既存seedAppConsent({locale:'ja', failOnce:true})。
- 到達経路: View decline callbackはdeclinedを保持 -> handleAccept catchでerror追加 -> Viewはerrorとdeclinedの両Noticeをfooterに描画 -> 固定height panelのheader/footer flex-shrink:0によりdocumentsがmin-height:0まで縮む。
- 禁止された結果/利用者影響: 文書本文がほぼ全てclipし、scrollしても文書を読めない。文書全文到達を維持するAC-3に違反。画像を監査者が直接確認した。
- 計測: panel y16/h509、documents y183/h18/clientHeight16（上下paddingが合計16）、scrollHeight7316、scrollTop7300、footer y217/h299。最終段落bottom192だがdocの内容領域に有効な高さがない。
- 続けてcheckbox解除: documentsは同じ18px、footer bottom566に対しpanel bottom525。pageHeight566へ増えるが本文領域は復旧しない。
- 比較: 同viewportのinitialはdoc clientHeight134、decline単独60、error単独80。error+decline累積が破綻の具体的条件。
- 実行: `node --experimental-strip-types .codex/plans/issue-916-audit-reflow.mjs` と extra.mjs。既存distの4176はconnection refusedだったため独立previewを4188で起動。scriptにてpointer操作とDOM寸法を採取しfullPage screenshotを保存した。tracked file変更なし。
- 証拠: `.codex/plans/issue-916-audit-reflow.mjs`、`issue-916-audit-reflow-extra.mjs`、`issue-916-audit-541-both.png`、`issue-916-audit-541-both-unchecked.png`。
- 最小修正方向: documentsへ可読min-heightを持たせ、header/footer/noticeを含む内容が利用可能高を超えた場合はpage scrollで到達可能にする。hardcoded viewport境界だけで可読高を保証しない。
- inventory最終: 合計6 / 適合5 / 不適合1（INV-2） / 未分類0。
- blocker: 1。修正deltaと失敗sequenceの回帰testを再監査する。追加の必須validation待ちは引き続き未確認。

## Delta監査: aef944c05a9b6a18b6f31a0053b011bb1c5a9042

- 基準: 初回監査head `6819c14fdd8525db5e63d0a8b9227ad86aa7b448`。
- 対象: `aef944c05a9b6a18b6f31a0053b011bb1c5a9042`（git rev-parseで確認）。Scope revision 916-r1不変。
- delta: productionはgate専用base.cssのみ。flexをgridに変更し、本文行と本文自体に12remを確保、panelをmin-height:min-contentで内容に合わせて拡張可能にした。shared CSS/IPC/状態・保存sinkは変更なし。回帰test1本、Linux visual baseline2枚、作業記録を追加。
- 追加testの修正前ログ issue-916-audit-red.logを確認: `short windows preserve readable terms after declining and a save error` はclientHeight16、expected>=128で失敗。
- 修正後ログ issue-916-browser.logを確認: 同testを含む26test成功。

監査者自身が新しい一時script `issue-916-audit-delta.mjs` を実行し、既存fixture・現在distに対して次を確認した（既存4188 previewを利用）。

| viewport | error+decline+チェック解除後本文clientHeight | 末尾までの残px | 横overflow |
| --- | --- | --- | --- |
| 1280×800 | 288 | 0 | 0 |
| 390×844 | 259 | 0 | 0 |
| 390×541 | 190 | 0 | 0 |
| 760×480 | 214 | 0 | 0 |
| 640×400 | 190 | 0 | 0 |

全5条件で以下のassertが成功した。

- 初期のdisabled同意buttonへ座標pointer clickし、同意IPC0。
- Decline -> checkbox -> accept -> 保存失敗 -> checkbox解除後も可読本文高を維持。
- 文書focus + Control+EndでscrollHeight-scrollTop-clientHeightが0（末尾完全到達）。
- checkboxからShift+Tabで文書、Tabでcheckbox、Space選択、Tabでaccept、Tabでdecline、Shift+Tabでaccept。
- 有効acceptのfocus boxがviewport内、意図しない横overflow0。

390×541のdelta screenshotを監査者が直接表示して、末尾段落が可読でありfooterと重なっていないことを確認した。各画像は `issue-916-audit-delta-<width>x<height>.png`。初回再現scriptの再実行で `issue-916-audit-541-both-unchecked.png` は変更後画像へ上書きされたため、初回失敗の正本は初回計測値、初回both.png、およびred logとする。

`git diff --check 6819c14f aef944c0` は成功。新規testにより既存assertionの削除・緩和なし。CSSはapp-consent surfaceだけに支配され、INV-6へのstyle伝播なし。

**初回blocker: 解消。delta機能監査: PASS。inventory: 合計6 / 適合6 / 不適合0 / 未分類0。**

全体の監査判定は、native WebView・addon-a11y・最終必須validationの実行証拠待ちのため **INCONCLUSIVE**。CI自体はmerge gateとして別途成功が必要。これらの証拠が揃えば同じ全監査を再実施せず、証拠補完と必要なdeltaだけで最終判定する。

## 最終証跡の独立照合（aef944c0）

- `issue-916-rust-0.log`のcheck完了を確認。
- `issue-916-rustretry-0.log`を確認: nextest 887 passed / 4 skipped、別slice22 passed、Vitest160files/1275passed。失敗していたMSVC SDK探索と既存Vitest timeoutを成功として流用せず、再実行の完了を根拠にする。
- `issue-916-uirest-results.json`の4command exit0に加えて実browser log114passed、visual log18passedを確認。Windows visualはignoreSnapshotsのsmokeであり、Linux比較とは区別。
- gh run view 34248845019のheadShaが対象aef944c0と一致、linux-desktop-ui/browserのconclusion successを監査者が直接確認。残るCI jobsは親担当がmerge gateとして確認する。
- `issue-916-story-a11y.json`を解析し、7story×6組合せ=42case、violations0を独立確認。logもcases42/failures[]。disabled/理由のRGB pairも記録され、対象2themeで判読可能。
- Tauri同意4、invoke_gate1、restore filter2、legacy1の個々のtest名/成功行を照合。直接exe実行のためcargo runner全体成功とは同一視しない。restore filter2はinvoke allowlistと終了時待機testであり、activation全部のテスト件数とは表現しない。関連host restore/backup回帰はRust全suiteで補完。
- WebKitGTK2.50.4のafter JSON: initial checkbox/理由/acceptが800px内、document-endで4642+418=5060の末尾到達、checked/unchecked state、disabled-clickでcalls0、save-errorでchecked true/calls1、retryでcalls2/shell true、failures[]を確認。
- Windows WebView2隔離ホストのnative-after/native-error画像を監査者が直接表示。初期理由とdisabledの区別、文書とfooter分離、error時の選択保持・有効retryを確認。fake同意IPCを使う入力/描画実機であり、実network/storeの証明はRust境界testとソース支配関係に依拠する。
- 実screen reader音声の発話とOS設定としてのWindows High Contrastは未確認。ARIA description/semantic DOM/addon-a11y/forcedColorsとは同一視しない。この未確認だけで固定scopeへ新しい製品条件を追加しない。

実200%zoomの親担当による追加観測待ち。既存640×400 reflow・独立deltaの入力/文書末尾成功とは区別して追記予定。現時点では総合INCONCLUSIVEを維持し、具体的blocker0。

## 最終判定: PASS

- 対象commit: `aef944c05a9b6a18b6f31a0053b011bb1c5a9042`。最終時点でもgit HEAD一致を確認。
- Scope revision: `916-r1`、区分C。
- inventory: 合計6 / 適合6 / 不適合0 / 未分類0。
- blocker: 0（初回のnotice累積時本文clipは修正され、同一sequenceのred/greenと独立delta観測で解消確認）。
- 判定: **PASS**。固定AC-1〜5/INVAR-1〜4とINV-1〜6/TR-1〜8は上記ソース・既存/追加test・実行ログ・描画/実機証拠に対応している。これまでのFAIL/INCONCLUSIVEはそれぞれの当時の記録として維持する。

残っていた実200%zoomについて、監査者は `issue-916-webkit.py` のWebKit set_zoom_level(zoom)、2.0のget_zoom_levelチェックとXTest入力経路を読み、`issue-916-webkit-after-zoom200.json` とlogを確認した。WebKitGTK2.50.4で実engine zoom2.0、1280×800window -> DOM640×400、scrollWidth628で横overflowなし、本文clientHeight190・scrollTop5826+190=6016で末尾到達、Tab後focus BUTTON・y317+h48<=400、calls0、failures[]。CSS transform/viewport縮小のみの確認とは異なる。

`issue-916-webkit-after-zoom200-actions.png` を監査者が直接表示し、文書末尾、checkbox、focus表示中の同意button、拒否の可読性・到達性と非重複を確認した。Windows `issue-916-native-retry-result.png` も直接表示し、保存失敗後の明示retryがsynthetic成功結果の既存status表示へ進むことを確認した。これは隔離ホストのsynthetic結果であり、runtime/networkが実際に起動した試験とは表現しない。

### 明示する未確認範囲とmerge条件

音声screen readerの実発話、OS設定そのもののWindows High Contrast、Windows WebView2の実200%zoom（確認ホストでzoom hotkey無効）は未確認。実engine200%zoomはLinux WebKit、reflowはChromium、forcedColors/ARIA/addon-a11yは別証拠であり、上記未確認を全OSでの完全な確認として扱わない。固定ACを満たす観測と実装の具体的な裏付けが揃っており、この範囲を新要件としてblocker化しない。

監査PASSは残る必須CIの免除ではない。merge担当者は全必須CI成功を確認し、merge treeがこの監査対象のsurfaceと一致することを確認する。docsのみの追記を含めheadが変わる場合は差分を確認し、製品surfaceが変わればそのdeltaと影響先だけを再監査する。
