# Issue #1190 MSIX／Microsoft Store 作業記録

## Scope

- Scope revision: `2026-09-19-v3`
- 基準 commit: `f3d481f0fda732941033275510e6598eda93f2ef`
- リスク区分: C
- Issue: [#1190](https://github.com/kukuri-app/kukuri/issues/1190)
- 実装 branch: `codex/issue-1190-msix-store`

Partner Centerの公開identityは`KingYoSun.kukuri`、Publisher `CN=33EB763C-4859-4E44-886F-1784E16DD6D5`、Publisher display name `KingYoSun`、PFN `KingYoSun.kukuri_p8fpcaf1kx88g`、Store ID `9NQ18HML4GS3`。製品はpackage未提出のdraftだったため、初回Store package versionを`1.0.0.0`へ固定した。

## 実装

- Microsoft WinApp CLI 0.6.1、固定manifest、x64 Tauri `--no-bundle` outputからunsigned MSIXを生成する`cargo xtask windows-store-package`を追加した。
- `Package.appxmanifest`にPartner Center identity、SHA-256 package integrity、Desktop full-trust entry、最小visual assets、`kukuri:` protocolを固定した。
- package scriptはclean output、source SHA、WinApp CLI version、payload allowlist、packed identity、block map SHA-256、MSIX hash／bytesを検査して`store-package.json`へ記録する。
- Store upload candidateはcertificate optionなしのunsigned MSIX。2026-09-19のユーザー判断でMicrosoft公式Tauriガイドへ統一し、PFX／SignTool／certificate storeをStore package経路から除外した。Microsoft Storeがcertification後に署名する。
- Store buildはCargo featureとVite distribution flagの両方で区別する。frontendは起動時／30分timer／Settings updater操作とGitHub外部送信表示を出さず、backendはupdate check／download／install／restart gateをnetwork／installer sinkより前に拒否する。
- Store版の更新はMicrosoft Store／Windowsへ完全委譲し、`StoreContext`等の別app内updaterは追加しない。Direct／NSIS・Linux版は既存GitHub updaterを維持する。
- unsigned packageをsecretなしでbuildするpath限定のWindows CIと、identity／manifest／unsigned-only境界／workflow contractを追加した。
- release／quickstart／legal data-flow／privacy／external-transmission／三言語UIをdistribution差分へ同期した。外部送信を増やさない補記なのでlegal bundle version 8は変更していない。

## 実packageの観測

### 公式開発証明書によるinstalled MSIX検証（v3）

- ユーザーの追加指示により、v2で省いた署名付きMSIXのローカル検証を復帰した。Store提出用buildはunsigned-onlyのまま、検証用は公式`winapp cert generate`／`winapp pack --cert`／`winapp cert install`を利用する。既存の`code_sign_certificate.pfx`は使用しない。
- 有効期間7日の使い捨て開発証明書をGit除外の`test-results/kukuri/issue-1190-devcert`に生成した。ユーザーが管理者権限で`winapp cert install`を実行し、`LocalMachine/TrustedPeople`への登録を確認した。
- `Add-AppxPackage`で`1.0.0.0`を正常installした。`IsDevelopmentMode=False`、`Status=Ok`、PFN=`KingYoSun.kukuri_p8fpcaf1kx88g`。WindowsApps配下の実行ファイルから画面が起動することを確認した。
- `KUKURI_APP_DATA_DIR`に専用`test-results/kukuri/issue-1190-installed-profile`を指定した。既存ユーザーデータを移行・削除せず試験した。
- 検証用manifestだけversionを`1.0.1.0`へ変更し、同じ実行ファイルと開発証明書で再packした。上位版を`Add-AppxPackage`してversionと`Status=Ok`を確認した。
- 停止後／更新後・再起動前の専用profile全38ファイルのSHA-256が一致した。更新後の実機画面でプロフィール`test`とタイムライン表示を確認した。ネットワーク同期開始後はDBが変わり得るため、再起動後の全ファイル不変までは主張しない。
- 実機のリリース設定でMicrosoft Store管理の説明が表示され、アプリ内の更新確認／インストールボタンがないことを確認した。
- OS通知の実配信、Storeサーバーによる更新配信、Partner Center validationは本観測だけでは検証済みにしない。

### WinApp CLI package

- `winapp --version`: `0.6.1`
- 最初のpackは、manifestが`Square310x310Logo`だけを指定していたため、WinApp CLI／MakeAppxが`Wide310x150Logo`必須として`0x80080204`で拒否した。任意large tileを削除した最小manifestへ直し、再packに成功した。
- development candidate（dirty worktree）の例: `KingYoSun.kukuri_1.0.0.0_x64.msix`、SHA-256 `57d6c410aa73b6a8591457d36c66a16bea1508aa1430bee76b67db3732997f9e`。これは実装中の例でありPartner Center提出candidateではない。
- package展開payload: `AppxManifest.xml`、`AppxBlockMap.xml`、`[Content_Types].xml`、`kukuri.exe`、3 icons、PRI 3 filesのみ。
- packed identity: `KingYoSun.kukuri | CN=33EB763C-4859-4E44-886F-1784E16DD6D5 | 1.0.0.0 | x64`。
- provenance hashと実file hash一致。`SignTool verify /pa`は`No signature found`で非0となり、Store candidateが意図どおりunsignedであることを確認した。

### loose package identity

- `winapp run --detach --json`はAUMID `KingYoSun.kukuri_p8fpcaf1kx88g!kukuri`と`kukuri.exe` processを返し、processは応答状態だった。
- `kukuri:topic:issue-1190-smoke`をactivationしてもprocess件数は1のままだった。
- development packageは`KingYoSun.kukuri_1.0.0.0_x64__p8fpcaf1kx88g`、`IsDevelopmentMode=True`で登録された。
- package containerと既存`%APPDATA%\app.kukuri.desktop`の存在を観測した。ただしこれだけではinstalled MSIXの実書込先やvirtualizationを証明できない。専用profile指定の検証とは区別し、既定pathでのNSISとのデータ共有は未確認とする。
- exact PIDを終了して`winapp unregister --manifest ...`を実行し、対象development packageだけが消え、既存roaming app dataが残ることを確認した。

### Superseded: local PFX署名調査

- Scope revision v1ではlocal PFX署名も検討したが、Microsoft公式ガイド上、Store提出用MSIXは署名不要でStoreが再署名する。ユーザー判断によりv2でこの経路を削除した。
- 調査中に一時importしたcertificateは`CurrentUser\My`、`TrustedPeople`、`Root`のすべてで残存0件を確認した。PFX／passwordはtracked file、PR、CI、artifactへ含めていない。
- `code_sign_certificate.pfx`はrootでGit除外されたまま保持し、#1190のbuild／validation／uploadから参照しない。

## Validation

### 独立監査後の追加検証

- 出力先guard単体の非破壊testで、既存directoryを誤って受理する旧実装のFAILを再現。修正後は`dist`内の新規directoryだけを受理し、既存出力やrepository内の他directoryを拒否する。既存出力の再帰削除は撤去した。
- `--skip-build`を撤去し、Store専用`target/microsoft-store`で常にbuildする。build前後のcommit／worktreeを照合する。PowerShellの未知optionもparameter bindingで拒否する。
- Windows library test実行時の`STATUS_ENTRYPOINT_NOT_FOUND`はCommon Controls v6 manifestの欠落と判明。test executableのcopyへSDK `mt.exe`でmanifestを埋めたうえで、Store featureのupdater test4件が成功した。製品binaryの変更やassertionの無効化は行っていない。
- Store featureのcheck／download／install／restart gateを、updater plugin／session stateすら存在しないmock appで2回ずつ呼び、全てがStore管理エラーで先に拒否されるtestを追加。Direct専用state testはDirect featureで継続する。
- 独立した`KingYoSun.kukuri.NotificationSmoke`のloose packageに実際のRust notification test executableを登録して`winapp run`から実行した。修正前のNSIS ID指定はpackageの通知履歴への到達がFAIL、修正後の引数なし`CreateToastNotifier()`ではPASS。表示overlayや通知クリック後のpost解決までの確認とは区別する。
- MSIXではprotocol登録をmanifestへ委譲し、起動時に通常NSIS版のHKCU登録を上書きする`register_all`を呼ばない。

| 対象 | 結果 |
| --- | --- |
| `python scripts/release/test_windows_store_package.py` | 6 tests PASS |
| `python scripts/release/test_release_workflow.py` | 14 tests PASS（isolated PyYAML 6.0.3） |
| targeted frontend（distribution／ReleasePanel／scheduler／i18n parity） | 83 tests PASS |
| `cargo test -p xtask desktop::package_tests` | 6 tests PASS |
| default／`microsoft-store` Tauri `cargo check` | PASS |
| `cargo xtask check` | PASS（fmt、workspace clippy、Tauri check、lint、typecheck） |
| `cargo xtask test` | PASS（Rust 1038、doctest、frontend 1906） |
| `cargo xtask e2e-smoke` | PASS（`desktop_smoke_post_persist` 6 steps） |
| `cargo xtask desktop-ui-check` | PASS（frontend 1906、Storybook、browser 366、visual reachability 42） |
| `git diff --check` | PASS |

WindowsのTauri unit test executableの直接起動は当初`STATUS_ENTRYPOINT_NOT_FOUND`で失敗したが、上記のCommon Controls v6 manifestを付与する`test-windows-store-updater.ps1`によりDirect 5件／Store 4件の実行がPASSした。CIにも同じ検証入口を追加した。

### 最終candidateと追加更新試験

- clean source `0aa328ac11d11d894c4014cf6aeeefd7a21fa5a8`からStore専用targetで再buildしたunsigned candidateは`dist/microsoft-store-0aa328ac/KingYoSun.kukuri_1.0.0.0_x64.msix`。SHA-256は`47e4026a35988d4a61c90fee774b49ff8a9bfa78a0ea8f0f554a7ede16b117b1`、35,591,507 bytes。
- 同じstagingを検証専用version `1.0.2.0`で開発署名し、installed `1.0.1.0`から更新。`Status=Ok`、専用profileの全38ファイルのSHA-256が更新前後で一致し、更新後processの起動に成功した。
- 既存HKCUの`kukuri:` handlerはDirect開発版を指していた。process件数が1のままだった観測だけをStoreへのURI配送の証明とは扱わず、既存の利用者protocol選択は変更していない。
- ユーザーがPartner Centerへのuploadを手動で担当する。提出対象は上記unsigned candidateで、開発署名付き`1.0.2.0`ではない。

## 残工程

### 透過shell iconの修正

- ユーザーのWindows実機で青い背景plateを観測。元PNGとmanifestは透明背景だったが、targetsize／unplated／lightunplatedが欠落していた。既存ロゴの意匠・app内UI・NSISを変えず、Store stagingだけに14サイズ×3 variantsを追加する。
- 寸法、透明corner、非空画像、42資産の名前を検査するtestを先に追加しFAILを確認。生成処理追加後にpackage contracts全7件がPASSした。
- 旧candidate `47e4026a...`はこの表示修正前のため、提出候補としては失効。修正後のclean buildと実機確認の結果をPRに記録する。

- 固定unsigned candidateのPartner Center validation結果を記録する。
- 区分Cの独立監査、必須CI、PR merge後tree照合。
- ユーザーによるupload後、certification後のMicrosoft署名済みpackageでinstall／update／activationを確認する。certification提出とavailability／一般公開は外部状態を分離して記録する。
