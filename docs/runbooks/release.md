# Release Runbook

## 対象と公開前の判断

Preview tagは`vX.Y.Z-preview.N`。Windows NSIS／updater、Linux AppImage x86_64、CLI x86_64／aarch64を同じsourceから生成する。Linux資材が公開済みかはReleaseのasset一覧を正とし、workflow実装だけで公開済みとしない。

version／tag／source SHAと公開の依頼範囲を先に確定する。実装PRの承認はRelease公開の承認と区別する。既存tag／公開assetの上書き、検証用一時鍵の転用はしない。

GUIはWindows／Linuxとも既存`cargo xtask desktop-package`を使う。Ubuntu 22.04はLinux build基盤で、全Linux環境の保証ではない。確認済み範囲と延期環境は[AppImage作業記録](../progress/2026-09-05-issue-889-linux-appimage.md)、利用方法は[quickstart](./mvp-user-quickstart.md)と[Linux CLI](./linux-cli.md)を参照する。

## 配布用Secrets

- `TAURI_SIGNING_PRIVATE_KEY`: updater署名秘密鍵。
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: 必要な場合だけ設定するpassword。
- `TAURI_UPDATER_PUBLIC_KEY`: 対応する公開鍵。Windows／Linuxで同じ鍵を設定し、build前にTauri configへ反映する。

鍵はrepository外の安全な場所で生成する。

```bash
cd apps/desktop
npx pnpm@10.16.1 tauri signer generate --write-keys <secure-private-key-path>
```

秘密鍵・passwordをlog、artifact、cache、PRへ保存しない。PRのLinux packageはtest署名のみで配布用secretを渡さない。WindowsのOSコード署名はupdater署名とは別で、未設定ならSmartScreen警告をRelease notesに明記する。

## 検証

path別の選定は[REFACTORING.md](../../REFACTORING.md#path別検証マトリクス)。既存必須CIは維持し、同じsource・code・依存・条件で成功した検証を理由なく重複しない。

```bash
cargo xtask release-check v0.1.8-preview.2
python scripts/release/test_release_assets.py
python scripts/release/test_cli_archive.py
python scripts/release/test_native_compliance.py
python scripts/release/test_publish_preview.py
python scripts/release/test_verify_public_preview.py
```

```powershell
./scripts/release/test-create-preview-assets.ps1
./scripts/release/test-create-preview-assets-linux.ps1
./scripts/release/test-updater-signature-wrapper.ps1
```

`Kukuri Release Contracts`は上記にworkflowの構文・guard検査を加える（PyYAML 6.0.3）。ラッパーfixtureはroutingの検査で、実署名検証の代替ではない。

## Workflowとsource固定

1. 承認したsourceに新しいtagを作成・pushする。tag pushは既定でdraftまで。
2. 手動実行なら`Kukuri Release`のworkflow refにも同じtagを選び、入力`tag`を一致させる。`draft`の既定値は`true`。別refのworkflowで過去tagをbuildする入力は拒否する。
3. `validate-release-inputs`がevent／tag／versionを検証し、tagとworkflowのsourceを照合してcommit SHAを一度固定する。後続checkoutはそのSHAを使う。
4. `linux-verify`が既存製品CIを実行。Windows／Linux GUI package、CLI 2archのjobで本体を生成し、source／target／version／SHA-256を`release-package.json`へ記録する。
5. CLIはarchiveから展開した実binaryでschema、専用profileのdaemon起動・status・終了を確認する。aarch64はQEMUで実行し、cross-compileだけを成功条件にしない。HOME／XDGとprofileは一時領域で、GUIのidentityを共有しない。
6. `changelog`が固定sourceからRelease notesを生成し、`release-assets`が4targetの資材を集約する。必須job失敗・欠落・異なるsource／version／鍵・test署名・hash不一致は公開前に拒否する。
7. 同じWindows buildの実Rust verifierで、最終manifestの両platformの実bytesとembedded signatureを検証する。正常bundle受理と1 byte改変拒否の双方が必要。installは行わない。
8. `publish-draft`が完全性と現在のtag SHAを再検証し、draftを作成してuploadする。公開指定でも、全assetのuploadとGitHub SHA-256 digest照合が終わるまで公開しない。
9. 公開指定時は`verify-published`が安定updater URL、checksum／provenance、4本体を取得して候補hashと照合する。失敗は公開後検証未完了として扱う。

GitHub上の`prerelease` flagは既存互換のため`false`、公開時`make_latest=true`を維持する。製品としてはPreviewだが、`prerelease=true`へ変えると既存clientの`/releases/latest/download/latest-preview.json`に出なくなる。

## Release assets

正確な一覧は`release-assets.txt`、各hashは`SHA256SUMS.txt`、source／targetとの対応は`release-provenance.json`。主要資材は次のとおり。

- Windows NSIS／updaterと`.sig`、Linux AppImageと`.sig`、両platformの公開鍵。
- `kukuri-cli_<version>_x86_64-unknown-linux-gnu.tar.gz`とaarch64版（binary、LICENSE、README、THIRD_PARTY_NOTICESを含む）。
- `latest-preview.json`、上記の一覧／checksum／provenance。
- `THIRD_PARTY_NOTICES.md`、Linux native notice／source archiveとnative compliance JSON。
- `RELEASE_NOTES_DRAFT.md`、`manual-smoke-checklist.md`（追加手動試験を一律要求するものではない）。

manifestは`windows-x86_64`と`linux-x86_64`の両entryを持ち、signatureは`.sig`の内容を埋め込む。UTF-8 BOMは禁止。Windows PowerShell 5.1の`Set-Content -Encoding UTF8`で手動上書きせず、`create-preview-assets.ps1 -IncludeLinux -SourceCommit <SHA>`を使う。入力は各jobの`release-package.json`を含むartifactを子directoryへ展開したもの。出力先は空でなければならない。

## Native notice／source

Rust／npm／非code assetの正本は`docs/THIRD_PARTY_NOTICES.md`と`docs/ASSET_MANIFEST.json`。非code assetの追加・変更・削除時は正確なpath／hash／由来／権利／再配布条件を更新し、次を実行する。

```powershell
cargo xtask asset-check
./scripts/release/generate-third-party-notices.ps1
./scripts/release/generate-third-party-notices.ps1 -Check
```

Linux配布署名jobは最終AppImageのruntime inventoryを採取し、`native_compliance.py`で対応するUbuntu sourceをexact versionで取得、DSCのsize／SHA-256と照合する。外側runtimeは`native-runtime-sources.json`の固定source／patch／build recipe／noticeをhash検証し、AppImageのruntime prefixが固定binaryと一致することも確認する。未確定の由来を推定だけで承認しない。

収集は使い捨てCI runnerで`deb-src`を有効にして行う。開発端末のAPT設定を変える必要はない。取得・照合・notice生成の失敗時は配布packageを完成扱いにしない。`--verify-static-only`／`--verify-ubuntu-only`は部品検査で、公開条件を満たした結果ではない。最終archiveには対応source、patch、build／再リンク手順、共有libraryの差し替え手順を含める。詳細は[同梱物の引渡し条件](./linux-appimage-runtime-evidence.md)。

## Changelog

`update-changelog.ps1`はprevious tagから対象tagまでのcommitを分類し、PRリンク付きsectionを作る。`changelog` jobは固定sourceをfull historyでcheckoutし、tag SHAを再照合する。`kukuri-changelog-section` artifactをRelease notesへ埋め込む。

CHANGELOGの記録は別の`chore/changelog-<tag>` branchへ通常pushしてPRを作成する。mainへの直接pushやforce pushはしない。PR作成部分はbest-effortで、失敗しても生成済みRelease notesは失われない。

## 同一候補の再開・公開後確認

upload失敗時は同一runの`kukuri-release-assets`を保持する。`publish_preview.py`は既存draftの本文／既存asset名／digestが候補と一致するときだけ欠落assetを追加する。異なる既存assetは削除・置換しない。完全な公開済みReleaseなら変更せず終了する。再buildすると署名・時刻・archive bytesが変わり得るため、既存draftへ別runの資材を混ぜて再開しない。

承認済み候補をdownloadして再開する例（version／SHA／repositoryは対象へ置換）:

```bash
python scripts/release/publish_preview.py --input <same-run-assets> --tag v0.1.8-preview.2 --repository KingYoSun/kukuri --source <full-SHA> --draft true
```

公開承認後に同じcommandの`--draft false`で公開する。`GH_TOKEN`は環境変数で供給し、引数や記録へ値を書かない。公開前に同一候補の実署名検証成功が必要。build／smoke／署名／完全性の失敗を手動公開で迂回しない。

```powershell
./scripts/release/test-published-updater-signature.ps1 -Tag v0.1.8-preview.2 -Platforms windows-x86_64,linux-x86_64 -InputDir <same-run-assets> -PublicKeyFile <same-run-assets>/windows-x86_64-updater-key.pub
```

```bash
python scripts/release/verify_public_preview.py --input <same-run-assets> --tag v0.1.8-preview.2 --repository KingYoSun/kukuri --source <full-SHA>
```

安定URLは`https://github.com/KingYoSun/kukuri/releases/latest/download/latest-preview.json`。一時redirect URLを設定へ保存しない。CDN未反映なら同じ公開候補への読み取り検証を再実行し、assetを上書きして直さない。大きなnative source archiveは公開前のGitHub upload digestで全件照合済みとし、公開後に同じdownloadを重複しない。

## 既存動作の採用と利用者データ

更新エンジン・保存形式が不変なら#889のinstall／restart・失敗時保持証拠を採用する。Windows 10／11、追加Ubuntu／Debian、全通知・全データの手動matrixを各Releaseで再要求しない。変更影響や具体的な失敗を既存証拠と自動検証で判定できない場合だけ、必要な手動補完と終了条件を相談する。

identity、profile DB、Iroh、CN設定、private capability、通知inboxを保持する。障害時にapp dataを削除して回復を試みない。利用者は`Settings -> Release`から秘密値を除外した診断reportを取得し、Preview feedbackへ添付する。
