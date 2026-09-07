# #905 Linux Deb配布・署名付き更新

## 現在判定

- In progress。基準 `c4616fc706b94150ac6c2ac06aec68bc1c2b0f5a`。
- Scope revision: `2026-09-07-linux-deb-release-addition-v2`。リスク区分C。
- 2026-09-07に計画・実装・commit・PR・必須CI／独立監査成功後のmergeを利用者が承認。Releaseのversion／tag／公開承認は別工程。
- 固定AC-D1〜6、INVAR-D1〜5、INV-D1〜5、TR-D1〜5（TR-D4a〜d）は[Issue #905](https://github.com/KingYoSun/kukuri/issues/905)。#890の公開前追加依存であり、#889／#904の完了済み監査は変更しない。

## 変更前の証拠

- `cargo test --locked -p xtask linux_bundle_config_enables_signed_appimage_and_deb`: FAIL。実configは`appimage`単独、期待は`[appimage, deb]`（AC-D1／TR-D1）。
- `python -B -m unittest scripts.release.test_release_assets.ReleaseTests.test_linux_appimage_without_deb_is_not_a_complete_release`: FAIL。Debなしの4target候補を拒否せず集約できた（AC-D3／TR-D5）。
- `test_deb_package.py`はpayload契約を実装前に追加。検査module未作成によるimport失敗を確認したが、これは既存挙動の再現成功とは区別する。
- locked Tauri updater 2.10.1はDeb認証失敗後にzenity／kdialog／sudoへfallbackする。標準installerをDeb UIから呼ばない最小adapterが必要（AC-D4／TR-D4c）。

## 追加surfaceと実装判断

- INV-D1／2: 同じTauri buildで両形式を生成。Debはsystem配置と宣言依存を使用し、maintainer scriptとユーザーデータ操作を追加しない。署名検査後に実archiveのmetadata、ELF、desktop／icon／deep-link、noticeと全payload pathを検査する。
- INV-D3: 実装中。署名済みbytesと昇格install間の書換え境界、取消時停止、形式別targetを固定する。定期・手動checkと直接IPCのcallerを含む。
- INV-D4／5: 集約・native資料・公開確認・運用文書をDebへ拡張する。公開済みとは扱わない。

## 検証

必須: xtask tests、Deb／release Python・PowerShell contracts、Linux package CI、Tauri check／e2e-smoke、frontend変更時のdesktop-ui-check、`local2` Ubuntu 24.04.4 LTS実機、固定PR head独立監査。未実施分は成功扱いしない。

実機はSSH到達とOS／archだけ確認済み。既存Debなし、`sudo -n`不可。接続済みRemote DesktopのComputer Useは許可済み。OS認証dialogの操作は利用者に引き継ぎ、検証profileを隔離する。実機package変更・GUI確認・更新確認はまだ未実施。

### 初回実装のtargeted結果

- xtask: 45 passed、実AppImage署名fixture 1 ignored（Linux buildで実行する）。追加config契約は変更前FAIL→PASS。
- Release Python: 33 passed。WindowsではUTF-8 modeと検証用PyYAML 6.0.3を使用。
- PowerShell Linux＋Windows集約: PASS（21資材、Debを含む3 updater entry、既存出力不変）。署名wrapperの3形式routing／形式混同拒否: PASS。stub検査を実crypto検査と混同しない。
- updater frontend targeted: 3 files／17 tests passed。Tauri backendはWindowsで`cargo xtask tauri-check` PASS。
- CLI parityは登録＋3を分類した後、固定件数139が不一致となった。基準／revision／件数142へ同期し、再実行する。
- `cargo xtask desktop-ui-check`進行中。Linux compile／実Deb生成・installとmemfd、実Desktop更新は未実施。
- `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check`は新規fileと既存未変更fileのformat差を報告。変更した新規module／xtaskだけrustfmtを適用し、既存state等の無関係なformat差は混ぜない。

### 境界テストと手順の追加

- CLI parity: 5/5 PASS、142入口に同期済み。`cargo xtask e2e-smoke`: post永続6step PASS。
- `cargo xtask desktop-ui-check`: lint／typecheck、151files・1185tests、Storybook、browser64、visual smoke14 PASS。Windowsのvisualはpixel比較ではなく到達smoke、Linux CIのpixel比較も初回headでPASS。
- `local2`の隔離cloneでLinux backendを実行。Deb unit4 PASS（sealed memfdの他handle／child reader、取消／拒否／agent不在、形式／metadata拒否、実dpkgの隔離rootでhalf-configuredとなる部分失敗・試行1回・profile sentinel保持）。実機systemのdpkg databaseは変更していない。
- updater boundary4 PASS（厳密manifest、直接IPC相当のstale／unverified／busy拒否とrestart禁止、installed状態上書き禁止、実capability付きmock Webviewから上流updaterのcheck／download／install／download_and_install拒否）。失敗後payload消費の追加testは実行待ち。
- Windows backend unit testはcompile成功後に`STATUS_ENTRYPOINT_NOT_FOUND`でtest process起動失敗。合格扱いせずLinux実機／CIを使用する。通常のWindows Tauri compileはPASS。
- 実Deb署名の正常download／1byte改変／形式別entry欠落はLinux package CIの既存`updater_install`へ追加し、実package生成後に実行する。fixture routingだけを署名証拠にしない。
- README en／ja、Deb導入・更新・失敗時回復／削除runbook、quickstart／troubleshooting／dev／release、builder preview現状を同期。Debの公開は未実施。
- 手持ちUbuntu Desktop用に隔離stagingの`99.0.1→99.0.2→99.0.3` test-key Debを準備中。公開設定・配布秘密鍵は使用せず、日常profileへ書き込まない。system packageの導入・実GUI更新／取消はまだ未実施。

### 実Deb生成で発見した不一致

- 初回Linux package CI `34078227469`と`local2`でAppImage／Deb生成・署名は成功したが、Deb payload検査が`desktop executable/deep-link mismatch`でFAIL。
- 実Debのdesktop entryは`MimeType=x-scheme-handler/kukuri`を含む一方、`Exec=kukuri-desktop-tauri`でURL引数が欠落していた。Tauri bundler既定値を受理するための検査緩和はせず、Deb専用desktop templateへ`Exec=kukuri-desktop-tauri %U`を設定し、実物を再検査する。
- `test_bundler_default_without_url_argument_is_rejected`へ同じ不一致を固定。既存失敗証拠と新しいfixtureを区別する。
- `local2`のruntime AppIndicatorは導入済みだが開発用pkg-config情報がなく、初回bundlerは停止した。Ubuntuの対応dev package4個を検証directoryへ非root展開し、限定`PKG_CONFIG_PATH`で解決。OSのAPT設定・installed packageは変更していない。
