# Issue #1190 MSIX／Microsoft Store 作業記録

## Scope

- Scope revision: `2026-09-19-v1`
- 基準 commit: `f3d481f0fda732941033275510e6598eda93f2ef`
- リスク区分: C
- Issue: [#1190](https://github.com/kukuri-app/kukuri/issues/1190)
- 実装 branch: `codex/issue-1190-msix-store`

Partner Centerの公開identityは`KingYoSun.kukuri`、Publisher `CN=33EB763C-4859-4E44-886F-1784E16DD6D5`、Publisher display name `KingYoSun`、PFN `KingYoSun.kukuri_p8fpcaf1kx88g`、Store ID `9NQ18HML4GS3`。製品はpackage未提出のdraftだったため、初回Store package versionを`1.0.0.0`へ固定した。

## 実装

- Microsoft WinApp CLI 0.6.1、固定manifest、x64 Tauri `--no-bundle` outputからunsigned MSIXを生成する`cargo xtask windows-store-package`を追加した。
- `Package.appxmanifest`にPartner Center identity、SHA-256 package integrity、Desktop full-trust entry、最小visual assets、`kukuri:` protocolを固定した。
- package scriptはclean output、source SHA、WinApp CLI version、payload allowlist、packed identity、block map SHA-256、MSIX hash／bytesを検査して`store-package.json`へ記録する。
- Store upload candidateはcertificate optionなしのunsigned MSIX。local-test copyだけをPFXから一時importしたthumbprintで署名し、passwordをargvへ渡さず、成功／失敗のどちらでも新規import certificateをcleanupする。
- Store buildはCargo featureとVite distribution flagの両方で区別する。frontendは起動時／30分timer／Settings updater操作とGitHub外部送信表示を出さず、backendはupdate check／download／install／restart gateをnetwork／installer sinkより前に拒否する。
- Store版の更新はMicrosoft Store／Windowsへ完全委譲し、`StoreContext`等の別app内updaterは追加しない。Direct／NSIS・Linux版は既存GitHub updaterを維持する。
- unsigned packageをsecretなしでbuildするpath限定のWindows CIと、identity／manifest／署名境界／workflow contractを追加した。
- release／quickstart／legal data-flow／privacy／external-transmission／三言語UIをdistribution差分へ同期した。外部送信を増やさない補記なのでlegal bundle version 8は変更していない。

## 実packageの観測

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
- package container `%LOCALAPPDATA%\Packages\KingYoSun.kukuri_p8fpcaf1kx88g`は作成されたが、kukuriのaccount／DB／consent正本はDirect版と同じ`%APPDATA%\app.kukuri.desktop`だった。
- exact PIDを終了して`winapp unregister --manifest ...`を実行し、対象development packageだけが消え、既存roaming app dataが残ることを確認した。

### PFX negative boundary

- `KUKURI_MSIX_CERT_PASSWORD`なしの`--sign-local`は`Import-PfxCertificate`で非0終了した。
- unsigned MSIXとunsigned provenanceは保持し、signed local-test fileは残さず、certificate storeへ新規certificateを残さないことを確認した。
- 実PFX署名、signed install／upgrade、WACKはpasswordをprocess環境へmask入力したcandidateで別途行う。password自体は本記録、log、Gitへ残さない。

## Validation

| 対象 | 結果 |
| --- | --- |
| `python scripts/release/test_windows_store_package.py` | 4 tests PASS |
| `python scripts/release/test_release_workflow.py` | 14 tests PASS（isolated PyYAML 6.0.3） |
| targeted frontend（distribution／ReleasePanel／scheduler／i18n parity） | 83 tests PASS |
| `cargo test -p xtask desktop::package_tests` | 6 tests PASS |
| default／`microsoft-store` Tauri `cargo check` | PASS |
| `cargo xtask check` | PASS（fmt、workspace clippy、Tauri check、lint、typecheck） |
| `cargo xtask test` | PASS（Rust 1038、doctest、frontend 1906） |
| `cargo xtask e2e-smoke` | PASS（`desktop_smoke_post_persist` 6 steps） |
| `cargo xtask desktop-ui-check` | PASS（frontend 1906、Storybook、browser 366、visual reachability 42） |
| `git diff --check` | PASS |

WindowsのTauri unit test executableを直接起動するtargeted commandは、test本体へ入る前に既存native DLL entrypoint不足の`STATUS_ENTRYPOINT_NOT_FOUND`で終了した。default／Store featureのcompileは成功し、frontend／xtask／package contractsとCIで実行可能なtestを根拠にする。この環境固有失敗をtest PASSとして数えていない。

## 残工程

- clean PR headからunsigned candidateを再buildし、source SHA／hashを固定する。
- PFX passwordをchat／argvへ出さずprocess環境へ注入し、local-test copyの実署名、temporary certificate cleanup、install／same-identity upgrade、WACKを行う。
- 区分Cの独立監査、必須CI、PR merge後tree照合。
- 同一unsigned candidateをPartner Centerへuploadしてvalidationする。certification提出とavailability／一般公開は外部状態を分離して記録する。
