# #978 独立監査記録（PR #986 head `8323266`）

実装者の結論と [progress 記録](2026-09-11-978-developer-log-viewer.md) を前提にせず、別コンテキストの監査者が固定 AC / INVAR と対象 commit の差分から再構築した。手順は [Issue lifecycle runbook](../runbooks/issue-lifecycle.md) の「5. 独立監査」に従う。ビルド・テストの再実行は行わず（他検証と並行のため）、コード・test・登録簿の読解、`rg` / `git` による逆引き、および tracing-subscriber 0.3.23 の source 読解で監査した。test の実行結果は実装者環境の結果を証拠として引用する。

- 対象 commit: `8323266`（基準 `62d6920` からの差分 40 file。source 変更は `apps/desktop/src-tauri` 5 file、`apps/desktop/src` 20 file、`crates/kukuri-cli` 2 file、docs 5 file、視覚 baseline 1 file。`crates/kukuri-app-api` ほか業務 crate に差分なし）
- Scope revision: `978-2026-09-11-v1`
- リスク区分: C
- inventory: 合計 14 / 適合 14 / 不適合 0 / 未分類 0
- 判定: **PASS**
- 監査日: 2026-09-11

## inventory（監査者が登録点から再生成）

登録点: `lib.rs` の `generate_handler!`（146 entry、うち新規 2）、`invoke_gate.rs`、`command-parity.json`（146 entry）、`DesktopApi.readDesktopLogs`（`types.ts` → `developerLogsApi.ts` → `apiModules.ts` → `runtimeApi.ts` spread）、`useDeveloperModeBridge` / `useDesktopLogs` / `DesktopShellDeveloperLogs` / `DesktopShellSettingsDrawer` / `DeveloperPanel` / `DeveloperLogViewer` / `desktopLogs.ts` / `downloadTextFile.ts` / `mocks/api/developerLogs.ts` / stories。

| ID | 入口・trigger | helper | sink / 副作用 | guard | TR | 判定 |
| --- | --- | --- | --- | --- | --- | --- |
| E1 | process 内の全 `tracing` event（app crate・依存 crate・`log` crate 経由を含む） | fmt subscriber（EnvFilter）→ `Layered::event` → `DesktopLogBufferLayer::on_event` → `LineVisitor` → `DesktopLogBuffer::push` | in-memory `VecDeque` への push。file / network / DB なし | EnvFilter が `Layered::enabled` / `event_enabled` で先に評価され、標準出力へ出ない event は layer に届かない。上限 2,000 件 / 1 MiB / 1 行 4 KiB、lock 毒化時は捨てる | TR-1, TR-9 | 適合 |
| E2 | shell mount 時の mirror（`DesktopShellPage` → `useDeveloperModeBridge(developerModeEnabled)`、`App.tsx` は startup `ready` のときだけ shell を mount） | `invokeDesktop('set_developer_mode_enabled')` → invoke gate → `DeveloperLogState::set_developer_mode` | `AtomicBool` の更新のみ。永続化なし | `isTauriRuntime()` のときだけ送信。gate: Ready 以外・終了中は拒否。失敗は `.catch(() => {})` で握り、次の変更で再送 | TR-2, TR-6 | 適合 |
| E3 | 開発者モード checkbox（`DesktopShellSettingsDrawer.onDeveloperModeChange` → store + `writeDeveloperMode` → bridge effect の `[enabled]` 変更） | E2 と同じ | 同上 | 同上 | TR-3, TR-4 | 適合 |
| E4 | viewer mount 時の取得（開発者 section が active かつ ON のときだけ `DesktopShellDeveloperLogs` が mount → `useDesktopLogs(api, true)` の effect） | `api.readDesktopLogs(null, null)` → `developerLogsApi` → `invokeDesktop('read_desktop_logs')` → gate → `DeveloperLogState::read` → `DesktopLogBuffer::snapshot` | buffer の読み取りのみ | 二重 guard: drawer の `developerModeEnabled ? … : null` と `DeveloperPanel` の `developerModeEnabled ? … : null`。`SettingsDrawer` は非 keepMounted section を active 時だけ描画。backend: gate + mirror false で `developer_mode_disabled`（buffer に触れない） | TR-3, TR-5 | 適合 |
| E5 | 「ログを更新」click（`onRefresh` → `logs.refresh()`、`afterSeq = 最後の seq`） | E4 と同じ。世代番号 `requestRef` で無効化後の応答を破棄 | 同上 | `busy` 中は disabled。polling / timer / 購読なし | TR-5, TR-7, TR-8 | 適合 |
| E6 | 「ログをコピー」click（`copyLogs`） | `buildDesktopLogsExport(view)` → `copyTextToClipboard` | clipboard write + `CLIPBOARD_COPY_EVENT` dispatch（既存 helper の挙動） | click handler のみ。`view` null で disabled | TR-10 | 適合 |
| E7 | 「ログを書き出す」click（`exportLogs`） | `buildDesktopLogsExport(view)` → `downloadTextFile('kukuri-logs.txt', …)` | Blob + `<a download>`（利用者が保存先を選ぶ既存経路） | click handler のみ。`view` null で disabled | TR-10 | 適合 |
| E8 | `ReleasePanel.exportDiagnosticReport`（helper 共有化のみ） | `downloadTextFile('kukuri-diagnostics.txt', …)` | 従来と同一の Blob + `<a download>` 手順（inline 実装を関数へ移しただけ。差分で確認） | 従来どおり click のみ | 既存 | 適合 |
| E9 | `createDesktopMockApi` の `readDesktopLogs`（browser / Playwright / Vitest） | `developerLogFixtureSnapshot(afterSeq, limit)` | in-memory fixture 12 行 | なし（mock） | TR-3, TR-5 | 適合 |
| E10 | Storybook `DeveloperLogViewer.stories` / `DeveloperPanel.stories` | 固定 view を props で渡す。`onRefresh` は no-op | なし（IPC 不可到達） | なし | — | 適合 |
| E11 | startup 遷移（Initializing / ConsentRequired / Failed → Ready、device restore 中の Initializing→Ready、app consent の再判定） | `with_desktop_startup_gate` | 両 command は `NON_READY_COMMAND_ALLOWLIST` に無く Ready 以外で拒否。Failed では `App.tsx` が shell を mount せず frontend 側からも到達しない | gate | TR-2 | 適合 |
| E12 | 終了中（`DesktopLifecycle.requested()`） | `command_allowed_during_exit` | 両 command 拒否（allow は `cancel_device_backup` / `get_desktop_startup_status` のみ） | gate | TR-2 | 適合 |
| E13 | アカウント切替 / 端末復元後（`AccountKeyPanel.handleSwitch` → `window.location.reload()`、`DeviceBackupPanel` も reload） | shell 再 mount → E2 | backend の mirror と buffer は process 内で継続 | E2 と同じ | TR-6 | 適合 |
| E14 | lock 毒化（`Mutex` poisoned）時の push / snapshot | `push`: `let Ok(..) else { return }` で行を捨てる。`snapshot`: `into_inner()` で読む | 標準出力側には影響しない（layer は fmt の外側で、`Layered::event` は inner を先に呼ぶ） | — | TR-9 | 適合 |

group の列挙方法: 新規 command は `generate_handler!` の 2 行（`commands::developer_logs::*`）、frontend の IPC 呼出は `rg "set_developer_mode_enabled|read_desktop_logs" apps/desktop/src` で production caller が各 1 箇所。

## sensitive sink の逆引き（`rg`、test / stories を除く）

| sink | caller | 備考 |
| --- | --- | --- |
| `DesktopLogBuffer::push` | `DesktopLogBufferLayer::on_event` のみ | layer は `init_tracing` で 1 回だけ組む。`desktop_log_buffer()` は `OnceLock` で process に 1 つ |
| `DesktopLogBuffer::snapshot` | `DeveloperLogState::read` のみ | `read` は mirror false のとき snapshot より前に `Err` を返す |
| `DeveloperLogState::read` / `set_developer_mode` | `read_desktop_logs` / `set_developer_mode_enabled` のみ（両者 `fn` は private） | `DeveloperLogState` を `manage` するのは `lib.rs` の 1 箇所 |
| `invokeDesktop('set_developer_mode_enabled')` | `useDeveloperModeBridge` のみ | `DesktopShellPage` から 1 回呼ぶ |
| `invokeDesktop('read_desktop_logs')` | `developerLogsApi.readDesktopLogs` のみ | `api.readDesktopLogs` の production caller は `useDesktopLogs` のみ |
| `useDesktopLogs` | `DesktopShellDeveloperLogs` のみ | 呼出元は drawer の `developerModeEnabled` 三項のみ |
| `copyTextToClipboard`（viewer 内） | `DeveloperLogViewer.copyLogs`（click） | 他 13 箇所の既存 caller は差分なし |
| `downloadTextFile` | `DeveloperLogViewer.exportLogs`、`ReleasePanel.exportDiagnosticReport` | 新規 file / network sink は 0 |

## 状態遷移の照合

| ID | 事前状態 / sequence | 期待状態 | 根拠 / test | 判定 |
| --- | --- | --- | --- | --- |
| TR-1 | 起動 → 長時間稼働で件数・byte 上限超過 | 古い行から落ち、`len <= 2000 && bytes <= 1 MiB` を保つ。新しい行は残る | `push` は push 後に `while len > max_entries \|\| bytes > max_bytes { pop_front }`。`entry_limit_drops_oldest_lines_and_keeps_sequence`、`byte_limit_drops_several_old_lines_before_entry_limit`、`production_limits_are_fixed`（上限の 2 倍 push、各行 5 KB） | 適合 |
| TR-2 | Initializing / ConsentRequired / Failed / 終了中で 2 command | gate が拒否し、mirror と buffer は不変 | `invoke_gate.rs` の 3 test に 2 command 追加、`ipc_round_trip_…`（Initializing で両 command `is_err` かつ `developer_mode_enabled() == false`）。Failed は `App.tsx` が shell を mount しないため frontend からも不達 | 適合 |
| TR-3 | Ready、OFF → 開発者 section 表示 → ON | OFF では viewer 不在・IPC 0 回。ON で mirror 送信 → viewer mount → 1 回取得 | `developer log viewer reads backend logs only while the mode is on`（`readDesktopLogs` 0 回 → 1 回 `(null, null)`）、bridge test（変更時に `{enabled: true}`）、Playwright 1280 / 390 | 適合 |
| TR-4 | Ready、ON → OFF | viewer unmount、backend も `developer_mode_disabled` で拒否、その後 IPC 増えない | 同 shell test（OFF 後も 2 回のまま）、`read_is_rejected_until_…`（OFF に戻して再拒否）、`useDesktopLogs` cleanup で `requestRef` を進め in-flight 応答を破棄 | 適合 |
| TR-5 | ON、「更新」 | `afterSeq = 最後の seq` の差分取得、連続なら追記 | shell test（2 回目は `(12, null)`）、`mergeDesktopLogSnapshot` 4 test | 適合 |
| TR-6 | アカウント切替 / 復元 / 再起動 → 再 mount | bridge が mount 時に再送、buffer は process 継続中は保持 | `handleSwitch` → `location.reload()`、bridge test（mount 時に `{enabled: false}` を送る） | 適合 |
| TR-7 | 取得失敗（gate 拒否・mirror false・終了中） | 既存 view を保持し `role="alert"` + 更新で再試行 | `useDesktopLogs` catch で `view: viewRef.current`、viewer state test（error で Refresh enabled） | 適合 |
| TR-8 | 前回更新後に buffer が一巡 | 全件で置換し gap / dropped を表示 | `refresh after the buffer wrapped …`、viewer state test | 適合 |
| TR-9 | lock 毒化 | 標準出力は継続、buffer 行は捨てる / 読める | E14（コード確認。test なし、到達には lock 保持中の panic が必要で `push` 内に panic 経路なし） | 適合（根拠のみ） |
| TR-10 | コピー / 書き出し | click 前 0 回、click 後 1 回、本文は表示中の行 + 除外規則、file 名 `kukuri-logs.txt` | `developer log viewer copies and exports the shown lines only on request`（render 直後 `writeText` 0 回・downloads `[]`）、`downloadTextFile` unit、Playwright `download` event の `suggestedFilename` | 適合 |

## AC / INVAR evidence

- AC-1: 表示条件は drawer と `DeveloperPanel` の二重 guard。viewer は summary に `maxEntries` / `maxBytes`（`view.maxEntries.toLocaleString()`、`formatDesktopLogBytes`）を出し、各行に `level` を文字 + 色で表示。OFF 時は `DesktopShellDeveloperLogs` が unmount され `readDesktopLogs` は呼ばれない（shell test で OFF 時 0 回、ON 後 1 回、OFF に戻して増えないことを spy で検証。`SettingsPanels.test` で OFF rerender 後に「Logs」heading 不在）。Playwright は初期 `developer-mode=false` で heading `toHaveCount(0)` → check → region 表示 → uncheck → `toHaveCount(0)`。PASS
- AC-2: `copyLogs` / `exportLogs` は `onClick` からのみ到達（`rg` で他 caller なし）。file 名は `DESKTOP_LOGS_EXPORT_FILE_NAME = 'kukuri-logs.txt'`（unit test で固定）。Vitest は render 直後に clipboard / download が 0 回であることを先に assert してから click。Playwright は `page.waitForEvent('download')` で `kukuri-logs.txt` を確認。PASS
- AC-3: `LOG_BUFFER_MAX_ENTRIES / MAX_BYTES / LINE_MAX_BYTES` は `const` で `production_limits_are_fixed` が値を固定。`push` の eviction loop は件数と byte の両方を条件にし、`byte_len` は truncate 後の `target + message` で加減算が対称（`bytes` の整合は `stats()` で test）。`truncate_line` は `is_char_boundary` まで戻してから切るため UTF-8 境界を壊さない（`long_line_is_truncated_on_a_char_boundary_…` で「ああ」prefix を確認）。timer / 購読なし、buffer は process に 1 つ。PASS
- INVAR-1: 秘密値をログへ出さない契約は `crates/*`・command 本体に差分なし。監査者側でも `rg` で workspace の tracing macro に `passphrase / secret / nsec / private_key / plaintext / token` の値出力が無いことを確認（0 件）。viewer は `entry.message` をそのまま描画し、export 本文は header（件数・上限・除外規則）+ `ISO LEVEL target: message` のみで、backend の event 以外の情報源を持たない。`console.error` の追加なし。維持
- INVAR-2: `resolve_tracing_directives` と既存 2 test は無変更。`init_tracing` は `fmt().with_env_filter(..).with_target(true)` を据え置き、`.finish().with(layer).try_init()` に変えた。tracing-subscriber 0.3.23 の source で確認: 旧 `SubscriberBuilder::try_init` は `self.finish().try_init()`（`util::SubscriberInitExt`）を呼ぶだけなので、新経路と同じ `set_global_default` + `tracing_log::LogTracer` 初期化（default feature `tracing-log` 有効）を通る。`Layered::enabled` / `event_enabled` は outer layer（既定 true）→ inner（EnvFilter）の順で、EnvFilter が落とした event は `event()` に届かない。`max_level_hint` は `cmp::max(None, Some(inner))` = inner で不変。`layer_receives_only_events_that_pass_the_stdout_filter`（debug と対象外 target は buffer にも入らず、通過した 2 件だけ）。維持
- INVAR-3: 両 command は `NON_READY_COMMAND_ALLOWLIST` に無く、`command_allowed_during_exit` の allow にも無い。`ipc_round_trip_…` は mock app で Initializing → 拒否、Ready + mirror OFF → `developer_mode_disabled`、mirror ON → 取得（`afterSeq` / `limit` の camelCase 受理、`level == "WARN"`、`oldest_seq` / `next_seq`）、OFF → 再拒否まで通す。`command-parity.json` は 146 entry（`generate_handler!` の 339〜484 行 = 146 と一致）、除外 kind `gui_diagnostics` は `exclusion_allowed` に command 名で固定、`baseline_inventory_is_classified_once` の件数 assertion を 146 へ更新。維持

## 文書・locale の整合

- `docs/legal/developer-log-viewer-data-classification.md`: ADR 0002 の 11 項目（Feature 名 / Durable-Transient / Canonical Source / Replicated / Rebuildable From / Local Only / Gossip Hint / Blob / SQLite projection / 必須 contract / 必須 scenario）がすべてある。参照先 `docs/legal/account-key-export-data-classification.md` は存在する。
- `docs/legal/app-data-flow-inventory.md`: 「アプリ内ログ(開発者モード)」行の保持上限・非永続・利用者操作限定の記述は実装と一致。
- `docs/runbooks/mvp-troubleshooting.md`: 「Windows は取得不能」を「アプリ内から書き出す」に置換、Failed 状態は読めない旨を明記（設計の対象外と一致）。`docs/README.md` Legal に参照追加。
- locale: ja / en / zh-CN の `developer.*` キー集合は同一（`developer.logs.*` 19 キー）。削除した `developer.logs.description` / `.windows` への参照は残っていない。`parity.test.ts` は namespace ごとに全キーを比較する。

## 実行した validation（監査者は再実行せず、読解と逆引きのみ）

- `git diff 62d6920..8323266` の全 40 file を読解（source / test / 登録簿 / docs / i18n / stories）。
- `rg` による sink caller の逆引き（上表）と、`generate_handler!` の登録数・`command-parity.json` の entry 数の突合。
- tracing-subscriber 0.3.23（`~/.cargo/registry`）の `fmt/mod.rs` `try_init`、`util.rs` `SubscriberInitExt::try_init`、`layer/layered.rs` の `enabled` / `event_enabled` / `event` / `pick_interest` / `pick_level_hint` を読解。
- 実装者環境の結果（引用）: Tauri backend unit 14 件成功（tracing 8 / developer_logs 2 / invoke_gate 4）、`command_parity` 5 件成功、`cargo xtask check` 成功、`cargo xtask oversized-files` 成功（`runtimeApi.ts` 1041 行 / 基準 1042）、targeted Vitest 成功、Playwright `developer-mode.spec.ts` 16 件成功、Storybook build 成功。CI（Kukuri Fast）、全体 Vitest、rust-test、e2e-smoke は実装者側で確認中。Tauri 実機（Linux / Windows）は未確認。

## blocker

0 件。

## non-blocker とした事項

- Optional-hardening: `truncate_line` は 4 KiB で切った後に `…(truncated N bytes)`（最大 30 byte 程度）を付けるため、1 行の実長は 4 KiB を僅かに超え得る。合計は `LOG_BUFFER_MAX_BYTES` で別途上限が効き、test も `< 18 + 1 + 32` でこれを織り込んでいる。AC-3 の「固定」には抵触しない。
- Optional-hardening: `byte_len` は `target + message` の payload だけを数え、`DesktopLogEntry` の構造体分（seq / timestamp / level / String header）は含まない。2,000 件 × 定数で bounded なので長時間稼働で増え続けることはないが、「1 MiB」は RSS ではなく payload の上限である。
- Optional-hardening: 1 行が単独で `max_bytes` を超えると eviction loop がその行自身も落とす（`新しい行は捨てない` に反する）。production 上限（1 行 ≤ 4 KiB + suffix + target ≪ 1 MiB）では到達不能。
- Optional-hardening: buffer の行は event の field だけで、fmt layer が標準出力に前置する span context を含まない。app crate（`src-tauri`、`kukuri-app-api`）に span 使用は 0 件で、差は依存 crate の行に限られる。INVAR-2 は標準出力側の不変を求めるもので、buffer 側の書式は AC に含まれない。
- Optional-hardening: mirror は mount 時と変更時だけ送る best effort。shell が mount されたまま startup 状態が一時的に非 Ready になる区間（device restore 実行中の Initializing）で利用者が開発者モードを ON にすると、その mirror は gate で拒否され、viewer は `developer_mode_disabled` を alert に出す。「更新」では回復せず OFF → ON の再操作が必要。到達には restore 進行中の toggle という限定的な操作が要り、error は可視で回復手段もあるため blocker にしない。`read` が `developer_mode_disabled` を受けたときに mirror を再送する案は hardening。
- 既存パターン踏襲: `copyLogs` は `navigator.clipboard.writeText` が reject した場合に unhandled rejection になる（`void copyLogs()`）。`ReleasePanel.copyDiagnosticReport` と同型で本差分が新たに壊したものではない。
- New-requirement（scope 外、記録済み）: Tauri 実機（WebKitGTK / WebView2）での `<a download>` 保存ダイアログ・clipboard・実 backend 行の取得は未確認。既存の診断レポート書き出しと同じ経路を共有しているため独立した新経路ではない。Failed 起動状態での閲覧は INVAR-3 の固定により対象外。
- 備考: 視覚 baseline は `developer-enabled-ja-dark.png`（1280×800）だけが再生成され、spec が同じ loop で撮る `developer-enabled-en-light.png`（390×844）は未変更。390 幅では drawer 内 scroll の可視域にログ節が入らず差分が出なかったと解釈できる（baseline workflow `34655685231` が ja-dark のみ更新した事実と整合）。head での視覚 CI の結果は実装者側で確認中。

## 実装者 inventory との差分

実装者の progress 記録は INV-1〜6 / TR-1〜8。監査者の再構築（E1〜E14 / TR-1〜10）との差は次のとおりで、いずれも実装者側の分類を覆すものではない。

- 監査者は mirror の入口を mount（E2）と toggle（E3）に分け、終了中（E12）、アカウント切替の `location.reload()` 経由の再 mount（E13）、lock 毒化（E14 / TR-9）、Storybook（E10）、Failed 状態で shell が mount されない事実（E11）を別行にした。実装者側は INV-2 / TR-6 にまとめている。
- 実装者側 TR-7（取得失敗）に、監査者は「shell mount 中の一時的 non-Ready で toggle した場合に mirror が失われ再 toggle が必要」という sequence を追加した（上記 Optional-hardening）。
- 実装者側の「視覚 baseline は `developer-enabled-{ja,en,zh-CN}-{dark,light}.png` が変わる」は実際の spec と一致せず、存在する baseline は ja-dark / en-light の 2 つで、変更されたのは ja-dark のみ（上記備考）。
- sensitive sink の逆引き結果は一致（新規 network / DB / file sink 0、各 sink の production caller は 1 箇所）。

## 監査後 delta

- 本記録の追加（docs のみ）。source / test / i18n / 登録簿に差分なし。再監査は不要。
