# Windows Microsoft Store 配布

## 対象

Issue #1190で追加したWindows x64 MSIXのbuild、package identity付きlocal test、Partner Center提出を扱う。通常のWindows NSIS／GitHub updaterは[Release Runbook](release.md)の経路を使い、本書のStore profileと混在させない。

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
| Store package version | アプリ`major.minor.patch` → `(major+1).minor.patch.0`（`0.2.8` → `1.2.8.0`） |
| WinApp CLI | `0.6.1` |

identityの正本は`apps/desktop/src-tauri/windows/store/Package.appxmanifest`。Partner Centerの値はcase、空白、句読点を含め完全一致させる。versionの入力は既存の`apps/desktop/package.json`のみで、Tauri設定との一致もbuild時に確認する。Store versionはmajorに1を加えて末尾0を付け、自動生成した`dist/<出力先>/AppxManifest.xml`へ反映する。repositoryのmanifestの`0.0.0.0`はtemplate用の印で、直接pack／runには使わない。独立したStore用version bumpは行わない。

Store用は安定版の`major.minor.patch`のみ受理する。prerelease／build metadataは同一番号への衝突を避けるため拒否する。変換後の各要素は65535以下（アプリmajorは65534以下）。同一バージョンの再提出で別カウンタを導入せず、必要なら通常のアプリversionを更新する。アプリ内表示は既存versionのまま。

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
2. 新しい空stagingへ`kukuri.exe`とmanifestが参照する3つのiconを配置し、既存`icon.png`からWindows shell向け14サイズのtargetsize／unplated／lightunplated資産42個を生成する。透過背景を維持し、Windowsのaccent plateを避ける。
3. WinApp CLI 0.6.1の`pack`をcertificate optionなしで実行する。
4. `dist/microsoft-store/KingYoSun.kukuri_<store-version>_x64.msix`と`store-package.json`を生成する。

`store-package.json`にはsource commit、dirty状態、app／Store version、identity、architecture、WinApp CLI version、unsigned candidateのSHA-256を記録する。`-AllowDirty`は実装中のlocal確認専用で、Partner Centerへ送る候補には使わない。

出力先が存在する場合は停止し、既存candidateを削除・上書きしない。再実行は`--output dist/microsoft-store-<識別子>`で新しい出力先を指定する。`--skip-build`は受け付けず、毎回Store専用`target/microsoft-store`で現在sourceをbuildする。通常のNSIS build成果物を再利用しない。

`winapp pack` 0.6.1は`--cert`を付けないとunsigned packageを作る。Store提出候補はこのunsigned MSIXであり、PFX、password、local署名copyをuploadしない。Microsoft Storeはcertification後にpackageを再署名する。

## identity付きloose smoke

PFXなしでpackage identity、activation、通知を先に確認できる。

```powershell
winapp run .\dist\microsoft-store\staging `
  --manifest .\dist\microsoft-store\AppxManifest.xml `
  --executable kukuri.exe `
  --unregister-on-exit
```

別processで確認するときは`--detach`を使い、終了後に次を実行する。

```powershell
winapp unregister --manifest .\dist\microsoft-store\AppxManifest.xml
```

`winapp unregister`はdevelopment mode登録だけを対象にする。別project treeの登録へ`--force`を使わない。Issue #1190ではAUMID `KingYoSun.kukuri_p8fpcaf1kx88g!kukuri`、single processの`kukuri:`再activation、対象登録だけの解除を確認した。

## Store署名

Microsoft公式の[Tauri向けWinApp CLIガイド](https://learn.microsoft.com/ja-jp/windows/apps/dev-tools/winapp-cli/guides/tauri)どおり、Microsoft Storeへ提出するMSIXは事前署名しない。Microsoft Storeがcertification後にpackageを署名する。

repository rootにlocal PFXが存在しても、Store提出用package commandはPFX、password、private key、certificate store、SignToolを読み書きしない。MSIXのローカル動作検証には、公式ガイドに従って別の開発証明書を生成する。検証用packageはStore候補、CI artifact、Partner Center uploadへ混在させない。

## 開発証明書によるMSIX動作検証

既存の製品用PFXは使わず、Git除外された検証directoryで次を実行する。CLI既定passwordは使い捨て開発証明書のためのもので、配布鍵として利用しない。

```powershell
winapp cert generate --manifest dist/microsoft-store/AppxManifest.xml --output test-results/kukuri/issue-1190-devcert/devcert.pfx --valid-days 7 --if-exists skip --export-cer
winapp pack dist/microsoft-store/staging --manifest dist/microsoft-store/AppxManifest.xml --executable kukuri.exe --cert test-results/kukuri/issue-1190-devcert/devcert.pfx --output test-results/kukuri/issue-1190-devcert/kukuri-test.msix
```

続いて管理者PowerShellで`winapp cert install <devcert.pfxの絶対path>`を実行し、通常ユーザーで`Add-AppxPackage <kukuri-test.msixの絶対path>`を実行する。署名付きMSIXの起動、deep link、通知、restart、update、データ保持を検証する。証明書登録／解除対象は、この工程で生成した証明書のthumbprintで特定する。利用者の既存証明書と実データを削除しない。Store用unsigned candidateと検証用signed packageのpayload一致も照合する。

## app dataとDirect版の共存

Tauriの論理app data pathは`%APPDATA%\app.kukuri.desktop`。ただし既存roaming directoryの存在だけでは、installed MSIXの実際の書込先やWindowsによるvirtualizationは判定できない。今回のinstalled MSIX検証は`KUKURI_APP_DATA_DIR`で専用profileを指定したため、既定pathでのNSISとのデータ共有は未確認。development package解除後に既存roaming dataが残ることと、専用profileのMSIX更新時保持は確認済み。

- Store版への切替でdataを自動copy・移行しない。既存accountが見えない場合はdevice backup／restoreを使い、pathの同一性を仮定しない。
- Direct版とStore版を同時起動しない。同じprofile DBを二つのprocessで開かない。
- 切替前に全accountのdevice backupを別の安全な場所へ作る。
- MSIXのuninstallをkukuri data削除手段として扱わない。data削除を目的に`%APPDATA%\app.kukuri.desktop`やpackage containerを手動削除しない。

同一identityの上位Store versionへupgradeするときは、account key、profile、consent、settings、draft、private capability、DBをbefore／afterで照合する。失敗時にapp data削除で回復しない。

## Partner Center

提出候補はclean worktreeから作ったunsigned MSIXに固定し、source SHA、Store version、SHA-256を記録する。

1. unsigned candidateをPartner CenterのStore ID `9NQ18HML4GS3`へuploadする。
2. Partner Center validationのerror／warningを保存し、失敗をoverrideしない。再buildは別candidateとして全検査をやり直す。
3. certification提出とavailability／一般公開を分離し、承認された公開範囲・日時だけを適用する。
4. certification後にStoreから取得したMicrosoft署名済みpackageについて、signature、identity／version、clean install、同一identity update、起動、deep link、OS notification、app data保持、Store update認識を確認する。

Store upload、certification、一般公開は外部状態の異なる操作である。実装PRやplanの承認だけを一般公開の承認として扱わない。

## 回帰検証

```powershell
python scripts/release/test_windows_store_package.py
pwsh -NoProfile -File scripts/release/test-windows-store-updater.ps1
cargo test -p xtask desktop::package_tests
cd apps/desktop
npx pnpm@10.16.1 test -- src/lib/distribution.test.ts src/components/settings/ReleasePanel.update.test.tsx src/shell/DesktopShellPage.updateSchedule.test.tsx
```

加えて`cargo xtask check`、`cargo xtask test`、`cargo xtask e2e-smoke`、既存release contractsを実行する。Store差分を通常`desktop-package`へ混ぜず、NSIS／GitHub updater、Linux AppImage／Deb、CLIの既存成果物を維持する。
