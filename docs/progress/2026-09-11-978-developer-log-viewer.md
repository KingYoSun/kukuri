# #978 開発者向けアプリ内ログの閲覧・書き出し

- 判定: In progress（実装・ローカル検証完了。独立監査・CI・merge の最終結果は Issue の Current status と PR を参照する）
- Scope revision: `978-2026-09-11-v1`（推奨案で承認済み: 常時 bounded capture + 読み取りだけを開発者モードで gate、backend への in-memory ミラー、手動更新、除外 kind `gui_diagnostics`）
- 基準 commit: `62d6920`
- リスク区分: C。新 IPC 2 本、ログ本文（peer / topic / Node URL / DB path 等の識別子を含み得る）の表示・clipboard・file export、shared guard（invoke gate）を通る新入口。外部送信・永続化は増やさない。
- UI 分類: 既存画面の改善（設定 > 開発者 の「ログ」節を案内文からビューアへ置換）。利用者は不調を調べる開発者モードの利用者。単一目的は「直近ログを設定内で確認し、診断レポートと同じ経路で共有できる」こと。
- 対象外: ログのファイル常時保存、自動送信、community node への送信、ログレベルの永続設定変更、起動失敗（Failed）状態での閲覧、自動更新（polling）。
- 正本: [Issue 運用手順](../runbooks/issue-lifecycle.md)、[ADR 0002](../adr/0002-feature-data-classification-template.md) → [分類](../legal/developer-log-viewer-data-classification.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[DESIGN](../../DESIGN.md) 4.2、[外部送信一覧](../legal/app-data-flow-inventory.md)、[troubleshooting runbook](../runbooks/mvp-troubleshooting.md)。

## 調査で固定した事実

- `apps/desktop/src-tauri/src/tracing.rs` は `tracing_subscriber::fmt().with_env_filter(...)` だけで、メモリ保持・IPC が無かった。`init_tracing()` は `tauri::Builder` より前に呼ばれる。
- 開発者モードは frontend の `localStorage`（`kukuri.desktop.developer-mode`）だけが知っており、backend にミラーが無い。INVAR-3 を backend で満たすには `set_os_notification_settings` と同型のミラー command が必要。
- `invoke_gate.rs` は `NON_READY_COMMAND_ALLOWLIST` 外の全 command を Ready 以外で拒否する。新 command を allowlist に入れなければ「runtime 未準備で拒否」は自動的に成立する。
- command 登録簿（`crates/kukuri-cli/command-parity.json`、144 entries）の除外 kind は `os` / `frontend_state` のみで、`tests/command_parity.rs` の `exclusion_allowed` に command 名がハードコードされている。
- 診断レポートの書き出しは `Blob` + `<a download>`、コピーは `copyTextToClipboard`。plugin 権限の追加なしで同じ経路を再利用できる。
- CodeGraph はこの環境に無く、`rg` とファイル読みで探索した。

## 変更

### backend（`apps/desktop/src-tauri`）

- `tracing.rs`: `DesktopLogBuffer`（`VecDeque` + `Mutex`、上限 2,000 件 / 合計 1 MiB / 1 行 4 KiB）と `DesktopLogBufferLayer`。`init_tracing` は既存の `fmt().with_env_filter(..).with_target(true)` を `finish()` した上に layer を重ねる（`SubscriberInitExt::try_init` で従来どおり `log` 互換も維持）。EnvFilter が subscriber 側にあるため、buffer は標準出力へ出る event だけを受ける。上限超過は古い行から落とし、1 行は char 境界で切り詰めて `…(truncated N bytes)` を付ける。`seq` は単調増加し、`snapshot(after_seq, limit)` が `oldest_seq` / `next_seq` を返す。process 全体で 1 つ（`OnceLock`）。
- `commands/developer_logs.rs`: `DeveloperLogState`（`AtomicBool` ミラー + buffer）。`set_developer_mode_enabled(enabled)` は in-memory のみで永続化しない。`read_desktop_logs(after_seq?, limit?)` はミラー false のとき `developer_mode_disabled` の `CommandError` で拒否し buffer に触れない。`limit` 0 / None は既定上限。
- `lib.rs`: state の `manage` と `generate_handler!` 登録。allowlist には入れず、gate を通す。
- `invoke_gate.rs` の既存 test 3 件に 2 command を追加（Ready 以外で拒否、終了中拒否、allowlist 外）。

### 登録簿（`crates/kukuri-cli`）

- `command-parity.json` に 2 entry を `excluded: "gui_diagnostics"` で追加、`tests/command_parity.rs` の `exclusion_allowed` に kind を追加し、件数 assertion を 146 へ。baseline / scope_revision は据え置き。

### frontend（`apps/desktop/src`）

- `lib/api/types.ts` に `DesktopLogEntry` / `DesktopLogSnapshot` と `DesktopApi.readDesktopLogs`、`lib/api/commands/developerLogsApi.ts` に `command('readDesktopLogs', ...)`（`afterSeq` / `limit` は camelCase で送る）を置き、`commands/apiModules.ts` 経由で `runtimeApi.ts` へ spread する（大型ファイル ratchet を増やさないため）。mock は `mocks/api/developerLogs.ts`（固定時刻の 12 行、長い 1 行と ERROR / WARN を含む）。
- `shell/useDeveloperModeBridge.ts`: `isTauriRuntime()` のときだけ mount 時と変更時に `set_developer_mode_enabled` を送る。`DesktopShellPage.tsx` で `useOsNotificationBridge()` の直後に呼ぶ。
- `lib/desktopLogs.ts`: `mergeDesktopLogSnapshot`（差分取得の追記、buffer から落ちた行の除去、gap 検出）、`buildDesktopLogsExport`（先頭に件数・上限・除外規則、以後 `ISO LEVEL target: message`）、`formatDesktopLogBytes`。`lib/downloadTextFile.ts` を新設し、`ReleasePanel.tsx` の診断レポート書き出しも同じ helper を使う（挙動は同一）。
- `shell/useDesktopLogs.ts`: enabled のあいだだけ表示時に 1 回取得し、`refresh()` で `afterSeq = 最後の seq` の差分取得。polling なし。無効化・unmount 後の応答は世代番号で捨てる。
- `components/settings/DeveloperLogViewer.tsx`: 一覧（`role="region"`、最大高さ 18rem で内部 scroll、`overflow-wrap: anywhere`、level は文字 + 色）、「ログを更新」「ログをコピー」「ログを書き出す」（`kukuri-logs.txt`）、loading / empty / error（`role="alert"` + 更新で再試行）/ 上限到達 / gap の各表示、共有前の注意、標準出力の案内、runbook link。コピー／書き出しの結果は `aria-live="polite"`。
- `DeveloperPanel.tsx` は `logs` slot を受け、ON 時だけ描画する。`shell/page/DesktopShellDeveloperLogs.tsx` が hook と viewer を束ね、`DesktopShellSettingsDrawer.tsx` が `developerModeEnabled` のときだけ渡す（OFF 時は unmount され取得も起きない）。
- locale 3 言語の `developer.logs.*` を viewer 用に置換（Windows で取得不能という文言を削除）。Storybook に `DeveloperLogViewer`（Ready / Loading / Empty / LimitReached / ReadError / Narrow）を追加し、`DeveloperPanel` の Enabled / Narrow story に fixture を渡す。

### 文書

- `docs/legal/developer-log-viewer-data-classification.md`（ADR 0002 の 11 項目）、`docs/legal/app-data-flow-inventory.md` に「アプリ内ログ（開発者モード）」行、`docs/README.md` Legal に参照、`docs/runbooks/mvp-troubleshooting.md` の「ログの確認」を置換（Windows はアプリ内から書き出す、起動失敗時は読めない）。

## 修正前の再現

- 基準 commit では `tracing.rs` に buffer が無く、`read_desktop_logs` / `set_developer_mode_enabled` が存在しない。`SettingsPanels.test.tsx` の「ログ所在を説明する」test（#962）は「専用ビューアは無い」文言を検証していたため、本 Issue でビューア検証（`developer panel links to the diagnostic report and shows the log viewer only while enabled` ほか 2 件）へ置き換えた。
- 新規 test を基準 commit に足すと、`tracing.rs` の 6 件（型が無い）、`developer_logs.rs` の 2 件（module が無い）、`command_parity` の件数 assertion、`DesktopShellPage.developerMode.test.tsx` の `developer log viewer reads backend logs only while the mode is on`（`readDesktopLogs` が無い）が失敗する。実装後はすべて成功した。

## AC / INVAR の証跡

| 条件 | 実装・test / evidence |
| --- | --- |
| AC-1 | `DeveloperPanel` の `logs` slot は ON 時だけ描画、`DesktopShellSettingsDrawer` は ON 時だけ `DesktopShellDeveloperLogs` を渡す。`developer log viewer reads backend logs only while the mode is on`（OFF で `readDesktopLogs` 0 回、ON で 1 回、更新で 2 回、OFF に戻して増えない）、`developer panel links to the diagnostic report and shows the log viewer only while enabled`、browser `developer log viewer lists backend lines and exports only on request at 1280 / 390` |
| AC-2 | `DeveloperLogViewer` の copy / export は click handler だけから `copyTextToClipboard` / `downloadTextFile('kukuri-logs.txt')` を呼ぶ。`developer log viewer copies and exports the shown lines only on request`（render 直後は 0 回、click 後に本文・件名を確認）、`downloadTextFile` unit test、browser spec の `download` event（`suggestedFilename === 'kukuri-logs.txt'`） |
| AC-3 | `LOG_BUFFER_MAX_ENTRIES` / `LOG_BUFFER_MAX_BYTES` / `LOG_LINE_MAX_BYTES` 固定。`entry_limit_drops_oldest_lines_and_keeps_sequence`、`byte_limit_drops_several_old_lines_before_entry_limit`、`long_line_is_truncated_on_a_char_boundary_before_it_is_stored`、`production_limits_are_fixed`（上限の 2 倍 push 後も件数・byte が上限内） |
| INVAR-1 | ログへ秘密値を出さない既存契約は変更なし（`crates/*` に差分なし）。viewer は event をそのまま表示するだけで新たな情報源を持たない。書き出し本文の先頭に除外規則を明記（`buildDesktopLogsExport` test） |
| INVAR-2 | `resolve_tracing_directives` と既存 2 test は無変更。fmt subscriber の設定は据え置きで layer を外側に重ねるだけ。`layer_receives_only_events_that_pass_the_stdout_filter`（debug と対象外 target は buffer にも入らない） |
| INVAR-3 | `read_is_rejected_until_developer_mode_is_mirrored_and_limit_is_clamped`、`ipc_round_trip_rejects_reads_while_mirror_is_off_and_serves_them_after_mirror`（Initializing で両 command 拒否・ミラー不変 → Ready でミラー OFF は `developer_mode_disabled` → ON で取得 → OFF で再拒否）、`invoke_gate.rs` 3 test、`command_parity` 5 test |

## Surface inventory（追加 INV-1〜6。既存入口の分類変更なし。未分類 0）

| ID | 入口・trigger | helper / owner | 読み書き・副作用 | guard | TR |
| --- | --- | --- | --- | --- | --- |
| INV-1 | `tracing` の全 event（app 内の `info!` / `warn!` 等、依存 crate を含む） | fmt subscriber の EnvFilter → `DesktopLogBufferLayer::on_event` | in-memory ring buffer への push のみ | 上限固定、lock 毒化時は捨てる | TR-1 |
| INV-2 | `set_developer_mode_enabled`（`useDeveloperModeBridge`: shell mount 時と `developerModeEnabled` 変更時） | invoke gate → `DeveloperLogState::set_developer_mode` | `AtomicBool` 更新のみ | Ready 以外・終了中は gate が拒否 | TR-2, TR-6 |
| INV-3 | `read_desktop_logs`（`useDesktopLogs`: viewer mount 時と「ログを更新」） | invoke gate → `DeveloperLogState::read` → `DesktopLogBuffer::snapshot` | buffer の読み取りのみ | Ready 以外は gate、ミラー false は `developer_mode_disabled` | TR-3, TR-4 |
| INV-4 | 「ログをコピー」「ログを書き出す」 | `copyTextToClipboard` / `downloadTextFile` | clipboard / 利用者が選ぶ保存先 | click のみ。view が無ければ disabled | TR-5 |
| INV-5 | `ReleasePanel` の診断レポート「書き出し」（helper 共有化） | `downloadTextFile('kukuri-diagnostics.txt', …)` | 従来どおり | 従来どおり | 既存 |
| INV-6 | mock（Storybook / Playwright / Vitest） | `mocks/api/developerLogs.ts` | in-memory fixture | なし | TR-3, TR-5 |

sensitive sink の逆引き: `DesktopLogBuffer::snapshot` の caller は `DeveloperLogState::read` のみ、`DeveloperLogState::read` の caller は `read_desktop_logs` のみ（`rg`）。`set_developer_mode` の caller は `set_developer_mode_enabled` と test のみ。`invokeDesktop('set_developer_mode_enabled'` の caller は bridge のみ、`readDesktopLogs` の production caller は `useDesktopLogs` のみ。`downloadTextFile` の caller は `ReleasePanel` と `DeveloperLogViewer`。新規の network / DB / file 書込み sink は 0。

## 状態遷移

| ID | 事前状態 / sequence | 期待状態 | 禁止する副作用 | 検証 |
| --- | --- | --- | --- | --- |
| TR-1 | 起動 → 長時間稼働（上限超過） | 古い行が落ち、件数・byte が上限内。標準出力は不変 | 無限増加、filter の変化 | Rust unit 4 件 |
| TR-2 | Initializing / ConsentRequired / Failed / 終了中で 2 command | gate が拒否、buffer・ミラー不変 | runtime 構築、state 変化 | `invoke_gate.rs`、IPC round-trip |
| TR-3 | Ready、OFF → ON → 開発者 section 表示 → 更新 | ミラー true → 1 回取得 → 更新で差分取得 | OFF 時の取得、polling | shell test、browser spec |
| TR-4 | Ready、ON → OFF | viewer が消え、backend も拒否 | stale 表示、OFF 後の取得 | shell test、command test |
| TR-5 | ON、コピー / 書き出し | clipboard / `kukuri-logs.txt` に表示中の行 + 除外規則 | 自動送信・自動保存 | Vitest、browser download event |
| TR-6 | アカウント切替 / 再 mount / 再起動 | bridge が再ミラー、buffer は process 内で継続 | ミラー消失による誤拒否 | bridge test（mount 時に送信） |
| TR-7 | 取得失敗（gate 拒否など） | 既存表示を保持し、error と再試行を出す | 表示の消失 | viewer state test（`ReadError` story） |
| TR-8 | 前回更新後に上限を超えた | 全件で置き換え、gap と上限到達を表示 | 差分の誤追記 | `mergeDesktopLogSnapshot` test |

## 検証条件と結果

| 検証 | 条件 / 結果 |
| --- | --- |
| `cargo xtask doctor` | Linux（remote）、成功 |
| Tauri backend unit（`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib -- tracing:: developer_logs:: invoke_gate::`） | 本文の「検証ログ」参照 |
| `cargo test -p kukuri-cli --test command_parity` | 5 件成功 |
| `cargo xtask check`（`cargo fmt --check`、workspace clippy、`tauri-check`、`desktop-lint`） | 成功 |
| `cargo xtask oversized-files` | 初回 CI で `runtimeApi.ts` が 1042 → 1047 行に増えて失敗。`readDesktopLogs` を `commands/developerLogsApi.ts` へ移し `commands/apiModules.ts` で分割 module を 1 行で re-export して 1041 行に戻し、成功（commit `b0cd97b`） |
| targeted Vitest（desktopLogs、downloadTextFile、useDeveloperModeBridge、SettingsPanels、DesktopShellPage.developerMode、i18n/parity） | 成功。`parity.test.ts` は初回 CI で ja の `developer.logs.share` が禁止語 `Node` を含み失敗。「コミュニティノードの URL」へ言い換えて成功（commit `8323266`） |
| Playwright `developer-mode.spec.ts`（chromium、`PLAYWRIGHT_BROWSERS_PATH` に pin build 1234 → 環境の 1194 への symlink を置いて実行） | 16 件成功（既存 14 + 新規 2） |
| Storybook build | 成功 |
| Tauri crate clippy（`cargo clippy --all-targets -- -D warnings`、CI 対象外） | 本 PR の新規 file に指摘なし。既存 file（`background_notifications.rs`、`file_dialog.rs`、`restore_lifecycle.rs`、`app_update_tests.rs`）に既存の指摘 5 件があり、本 PR では触れない（Optional-hardening） |
| 全体 Vitest / `cargo xtask rust-test` / `cargo xtask e2e-smoke` | 下記「全体検証」参照 |

## UI 証跡と確認の限界

- 対象 platform / state: browser（Linux Chromium、mock）、1280 / 390px、dark、en（browser spec）、ja / en / zh-CN（Vitest は en、locale parity は 3 言語同期）、開発者 OFF / ON、ready / loading / empty / limit reached / error（Storybook）。
- Accessibility: 一覧は `role="region"` + `aria-label`（件数入り）+ `tabIndex=0` で keyboard scroll 可、level は文字と色の併用、エラーは `role="alert"`、コピー／書き出し結果は `aria-live="polite"`。screen reader の音声聴取は未実施。
- 未確認: Tauri / WebView 実機（Linux WebKitGTK、Windows WebView2）での表示・保存ダイアログ・実 backend からの取得。remote 環境では実行できないため、browser mock の成功で置き換えない。Windows での取得可否は backend の IPC test と `windows_subsystem` に依存しない実装から判断し、実機で再確認していない。
- 視覚 baseline: `visual.spec.ts` の開発者画面は `developer-enabled-ja-dark.png`（1280 幅）と `developer-enabled-en-light.png`（390 幅）の 2 枚。「Kukuri Visual Baseline」workflow（Linux / Chromium、`fonts-noto-cjk`、run 34655685231）で再生成し、ログ節が drawer の可視範囲に入る ja-dark だけが変わった（commit `f037757`）。en-light は 390 幅で節が可視範囲外のため変更なし。
- 性能: 最大 2,000 行を `<li>` で描画する。表示時と更新時だけ取得し、timer / 購読は無い。mock 12 行で計測は非該当。実 2,000 行の描画は 18rem の内部 scroll に収まり、仮想化は Optional-hardening とする。
