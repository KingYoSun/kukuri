# #905 Linux Deb配布・署名付き更新

## 現在判定

- 実装・実機検証済み。製品コード`747a7440`は全CI・独立監査PASS。最終記録差分の監査・merge tree確認はPR #906／Issue #905へ記録する。基準 `c4616fc706b94150ac6c2ac06aec68bc1c2b0f5a`。
- Scope revision: `2026-09-07-linux-deb-release-addition-v3`。リスク区分C。v2のDeb製品条件は不変、後述の承認済み通知test安定化を追加。
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

## Ubuntu Desktop実機の追加証拠

2026-09-07、SSH `local2`と接続済みRemote Desktopが同一Ubuntu 24.04.4 LTS／amd64／GNOME Wayland sessionであることを確認して実施。日常profileを使わず、検証directoryのXDG data／config／cacheへ隔離した。OS認証と18歳以上の自己申告は利用者が操作し、passwordを取得・保存していない。

### 資材と実行条件

- GUI buildの変更入力7ファイルは`747a7440`のGit blobと一致。検証専用のconfig overlayで版`99.0.1`／`99.0.2`／`99.0.3`、一時公開鍵、localhost endpointを指定したdebug Debを使用。公開用設定・秘密鍵やReleaseは変更していない。Ubuntu 22.04のrelease-mode成果物はLinux package CIで別途検査する。
- AppIndicatorの開発用pkg-config情報だけを非root展開。必要runtimeは既存system依存を使用。
- manifest serverは`127.0.0.1`のみでlistenし、公開directoryにはDeb／署名／manifestだけを配置。一時秘密鍵は配信directoryに置かない。
- 検証用user systemd unitは`ExitType=cgroup`。Tauriの再起動childを旧parent終了時にtest launcherがkillしないための条件で、製品へsystemd依存や新サービスは追加していない。

| 検証用Deb | SHA-256 |
| --- | --- |
| `99.0.1` | `3fc92808f908dc39253d20f04f50ba1e52c16a51a465ada47111190e3d9d1b5a` |
| `99.0.2` | `af154b519084b7b2deb274441db3faebd81af6c1f8a022708f345718ee9cb64a` |
| `99.0.3` | `a2d75a4d1c77eaa3dbd9e6ad92ef1bc2164b30c54248fcbef89032988d72da68` |

全Debのmetadata／配置／desktop entry／icon／deep-link／依存／notice検査が成功、payloadは6files。同じ実Debで`real_deb_signature_tampering_and_missing_target_fail_closed`がPASS（正常署名受理、1byte改変拒否、AppImage entryからのfallback拒否）。Linux backend追加5 updater tests／4 Deb testsもPASS。

### 導入・成功更新（TR-D3／TR-D4a）

- OS認証後に`99.0.1`の`install ok installed`とアプリ一覧のkukuriを確認。非root（uid1000）で起動し、利用者の初回同意後にruntime初期化を確認。
- GUIの定期確認で`99.0.2`を検出、明示download後に署名検証完了を表示。利用者が明示適用とOS認証を行った。
- 12:39:42 JSTに旧GUIのhost停止完了・exit0、12:39:45に新GUIが初期化。installed packageは`99.0.2`。再起動processもuid1000で、実行中ELF／installed ELF／`99.0.2`のDeb内ELFのSHA-256はすべて`8c95d4f78efd1d11c82f025b3d2cf10a0a2e07a7444e31daebd20e1509eb36ce`に一致。
- 同一アカウント登録簿のhash、DBの相対保存先、全32テーブルの行内容hashが不変。fresh profileのDBはmigration tableのみ非空であり、投稿保持を試験したとは扱わない。
- 代表的な保存データとしてGUIに入力した未公開下書き`Issue 905 Deb update retention draft - do not publish`を使用。更新前後ともWebKit localstorage DBに存在し、再起動後のComposerにも同じ本文を表示。投稿送信は行っていない。

### 認証取消（TR-D4c）

- `99.0.3`を手動確認・取得し、署名検証完了後に利用者が明示適用とOS認証の「キャンセル」を操作。
- 12:44:09 JST、pkexecは`Request dismissed`。GUIは権限承認取消・適用／再起動未実施の案内を表示。別方式の認証は開始しなかった。
- installed packageは`99.0.2`、GUI PIDは更新直後と同じ、アカウント登録簿／DB保存先／32テーブル／保存下書きも不変。再起動0回、認証の自動再要求0回。
- 不正形式・未検証／stale／busy・直接upstream IPC拒否、seal書換え拒否、agent不在、実dpkg隔離rootの部分失敗は前述の自動境界testsで補完。実機system packageの部分失敗注入は行っていない。

### 後片付け・CI

- 12:46:21 JSTに検証GUIへ通常のSIGTERM終了要求を送り、host停止完了を確認。12:48:31に利用者のOS認証後の`apt-get remove`が完了した。package、実行binary、desktop entryは不在となり、導入前の状態へ復帰した。
- package削除後も同じアカウント登録簿、DB保存先、32テーブル、保存下書きが残ることを確認した。検証用profileとbuild資材は保持し、日常データに変更はない。
- localhost serverと検証GUI unitは停止。一時署名秘密鍵と検証用runtime unitのdrop-inを削除した。公開鍵／署名／資材／保持確認のhash記録は再検証用に保持した。
- 製品コード`747a7440`の[Linux package CI](https://github.com/KingYoSun/kukuri/actions/runs/34079424398)はPASS。両形式の実署名検証、Debのinstall／reinstall／removeとsentinel保持、Linux backend 53tests、実Deb／AppImage updater 2testsがすべて成功。
- [fast CI](https://github.com/KingYoSun/kukuri/actions/runs/34079424384)、[CLI 2arch](https://github.com/KingYoSun/kukuri/actions/runs/34079424399)、[release contracts](https://github.com/KingYoSun/kukuri/actions/runs/34079424402)もPASS。全14checks成功。最終記録commitのrequired checksと監査をmerge前に確認する。

## 条件と証拠の対応

| 固定条件 | 実装・証拠 |
| --- | --- |
| AC-D1／INVAR-D1／INV-D1／TR-D1・2 | Tauri同一build、`xtask::appimage::verify_package`、Deb実payload検査、config fixture、Linux package CIの両成果物・署名 |
| AC-D2／INVAR-D3／INV-D2／TR-D3 | 使い捨てCI apt導入／再導入／削除＋sentinel、実機非root起動／正常終了・削除後保持、scriptなしの実archive |
| AC-D3／INVAR-D2・4／INV-D4／TR-D5 | `release_assets.py`、manifest3entry、Deb必須の欠落・改変・source／version／arch／notice fixtures、実署名verifier、既存出力／asset非置換tests |
| AC-D4／INVAR-D3〜5／INV-D3／TR-D4a〜d | 厳密target、backend private verified bytes、sealed memfd、単一pkexec、upstream IPC ACL拒否、成功限定restart、実機成功／取消、実dpkg隔離root部分失敗 |
| AC-D5／INVAR-D2／INV-D1・4 | Deb内notice6file payload、`native_compliance.py`の実Deb再検査・hash/source結合、first-party ELFとsystem dependencyの区別 |
| AC-D6／INV-D5 | README en／ja、ADR0049、Deb／release／quickstart／troubleshooting／dev手順、builder preview現状、CIと独立監査。実公開は#890へ引渡し |

独立監査は[監査記録](2026-09-07-issue-905-linux-deb-updater-audit.md)を参照。Release version／tag／sourceの公開承認は本Issueの実装承認に含めず、Debを含む最終公開確認が終わるまで#890をCloseしない。

## v3: 承認済み通知テスト安定化

最終記録head`794e6c9f`のCIで既存通知テストが2回5000msを超過したためmergeを保留し、利用者へ範囲追加を確認した。2026-09-07に「含めて修正してください」と承認を受け、AC-D6／T6にtest安定化を追加した。Deb本体・更新境界・既存の製品inventoryは変更しない。

- 修正前の再現: Fast CI `34081123960`の初回とattempt2で`os notification ... focus-reply-40`がtimeout。attempt2は5225ms、残り1184testsは成功。ローカル変更前の9testsは成功したが、同case3033ms、file内test合計23.64sであった。異なる環境の時間を同条件の性能保証として比較しない。
- 原因の切り分け: fixtureの背景TimelineとThreadで同じ返信群を重複描画していた。通知targetをThreadへ開いてfocusする契約に不要な背景側だけを親投稿1件に絞った。Threadには45件すべてを残す。
- 維持した検証: OS／in-app、first／older pageの両target、対象postのclassとfocus、scroll container、route、topic／thread、再クリック、競合、missing target、private channelの全既存assertions。新contractでThreadの30件＋15件、各targetの所属pageを明示的に検査した。
- 修正後の同じローカルcommand: 対象10tests PASS、問題case1445ms、file内test合計9.41s。timeout延長・retry・skip・製品コード変更は0。
- `cargo xtask desktop-ui-check`と新headのCIで必須validationを確認する。前回Deb監査PASSを維持し、testと記録deltaだけを独立監査する。最終の結果とmerge tree照合はPR #906に集約し、製品差分のない証跡更新のためだけに全実機試験を繰り返さない。
