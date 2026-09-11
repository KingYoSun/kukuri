# #967 独立監査記録（PR #984 head `4bd220c`）

実装者の結論と [progress 記録](2026-09-11-967-backup-discoverability.md) を前提にせず、別コンテキストの監査者が固定 AC / INVAR と対象 commit の差分から再構築した。手順は [Issue lifecycle runbook](../runbooks/issue-lifecycle.md) の「5. 独立監査」に従う。

- 対象 commit: `4bd220c4e627c5654234be5f91d9530b528ca944`（main 先端 `a17de1f` からの 3 commit: `df4e1d9` 実装 / `068201b` 視覚 baseline 再生成 / `4bd220c` docs。source 変更は `df4e1d9` のみ。`src-tauri`、`lib/api/deviceBackup.ts`、`lib/api/identity.ts` に差分なし）
- Scope revision: `2026-09-10-first-look-backup-discoverability-v1`
- リスク区分: C
- inventory: 合計 12 / 適合 12 / 不適合 0 / 未分類 0
- 判定: **PASS**
- 監査日: 2026-09-11

## inventory（監査者が登録点から再生成）

| ID | 入口 | helper | sink / 副作用 | 判定 |
| --- | --- | --- | --- | --- |
| E1 | 設定 nav「バックアップと復元」（`settings-section-backup`） | `SettingsDrawer.onSectionChange` → `changeSettingsSection` | `shellChromeState` + `syncRoute('replace')` のみ | 適合 |
| E2 | 設定 nav「アカウント」（位置移動、AccountKeyPanel 単独） | 同上 | 同上 | 適合 |
| E3 | deep link `?settings=backup`（初回: `slices/chrome.ts` の `parseInitialSettingsSection`、稼働中: `useRouteSynchronization`） | `isSettingsSection`（`backup` 追加） | section / open 状態のみ | 適合 |
| E4 | deep link `?settings=account` | 同上。chrome.ts の重複列挙を `isSettingsSection` に一本化 | 同上 | 適合（既存 drift の修正） |
| E5 | Control Center「システム」の「バックアップと復元」 | `openSettings('backup')` → `setOpen(false)` + `handleOpenSettingsSection` | drawer open / section / `syncRoute('push')` | 適合 |
| E6 | DeviceBackupPanel「アカウント鍵の設定を開く」 | `onOpenAccountKeys` → `openDiagnosticSettings('account')` | section / URL / nav item への focus | 適合 |
| E7 | AccountKeyPanel「バックアップと復元を開く」 | `onOpenDeviceBackup` → `openDiagnosticSettings('backup')` | 同上 | 適合 |
| E8 | `SettingsDrawer` の `scrollIntoView` effect（open / section 変更時） | DOM scroll のみ、focus 不変、`typeof` guard あり | なし | 適合 |
| E9 | Storybook `DeviceBackupPanel.stories` | `onOpenAccountKeys={() => undefined}` | なし | 適合 |
| E10 | 既存 DeviceBackupPanel の作成 / file 選択 / preview / 復元 / cancel | `handleCreate` / `handleChooseRestore` / `handlePreview` / `handleRestore` / cancel button | `lib/api/deviceBackup.ts` の 6 sink | 適合（handler・guard 無変更。`rg` 逆引きで caller は panel 内 handler のみ、新規入口から到達不能） |
| E11 | 既存 AccountKeyPanel の export / preview / import / switch | `handleExport` / `handlePreviewImport` / `handleImport` / `handleSwitch` | `lib/api/identity.ts` の 4 sink | 適合（同上） |
| E12 | panel mount 時の read（`listenDeviceBackupProgress`、`listAccounts`） | 既存 effect | 購読 / 読み取りのみ。以前も account section 表示時に同じ | 適合 |

## AC / INVAR evidence

- AC-1: `docs/progress/assets/967/before-settings-nav-ja-dark-1280x840.png` を閲覧し、変更前は nav が「開発者」で途切れ「アカウント」が nav 可視域外であることを確認。変更後 `settings-backup-ja-dark-1280x840.png` で 13 section が可視、`control-center-system-ja-dark-1280x840.png` に入口あり。未発見条件（既定 window 高さ以下 + ラベル「アカウント」のみ + Control Center 導線なし）は progress 記録に表と操作順で固定。実機 .deb 画像は承認済みの browser 代替。PASS（実機未確認は記録済みの exception）
- AC-2: `SETTINGS_SECTION_COPY` / `isSettingsSection` / `types.ts` / drawer / 3 locale `shell.json` に `backup` を登録。開発者モード判定は section の表示に関与しない（drawer は `developerModeEnabled` を panel の `showDiagnostics` にしか使わない）。Vitest `settings expose a backup section ...`、`the Control Center system section opens backup & restore directly`、Playwright ja / en の `Control Center opens backup & restore`（`DEVELOPER_MODE_STORAGE_KEY='false'` seed）。PASS
- AC-3: `deviceBackup.keyOnlyHint` / `accountKey.scopeNotice` を両 panel の操作 control より前に配置し、鍵のみと端末内データ（投稿・下書き・添付・設定・非公開チャネル秘密）の対象差を明示、相互 button で往復。Vitest `backup and account sections explain the scope difference and link to each other`、Playwright 相互移動。3 locale は `parity.test.ts` で鍵一致。PASS
- AC-4: Playwright `backup-discoverability.spec.ts`（1280×840 / 1280×768 で全 nav item が可視域、Enter で入口、本文 scroll で復元見出し到達、相互移動先 nav item への focus、Escape で閉じて URL から `settings=` 消失、1280 / 700 / 390 で横 overflow なし、ja dark / en light）。`developer-mode.spec.ts` は nav 順変更（developer が末尾）に合わせ Tab 数を更新（assertion は弱めていない）。既存 `DeviceBackupPanel.test.tsx` 3 件・`AccountKeyPanel.test.tsx` 4 件（成功 / 置換確認 / cancel 非表示 / export・import guard）は無変更で成功。PASS
- AC-5: 変更 path（shell / i18n / css / Playwright）に対応する validation を本監査で再実行し成功（下記）。本報告が区分 C の独立監査記録。PASS
- INVAR-1: 新規入口 E1〜E9 の到達先は `changeSettingsSection` / `openDiagnosticSettings` / `openSettings` のみ。`rg` 逆引きで 10 sink の caller は各 panel の handler と `lib/api` のみ。Vitest 5 件すべてで `expectNoSinkCalls()` が 10 sink の 0 回を検証。spy の妥当性: `vi.mock('@/lib/api/deviceBackup')` / `vi.mock('@/lib/api/identity')` が panel の import specifier と一致することを確認。維持
- INVAR-2: 追加文言（ja / en / zh-CN）にパスフレーズ・鍵・復号内容の値なし。`console.` の追加なし。維持
- INVAR-3: panel handler、`lib/api/*`、`src-tauri` に差分なし。確認 checkbox 前の passphrase 入力不可・作成 button disabled を Vitest `keeps the existing risk acknowledgement` で確認。置換確認・cancel は既存 panel test で維持。同意前の外部通信禁止に関わる code に差分なし。維持

## 実行した validation

- `npx vitest run`（backupDiscoverability、routes.unit、DeviceBackupPanel、AccountKeyPanel、parity、developerMode、shellChrome の 7 ファイル）: 170 tests passed（38.3s）
- `npx tsc --noEmit`: exit 0
- `VITE_KUKURI_DESKTOP_MOCK=1 vite build`（対象 commit の source で再 build、成功）
- Playwright `--project=chromium`: `backup-discoverability.spec.ts` 5 passed（再 build 前後とも）、加えて `developer-mode.spec.ts` / `localization-layout.spec.ts` / `hash-routing.spec.ts` を含め 39 passed（29.6s）
- `rg` による sink caller 逆引き、`git diff a17de1f..4bd220c` の src / i18n / docs 差分の全読

## blocker

0 件。

## non-blocker とした事項

- Existing-gap（本差分で修正）: `slices/chrome.ts` の重複列挙に `account` が無く、初回 load の `?settings=account` が既定 section に落ちていた（`a17de1f` 時点の `isSettingsSection` には `account` あり）。`isSettingsSection` への一本化で解消。挙動変化だが利用者不利益ではない。
- Existing-gap（未変更）: 作成 / 復元 pending 中に nav や相互案内 button で section を移動すると非 keepMounted の panel が unmount され、進捗と cancel button が消える（backend 処理は継続）。変更前も他 section への移動で同じ挙動であり、本差分は新しい移動経路を 1 つ増やしただけ。INVAR-3 の cancel は panel 表示中は維持。
- Optional-hardening: 上記に対し pending 中は相互案内 button を disabled にする案。固定 AC / INVAR に含まれない。
- New-requirement（scope 外）: Linux .deb（WebKitGTK）/ Windows WebView 実機での nav 高さ・native dialog、screen reader 聴取、200% zoom は未確認（記録で明示済み、AC-1 は承認により browser 代替）。
- 備考: `DesktopShellSettingsDrawer.tsx` の comment「順序は SETTINGS_SECTION_COPY が正本」は実際の描画順が `settingsSections` 配列順に依存するため厳密には正しくないが、両者は同一順で更新されており実害なし（Optional-hardening）。

## 監査後 delta

- 本記録の追加（docs のみ）。source / test / i18n / CSS に差分なし。再監査は不要。
