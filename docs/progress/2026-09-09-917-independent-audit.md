# #917 独立監査

## 対象と判定

- 対象PR: #949、Issue: #917。
- 対象commit: `206bd4d18046fd0c63eecb22e9867afe3e2c94ad`。
- 比較起点: `62b75baeeb40c62d9ee4afedee4e584b4f20c14a`。
- Scope revision: `917-r1`、リスク区分C。
- 固定条件: Issue本文のAC-1〜6、INVAR-1〜4、INV-1〜7、TR-1〜9（作業用の本文写し `.codex/plans/issue-917-current-body.md` と照合）。
- 独立監査担当は実装を変更せず、登録点・source・全caller・test assertion・実行出力から再構築した。実装担当の完了判定やCI件数は要件適合の代替にしていない。
- inventory: **合計7 / 適合7 / 不適合0 / 未分類0**。INV-6の条件付き追加は実装で必要と確定し、登録・allowlist・wrapper・CLI除外を同じgroupに含めた。
- blocker: 0件。
- 判定: **PASS**。固定AC／INVARと適用可能な遷移の証拠が揃い、コード上の不適合は発見していない。
- 必須CI・残る全体検証・merge後tree照合は別ゲートであり、この記録はそれらの成功を宣言しない。

## 探索方法と入口の再構築

CodeGraphの `explore` / `node` を先に使用した。`deviceBackup.ts` の最初の指定はパス不一致だったため、symbol探索から正しい `src/lib/api/deviceBackup.ts` を取得した。文字列登録・re-export・Storybook・test fixtureの補完には、対象ディレクトリに限定した `rg` を使用した。現在のHEADと対象SHAが一致し、監査時の製品コードに未コミット差分はないことを確認した。

| 固定group | 入口 → helper → sink、逆引き結果 | 適合の根拠 |
| --- | --- | --- |
| INV-1 | `main.tsx` → `initializeDesktopLocale` → `readStoredLocale` / `getSystemLocales` / `firstSupportedLocale` → `changeDesktopLocale` → i18n・HTML・既存storageキー。mainからの起動呼出しは1か所 | App描画前に完了。import時は同梱resource登録のみで検出・cacheなし。有効保存値ならnative読取りなし。1500msのrace後は検出結果に副作用なし |
| INV-2 | `ConsentGate` → `ConsentGateView` → `LocaleSelect` → `changeDesktopLocale`。初回・更新・復元後同意は同じgate | 選択は受諾callbackを呼ばず、年齢stateを維持。保存失敗はfalseとして表示・再試行へ。pendingはdisabledと同期`acceptInFlight`で二重に遮断 |
| INV-3 | Control Centerの `openSettings('appearance')`、既存settings route → Drawer → AppearancePanel → LocaleSelect → shellの共通言語helper | section IDは不変。入口の補足・section名で言語を示し、themeより前。locale変更でstore再作成・route変更・draft消去なし |
| INV-4 | `handleAccept` → `acceptAppConsents` → `accept_app_consents` → `record_app_consents` → `save_app_consent_store`。逆引きした他のproduction callerはCLI `ClientSession::accept_consents` | Tauriはdocument検証、operation lock、終了guard、ConsentRequired、restore phaseを検査後に記録。記録後だけruntime開始／復元activation。CLIも既存のdocument・state・phase guardを保持し変更なし |
| INV-5 | backup capture / apply / snapshot / rollback。`applyPendingDeviceRestoreFrontendState` のproduction callerはAppのReady status読取りと受諾Ready応答の2か所（api.tsはre-export） | 既存キー・allowlist・適用順を維持。同意前には適用せず、activation済みmarker → apply → ack → reload。部分write／ack失敗はsnapshotへrollback |
| INV-6 | `lib.rs`のgenerate_handler登録 → startup gate → `get_system_locales` → Windows APIまたはLinux環境の限定読取り。frontend callerは`getSystemLocales`のみ | 引数・AppHandle・DesktopState不要。OS候補以外の読取り、DB／同意write／networkなし。終了中は拒否。CLI対応表は有限の`os`除外名を追加し143登録を照合 |
| INV-7 | `normalizeSupportedLocale` → `getResolvedLocale` / format helper、App、ConsentGate、Storybook。`changeLanguage` の他のproduction使用はStorybook previewのlocale適用 | 新しいnetwork listener・background検出なし。通常UIは共通helper、Storybookはtoolbar指定、test setupはen固定。全resourceを同梱しIntl利用と法務表示は既存APIを維持 |

追加・削除の内訳は、製品IPC追加1、同意前Select追加1、通常設定の共通Select利用、detectorとimport時cache削除。新しい同意sink、backupキー、background再検出は0。CLI業務commandは増やしていない。

### Sensitive sinkからの逆引き

- `kukuri.desktop.locale` のproduction writerは `changeDesktopLocale` と既存backup apply／snapshot rollback。共通helperの全callerはbootstrap（保存値経路・検出経路）、ConsentGate、DesktopShellPage、ConsentGateView Story、AppearancePanel Story。test setupとfixtureは検証用として分類した。detectorによる隠れたwriterは依存削除と初期化sourceで除去を確認した。
- `record_app_consents` のproduction callerはTauriとCLIの2か所。言語選択・検出・文書閲覧・拒否からどちらへもcall pathはない。Tauriのruntime構築・restore activationは受諾記録の後。既存の起動済みstateへの早期returnも記録後にあり、新getterから到達できない。
- 新getterの登録は `with_desktop_startup_gate` の内側。exit gateが先行し、startup state欠落時は拒否する。新規許可はこの読み取り専用commandだけで、create_post／CN policy取得／restore frontend read・ackは非Readyでhandler到達0を実IPC testが確認する。Readyのpositive controlでは同じfixture sinkへ1回到達する。
- format consumerは `i18n/format.ts` の参照検索で、AboutPanel、DesktopShellPage、presentation、PostCard、LiveSessionPanel、GameRoomPanel、MetaverseRoomControls、MetaverseRoomDiscovery、DesktopShellAuxiliaryPanels、DesktopShellPrimaryWorkspace、ColumnComposerFooter、settings fixtures、storyFixturesを列挙した。すべて既存の描画・format用途で、変更による新しい永続化／外部送信consumerはない。

## AC / INVAR と遷移の証拠

| 条件 | source・assertion・観測 |
| --- | --- |
| AC-1 / TR-1,2 | 保存値3言語優先のbootstrap table test。OSがja・navigatorがenの最初の同意h1をMutationObserverで検査する `initial-locale.spec.ts`。Windowsログの `OS_LOCALES ["ja-JP","en-US"]`、Linux `LANG=ja_JP.UTF-8` で初回ja・restartの保存enを確認。nativeは実getter＋synthetic未同意host。通常shellは同一i18nを維持するApp source、fresh browser ja起動、ja受諾→shellのbrowser flowを連結して確認した |
| AC-2 / TR-3,4 | `matchSupportedLocale` はPOSIX encoding/modifierとunderscoreを除き、未知値はnull。全候補を走査してからenへfallback。bootstrap testで混在候補、storage例外、native rejection、期限超過、明示選択後の遅着、読取り中に保存された値の優先をassert |
| AC-3 / TR-5 | LocaleSelectは3つの自称表記。`LegalDocumentView` は同一i18nの全文・metadataを使用。App testでja/zh-CN、HTML言語、年齢チェック保持、IPCがstartup statusだけであることを検査。保存失敗時のsession切替とretry、reload保存優先も成功 |
| AC-4 / TR-8 | shellChrome testは設定入口のaccessible description、appearance選択、言語がthemeより前、変更直後のfocus、draft、section、閉じた後のdraft保持をassert。route/section ID変更なし。設定画像の配置と既存browser reload testを確認 |
| AC-5 / TR-6,7 | App testはpending中disabled、追加選択無効、受諾payload ja、保存失敗後チェック保持、en再選択後retry payload、受諾回数2をassert。既存二重送信test、申告済み文書更新のageAttested=false browser testも保持 |
| AC-6 | browserの3言語×2theme×3viewportで本文scrollと同意・拒否への到達を検査。Linux localization16件、visual18件、Storybook a11y60条件で違反0。Windows native画像、Linux200% zoomのfooter focus画像を直接確認。追加のkeyboard6条件でSelect→Tab→本文→Shift+Tab→Select復帰、矢印／Home選択、focus維持、同意IPC0を確認 |
| INVAR-1 | App選択／拒否testの受諾・restore呼出し0、Rust実IPC testの非Ready protected sink到達0、DesktopState未構築、startup status不変。getter sourceの全分岐がOS読取りだけで完結 |
| INVAR-2 | 法務本文3言語、currentVersion・slug・metadata定義、年齢条件、法務描画component、同意記録helperに差分なし。UI追加が文書配列とcheckbox guardを維持。法務全文・翻訳／metadataのtest成功 |
| INVAR-3 / TR-9 | 既存storageキー文字列は不変。backup testで許可外キー未保存、activation後apply・ack、ack失敗rollbackとretry、部分write失敗rollbackを確認。App testは受諾前restore IPC 0、Ready受諾後にrestore段階を待ちshellをまだ開かないことをassert |
| INVAR-4 | browser／Storybookのnative IPC 0とnativeの引数なし1呼出しtest。OS getterのDB／network非依存と全caller、同梱resources、format test、test setup en固定を確認。localeの外部送信を新設せず、既存の明示受諾languageだけを維持 |

TR-1〜9の適用可能な分岐を確認した。401、token refresh、複数nodeのglobal applyは固定範囲で非該当であり追加していない。期限後のnative結果、保存済みcache、Ready以外のrestore早期return、失敗から明示retry、部分保存rollbackは個別に照合した。

## 確認した実行結果

監査担当が対象HEADで再実行:

```text
cd apps/desktop
npx pnpm@10.16.1 test src/i18n/bootstrap.test.ts src/i18n/index.test.ts src/i18n/format.test.ts src/i18n/parity.test.ts src/lib/api/systemLocale.test.ts src/lib/api/deviceBackup.test.ts src/components/LegalDocumentView.test.tsx --maxWorkers=1
Test Files 6 passed (6), Tests 96 passed (96), 15.83s
```

指定した `parity.test.ts` と一致する別ファイルはなく、実際の検出結果は上記6ファイル。未実行ファイルを成功件数に含めていない。`git diff --check 62b75ba 206bd4d` も成功。

実行担当の結果は以下の生ログ・assertionを直接確認した。

- `.codex/plans/issue-917-browser-red.log`: 修正前のOS優先初回表示・同意前Selectの2件失敗。nativeのfresh環境では変更前もjaであり、報告時の保存状態まで再現したとは扱わない。
- `issue-917-state-tests.log`: App／shellChrome、29件成功。
- `issue-917-browser-final.log`: Windows browser118件成功。既存同意・設定・reload回帰を含む。
- `issue-917-linux-localization.log`: 16件成功。`issue-917-linux-visual-final.log`: 18件成功。
- `issue-917-tauri-system_locale.log`: 2件、`issue-917-tauri-invoke_gate.log`: 4件、`issue-917-tauri-app_consent.log`: 4件、`issue-917-tauri-restore.log`: 2件成功。filter間の重複を合計成功数として数えない。
- `issue-917-command-parity.log`: 実登録・mapped registry・無効除外・重複等の5件成功。
- `issue-917-vitest-final.log`: 1301成功・既存DM test1件timeout。`issue-917-isolated-reruns.log` の該当3suite30件成功を確認。全体初回成功とは記録しない。
- `issue-917-rust-test.log`: command parityで1件失敗し、243件未実行。後続の単独5件成功はこの残り全体テストの代替ではない。
- `docs/ui-reviews/assets/issue-917-story-a11y.json`: 60条件、違反総数0を集計。
- `issue-917-keyboard.log` / `issue-917-keyboard.json`: 1280×800／390×844 × ja／en／zh-CNの6条件、失敗0。すべての条件でTab移動・Shift+Tab復帰・keyboard選択・focus維持がtrue、consentCallsが0であることを直接照合。
- native観測は `docs/ui-reviews/assets/issue-917-native-checks.json`、元のWindows／Linux実行ログ、隔離host source、Linux keyboard操作scriptを確認。nativeの受諾保存・network開始はこのhostでは検証していない。

直接見た画像は、UI review assetsのnative-before／native-after／native-zoom-actions／settings。日本語同意画面でheader言語欄、本文scroll、年齢理由と操作を確認した。Linuxの実200% zoomでは本文をscroll領域として保ち、footerのDecline focusが見える。設定のlight画面は言語を上、themeを下に置き、選択sectionと閉じるボタンに重なりはない。

## Findings と限界

- Existing-gap / Regression: 具体的な到達経路と利用者影響を伴う不適合は0件。
- AC-6のShift+Tab操作証拠不足は追加の6条件実操作で解消した。新しい製品要件やコード修正を追加していない。
- 全体Rustの未実行、tauri-check／永続smoke、CIは実行担当が継続する別ゲート。単独テスト成功を全体成功へ読み替えない。
- nativeは実OS読取りとproduction frontendを使う隔離hostであり、同意状態はsynthetic。初回同意から実runtimeを開始する配布バイナリ全体のE2E実行としては扱わない。変更対象のnative依存部分と同意境界は別々のnative観測／Rust／App testで確認した。
- screen reader実発話と報告時の配布版・保存状態は未確認。固定範囲にない新要件へ拡張しない。一般的なstorage全面故障、将来の追加言語・常時OS追従等も今回のblockerへ追加しない。

監査後に製品surfaceが変更された場合は、そのdeltaと影響先だけを再監査する。監査記録やvalidation結果の追記のみなら製品コードの全監査を繰り返さない。
