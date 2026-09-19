# Windows Microsoft Store 配布

## 対象

Issue #1190で追加したWindows x64 MSIXのbuild、local test、WACK、Partner Center提出を扱う。通常のWindows NSIS／GitHub updaterは[Release Runbook](release.md)の経路を使い、本書のStore profileと混在させない。

Store packageの更新はMicrosoft Store／Windowsへ委譲する。kukuriはStore buildからGitHub Releasesを自動確認せず、`Windows.Services.Store.StoreContext`等の別updaterも持たない。SettingsはStore管理であることを表示する。

Microsoftは[`StoreContext`によるpackage更新](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/package-updates-from-store)も任意APIとして提供するが、#1190では採用しない。既定のStore更新経路をもう一つのapp内state machineで包まず、更新の有無・download・install・再起動はStore／Windowsの表示と設定を正とする。

## 固定identityとtool

| 項目 | 値 |
| --- | --- |
| `Package/Identity/Name` | `KingYoSun.kukuri` |
| `Package/Identity/Publisher` | `CN=33EB763C-4859-4E44-886F-1784E16DD6D5` |
| `PublisherDisplayName` | `KingYoSun` |
| PFN | `KingYoSun.kukuri_p8fpcaf1kx88g` |
| Store ID | `9NQ18HML4GS3` |
| 初回Store package version | `1.0.0.0` |
| WinApp CLI | `0.6.1` |

identityとStore versionの正本は`apps/desktop/src-tauri/windows/store/Package.appxmanifest`。Partner Centerの値はcase、空白、句読点を含め完全一致させる。Store versionはapp SemVerとは別の4整数で、末尾を0とし、既存x64 packageより上げる。

WinApp CLIの導入とversion確認:

```powershell
winget install Microsoft.WinAppCli --source winget
winapp --version
```

0.6.1以外ではpackageを作らず、CLI更新とmanifest変換結果を別変更で監査する。CIは`microsoft/setup-WinAppCli`をcommit SHAでpinし、実versionを検査する。

## Store候補のbuild

worktreeをcleanにして実行する。

```powershell
cargo xtask windows-store-package
```

commandは次を一つの工程として行う。

1. `VITE_KUKURI_DISTRIBUTION=microsoft-store`とCargo feature `microsoft-store`でTauri x64 release binaryを`--no-bundle` buildする。
2. 新しい空stagingへ`kukuri.exe`とmanifestが参照する3つのiconだけを配置する。
3. WinApp CLI 0.6.1の`pack`をcertificate optionなしで実行する。
4. `dist/microsoft-store/KingYoSun.kukuri_<store-version>_x64.msix`と`store-package.json`を生成する。

`store-package.json`にはsource commit、dirty状態、app／Store version、identity、architecture、WinApp CLI version、unsigned candidateのSHA-256を記録する。`-AllowDirty`は実装中のlocal確認専用で、Partner Centerへ送る候補には使わない。

`winapp pack` 0.6.1は`--cert`を付けないとunsigned packageを作る。Store提出候補はこのunsigned MSIXであり、PFX、password、local署名copyをuploadしない。Microsoft Storeはcertification後にpackageを再署名する。

## identity付きloose smoke

PFXなしでpackage identity、activation、通知を先に確認できる。

```powershell
winapp run .\dist\microsoft-store\staging `
  --manifest .\apps\desktop\src-tauri\windows\store\Package.appxmanifest `
  --executable kukuri.exe `
  --unregister-on-exit
```

別processで確認するときは`--detach`を使い、終了後に次を実行する。

```powershell
winapp unregister --manifest .\apps\desktop\src-tauri\windows\store\Package.appxmanifest
```

`winapp unregister`はdevelopment mode登録だけを対象にする。別project treeの登録へ`--force`を使わない。Issue #1190ではAUMID `KingYoSun.kukuri_p8fpcaf1kx88g!kukuri`、single processの`kukuri:`再activation、対象登録だけの解除を確認した。

## PFXとlocal-test署名

repository rootの`code_sign_certificate.pfx`は`*.pfx`でGit除外されている。PFX、password、private keyをGit、log、cache、CI artifact、Issue／PRへ保存しない。Store候補の作成はPFX非依存である。

signed local-test copyが必要なときだけ、passwordをterminalから`SecureString`へ直接mask入力する。`setx`、command引数、`winapp --cert-password`、`SignTool /p`を使わない。

```powershell
cargo xtask windows-store-package --skip-build --sign-local --prompt-certificate-password
```

非対話の隔離されたlocal jobでだけ、`KUKURI_MSIX_CERT_PASSWORD` process環境を代替入力にできる。persistent user／machine環境へ保存しない。

scriptはPFXを`CurrentUser\My`へnon-exportableで一時importし、manifest Publisherとの一致、private key、期限、code-signing用途を検査する。unsigned candidateのcopyだけをthumbprint指定でSHA-256署名し、`SignTool verify`後にpublic `.cer`と別hashをprovenanceへ追加する。処理前から存在したcertificateは残し、この処理で新規importしたcertificateだけを`finally`で削除する。

signed copyをinstallするtest user／VMでは、出力した`.cer`だけを`CurrentUser\TrustedPeople`へ一時importし、MSIXを`Add-AppxPackage`する。test後は対象packageと、このtestで追加したcertificateだけを正確なidentity／thumbprintで削除する。利用者の実profileや他certificateをcleanup対象にしない。

## app dataとDirect版の共存

package identity付きloose runでも、kukuriのapp data正本はDirect／NSIS版と同じ`%APPDATA%\app.kukuri.desktop`だった。Windowsのpackage container `%LOCALAPPDATA%\Packages\KingYoSun.kukuri_p8fpcaf1kx88g`も作成されるが、account registry、DB、consent、notification設定の正本として使わない。development package解除後も既存app dataが残ることを確認済み。

- Store版への切替でdataをcopyしない。既存pathをそのまま使う。
- Direct版とStore版を同時起動しない。同じprofile DBを二つのprocessで開かない。
- 切替前に全accountのdevice backupを別の安全な場所へ作る。
- MSIXのuninstallをkukuri data削除手段として扱わない。data削除を目的に`%APPDATA%\app.kukuri.desktop`やpackage containerを手動削除しない。

同一identityの上位Store versionへupgradeするときは、account key、profile、consent、settings、draft、private capability、DBをbefore／afterで照合する。失敗時にapp data削除で回復しない。

## WACKとPartner Center

提出候補はclean worktreeから作ったunsigned MSIXに固定し、source SHA、Store version、SHA-256を記録する。

1. 同じpayloadから作ったsigned local-test copyをWindows App Certification Kitとinstall／upgrade smokeへ使う。
2. unsigned candidateをPartner CenterのStore ID `9NQ18HML4GS3`へuploadする。
3. WACK report、signed copy hash、unsigned candidate hash、`store-package.json`の対応を記録する。
4. Partner Center validationのerror／warningを保存し、失敗をoverrideしない。再buildは別candidateとして全検査をやり直す。
5. certification提出とavailability／一般公開を分離し、承認された公開範囲・日時だけを適用する。
6. certification後にStoreから取得したpackageのMicrosoft signature、identity／version、起動、Store update認識を確認する。

Store upload、certification、一般公開は外部状態の異なる操作である。実装PRやplanの承認だけを一般公開の承認として扱わない。

## 回帰検証

```powershell
python scripts/release/test_windows_store_package.py
cargo test -p xtask desktop::package_tests
cd apps/desktop
npx pnpm@10.16.1 test -- src/lib/distribution.test.ts src/components/settings/ReleasePanel.update.test.tsx src/shell/DesktopShellPage.updateSchedule.test.tsx
```

加えて`cargo xtask check`、`cargo xtask test`、`cargo xtask e2e-smoke`、既存release contractsを実行する。Store差分を通常`desktop-package`へ混ぜず、NSIS／GitHub updater、Linux AppImage／Deb、CLIの既存成果物を維持する。
