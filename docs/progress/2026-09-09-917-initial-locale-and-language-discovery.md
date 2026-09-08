# #917 初回表示の言語決定と言語設定の発見性

## 現在判定

- Issue: [#917](https://github.com/KingYoSun/kukuri/issues/917)
- Scope revision: `917-r1`（2026-09-09承認・固定）、リスク区分C。
- 調査基準commit: `62b75baeeb40c62d9ee4afedee4e584b4f20c14a`。
- 判定: 実装と[独立監査](2026-09-09-917-independent-audit.md)はPASS。最終CI・mergeの現在判定は [PR #949](https://github.com/KingYoSun/kukuri/pull/949) に集約する。
- 詳細な固定AC／INVARはIssue本文。本書は実装・判断・証跡の記録であり、要件を別管理しない。

## 問題と変更

従来のi18n初期化はlocalStorage→navigatorの順で言語を検出し、自動検出結果も `kukuri.desktop.locale` へ保存していた。OSのUI言語を直接読む入口はなく、同意画面に言語選択がなかった。設定の言語Selectはthemeの後に置かれ、Control Centerでは「設定」とだけ表示していた。

変更後は有効な保存値→OS候補→navigator候補→英語の順で決め、最初のApp描画前に確定する。候補の対応判定と英語fallbackを分け、先頭が未対応でも後続の日本語等を選べる。同意headerに3言語の自称表記によるSelectを置き、全文・正文／参考訳案内・HTMLの言語を同期する。通常設定は「表示と言語」（英語は `Language & theme`）とし、Selectをthemeの前に置いた。既存のappearance ID・deep linkを維持する。

同意保存中は言語を固定し、明示受諾時の表示言語を従来のIPCへ渡す。言語の保存失敗時にもsession内の表示は維持し、同じ選択を保存し直すボタンを提示する。同意・年齢条件、文書版・本文、runtime開始条件、backupのlocaleキーと適用段階は変更していない。

## 修正前の再現と判断

- 修正前のPlaywrightで「fresh、日本語OS候補、英語navigator」と「同意前に言語選択」の2件が失敗した。OS候補を与えても英語の同意表示、Selectは存在しなかった。同じtestは変更後に成功する。
- 現行基準commitを使う隔離native hostでは、Windowsのnavigatorは `ja,en,en-GB,en-US`、Linuxの `LANG=ja_JP.UTF-8` では `ja-JP` であり、freshの日本語表示自体は両方で成功した。報告時の配布版・保存値・起動環境は確定できないため、報告のOS条件だけで常に再現したとは扱わない。
- 保存値だけから過去の自動検出と明示選択は区別できない。既存の英語を推測で消すmigrationは行わず、同意前の選択で回復できるようにした。nativeでも日本語OSのまま英語を選び、restart後に英語が維持されることを確認した。
- OSのUI言語をnavigatorの値から推測せず優先する契約のため、引数なしの `get_system_locales` を追加した。OSとnavigatorが一致する実機だけで、すべてのWebView設定が一致すると推定しない。

## データと実装境界

Windowsは既存 `windows` crateの `Win32_Globalization` featureを有効にし、`GetUserPreferredUILanguages(MUI_LANGUAGE_NAME)` で順序付きの言語名を読む。長さ取得後にUTF-16 bufferを確保し、取得失敗は空候補とする。[Microsoft API仕様](https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getuserpreferreduilanguages)

Linuxは `LANGUAGE` の候補順と、`LC_ALL → LC_MESSAGES → LANG` の有効なmessage localeを使う。空値を無視し、C／POSIXではLANGUAGEを使わず英語を返す。processの環境を変更しない。[GNU gettextの環境変数](https://www.gnu.org/software/gettext/manual/html_node/Locale-Environment-Variables.html)、[LANGUAGEの扱い](https://www.gnu.org/software/gettext/manual/html_node/The-LANGUAGE-variable.html)

getterはAppHandle／DesktopStateを受け取らず、OS言語以外の環境値、DB、同意ファイル、runtime、networkへ依存しない。非Ready allowlistへの追加はこのcommandだけで、終了中の拒否と既存の禁止commandを維持する。browser／StorybookではIPCを呼ばない。OS取得は1回・上限1500msで、期限後の結果には保存・描画の副作用を持たせない。

`i18next-browser-languagedetector` とimport時の検出・自動cacheを外し、同梱resourceの登録と起動時の選択を分離した。明示選択と起動時の決定は `changeDesktopLocale` を使う。保存形式は既存の3値と同じキーで、復元後のfrontend state適用・ack・rollbackは既存実装を維持する。

正本への反映: [DESIGN](../../DESIGN.md#43-アプリ初回同意)、[データ分類](../legal/app-consent-data-classification.md#初回表示と言語選択917)、[UI実装配置](../architecture/desktop-ui-implementation.md#表示言語の配置)、[quickstart](../runbooks/mvp-user-quickstart.md)、[troubleshooting](../runbooks/mvp-troubleshooting.md)。

## AC／INVARの証跡

| 条件 | 実装と検証 |
| --- | --- |
| AC-1 保存値優先・日本語OSの初回表示 | `i18n/bootstrap.ts`、`commands/system_locale.rs`。`bootstrap.test.ts`、`initial-locale.spec.ts` の最初の同意heading観測、Windows／Linux nativeの初回・restart |
| AC-2 地域表記・後続候補・fallback・期限超過 | `i18n/locale.ts` とbootstrap。21件の初期化testに、POSIX表記、未知先頭、OS失敗、storage例外、遅着、待機中の明示設定を含む。Rustの環境優先順位test |
| AC-3 同意前の選択・同期・保存失敗 | `LocaleSelect`、`App::ConsentGate`。App testの言語変更・保存失敗と再試行、fresh／reload browser、両nativeの3言語切替 |
| AC-4 設定の発見性・状態保持 | `AppearancePanel`、Control Center、設定drawer。shellChrome testで言語がthemeより先、設定入口のdescription、focus、workspaceのdraft保持を検証。visualにも表示名と順序のassertionを追加 |
| AC-5 同意pending・送信言語・retry | App testでpending中のSelect無効、表示言語を送るpayload、失敗後の言語変更とretry、年齢チェック保持。既存再同意browser testも維持 |
| AC-6 locale／theme／入力／layout | 同意browser条件、localization-layout、Linux visual、Storybook a11y60条件、nativeのpointer／keyboard・実200%zoom。画像は[UI review](../ui-reviews/2026-09-09-initial-locale-and-language-discovery.md) |
| INVAR-1 言語操作だけでは同意・runtime・networkを開始しない | App testで同意IPC0回、Tauri dispatch testで非Ready3状態のlocale成功と禁止handler到達0・DesktopState不在・起動状態不変を検証。Readyでhandlerに到達するpositive controlも置く |
| INVAR-2 文書・年齢・二重送信の契約維持 | `LegalDocumentView.test.tsx`、既存App同意・再同意test、Tauri `app_consent` filter。法務本文JSON・版・consent payload形状のdiffなし |
| INVAR-3 保存・復元互換 | 既存 `deviceBackup.test.ts` とAppのready後apply testを確認。旧キー・3言語を維持し、復元実装の変更なし。nativeで日本語OSに保存済み英語を復元してrestart |
| INVAR-4 local-only・既存consumer | systemLocale wrapper testでbrowser／mockのIPC0。getterの依存逆引き、index／format／parity、AboutPanel共有表示、Storybookを確認 |

## 固定surfaceの結果

| ID | 入口 → helper → sink | 変更／分類・証跡 |
| --- | --- | --- |
| INV-1 | main cold start／reload → bootstrap → OS／navigator／locale storage／DOM | 検出順と描画前確定を変更。TR-1〜4、初期化・browser・native |
| INV-2 | fresh／再同意／復元後ConsentGateの選択 → changeDesktopLocale → locale storage／DOM | 選択入口1つ追加。TR-5〜7、App・Story・browser |
| INV-3 | Control Center／appearance deep link／設定Select → 同じ適用helper | 既存導線の説明・表示名・順序を変更。TR-5,8、shellChrome・localization |
| INV-4 | 明示受諾 → acceptAppConsents → accept_app_consents → record_app_consents | 既存の同意・runtime sinkを維持。TR-6,7、payload・0回・Rust同意test |
| INV-5 | backup capture／activation後apply／ack／rollback → portable frontend state | 変更なしの互換確認。TR-9、deviceBackup・App restore |
| INV-6 | get_system_locales → OS専用read、startup／exit gate | 新規のread-only command1つ。TR-2〜4、非Readyで他sinkのhandler到達0、getter test |
| INV-7 | normalizeSupportedLocale／resource consumer、AppのHTML言語同期、Storybook／test setup | formatのen fallback、3resource、共有文書を維持。TR-5,8、parity・既存consumer回帰 |

機械的な照合はCodeGraphを先に使い、`normalizeSupportedLocale`、`DESKTOP_LOCALE_STORAGE_KEY`、`changeLanguage`、`documentElement.lang`、`acceptAppConsents`、`record_app_consents`、`applyPendingDeviceRestoreFrontendState` とcommand登録を限定rgで補完した。新規の言語writerはbootstrap／明示選択の共通helperだけ。backupのwriterは既存のまま。生産用IPC追加1、同意保存sink追加0、独自のbackground再検出0。

全体Rust検証の `baseline_inventory_is_classified_once` が新commandの未分類を検出したため、`crates/kukuri-cli/command-parity.json` にGUI専用の `os` として追加し、有限の除外名と143入口の照合を更新した（対応表revision `2026-09-09-917-initial-locale-r1`）。これはINV-6の登録同期であり、CLIの業務command追加や全体の除外許可ではない。

## 検証状況

- 修正前browser2件失敗→同意関連browser28件成功。追加の保存値優先・browser単独起動もLinux browserで確認。
- Linux browserは118件中117件が成功し、英語の長い設定名だけが既存の折返し禁止testで失敗した。名前を `Language & theme` に短縮し、全3言語のlocalization16件が成功した。
- Linux visual18件を確認。意図した同意2枚・設定3枚を更新。既存の比較許容差内だと旧画像が残るため、設定3枚は `--update-snapshots=all` で実際の新しい配置を固定した。画像更新だけでなく表示名・言語とthemeの順序をassertする。
- Storybook build成功、同意8stateと表示設定2state × 3locale × 2themeのa11y60条件で違反0。
- Windows browserの最終全体実行は118件成功。言語Select→Tab→本文領域→Shift+Tab→Selectへ戻り、矢印／Homeで選択してfocusを維持する6条件も成功し、同意IPCは0回。[keyboard証跡](../ui-reviews/assets/issue-917-keyboard.json)
- Tauri生成unit exeで `system_locale` 2件、`invoke_gate` 4件、`app_consent` 4件、`restore` 2件成功（filter間の重複あり）。Windowsの生成test／隔離host exeに必要なCommon Controls v6 manifestを検証環境だけで付加した。製品のtest assertionや同意guardは変更していない。
- 全Vitestの初回は1302件中1300件成功、既存media／route testの2件がtimeout。worker数を4にした全体再実行ではその2件を含む1301件が成功し、別の既存DM refresh testだけがtimeout。timeoutした3suiteをworker数1で個別再実行し30件すべて成功した。timeout値やassertionは緩めていない。
- Rust check成功。最初のRust testはWindowsのlibrary探索でvcruntime.libが見つからず実行前に失敗し、MSVCとWindows SDKのLIBを明示した再実行へ進めた。
- GUI／CLI対応表の同期後、`command_parity` 5件と全Rust test（887件、harness22件、doctest）が成功。CIのRust整形チェックで対応表testのassert_eq折返しが検出され、rustfmtで補正した。
- `cargo xtask tauri-check`、`cargo xtask e2e-smoke`（post永続往復6step）、`cargo fmt --all --check`、`cargo xtask oversized-files`、`git diff --check` が成功。Windowsのvisual smoke18件も成功し、pixel比較の証拠は別のLinux実行で確認した。
- 最初のCIは日本語の狭幅snapshotだけで文字が「□」になった。CJK fontがないCI画像をbaselineには採用せず、`kukuri-fast.yml` のbrowser jobと `kukuri-visual-baseline.yml` の生成jobに `fonts-noto-cjk` を追加した。Noto Sans CJK JPに揃えた隔離Linuxで対象1枚を再生成し、visual18件が成功した。比較許容差は変更していない。workflowはactionlint成功。

## Native確認と限界

実際のproduction frontendと同じOS getterを使う隔離Tauri hostで確認した。起動状態はsyntheticな未同意に固定し、実利用者の同意・年齢申告・runtime／network開始は行っていない。禁止副作用は別のRust／App境界testで確認する。

Windows WebView2とLinux WebKitGTK2.50.4（Ubuntu22.04、Xvfb）で初回日本語、3言語切替、保存値とHTML言語の一致、restartでの英語優先を確認。LinuxはTauriの `set_zoom(2.0)` による実200%zoomでもviewport640×400、横幅628でoverflowなし、Tabでfooterの操作に到達した。[nativeの観測JSON](../ui-reviews/assets/issue-917-native-checks.json)

実発話のscreen reader、報告時の配布版と保存状態そのものは未確認。自動a11y／nativeの成功をこれらの確認済み証拠とはしない。

## 次の工程

固定headの独立監査はPASS。整形・CI font・記録差分を再監査し、必須CIと監査PASSの後にmergeする。merge treeと監査対象の一致を確認してからIssueをCloseする。差分監査／CI／mergeの現在判定はPRとIssueに集約する。
