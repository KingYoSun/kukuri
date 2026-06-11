# MVP Troubleshooting

## Community Node の見方

- `Auto-approve`
  - `yes`: この node は preview happy path で consent を自動承認する
  - `no`: token refresh は自動でも、consent は手動操作が必要
- `Session Phase`
  - `connecting`: node 到達と session 準備を開始
  - `authenticating`: challenge / verify を実行中
  - `accepting`: required consent を処理中
  - `refreshing`: bootstrap metadata, connectivity URL, seed peer を更新中
  - `ready`: current session で利用可能
  - `retrying`: backoff 中。`Retry After` 以降に再試行する
- `Retry After`
  - 次回の自動再試行予定時刻
- `Connectivity URLs`
  - current session に反映済みの assist URL
- `Last Error`
  - 直近の auth / consent / metadata refresh 失敗理由

## よくある状態

### `ready` まで行かない

- `Last Error` を確認する
- `Retry After` が出ているなら待ってから再確認する
- 一度ウィンドウをフォーカスし直して即時再試行を促す

### `authenticating` と `retrying` を繰り返す

- node の `base URL` が正しいか確認する
- `auto_approve=no` の node や preview 既定外の node では、auth endpoint と consent endpoint が有効か確認する

### `accepting` で止まる

- `auto_approve=yes` の node で起きる場合は `Last Error` を確認する
- `auto_approve=no` の node は manual `Accept` を使う

### `restart required` が出る

- まず `Refresh` を試す
- それでも消えない場合だけ app restart を fallback にする

## Manual Actions

- `Authenticate`: token を明示的に取り直す
- `Consents`: 現在の consent 状態を再取得する
- `Accept`: required consent を明示的に受諾する
- `Refresh`: bootstrap metadata と connectivity assist を再取得する
- `Clear Token`: 該当 node の token を破棄し、次回 auth をやり直す

preview の primary UX は自動処理ですが、上の操作は troubleshooting 用に残しています。

## Updates

- `Settings -> Release -> Check` が失敗する場合は、ネットワーク到達性と GitHub Releases の `latest-preview.json` を確認する。
- `Install` が失敗する場合は、同じ release の updater bundle と `.sig` が揃っているか確認する。
- 署名検証に失敗した更新はインストールしない。release asset の差し替えや誤った signing key を疑う。
- 更新後にデータが消えたように見える場合は、別の Windows user profile、別の app data dir、または keyring fallback の使用有無を確認する。

## Diagnostics

- `Settings -> Release -> Copy Report` で GitHub issue に貼れる診断レポートを作る。
- 既定のレポートには secret key、auth token、private channel secret、invite/share token、DM 本文、ローカル DB path を含めない。
- `Export` は `kukuri-diagnostics.txt` を作成する。

## Data Safety

- `Settings -> Release` includes the release runbook and third-party notices.
- Reinstall or migration failures should be reported with diagnostics and must not silently clear local data.
- If state appears missing after update, confirm the Windows user profile, app data directory, and keyring fallback path before resetting anything.

## Installer Notes

- 初回 preview で Windows code signing が未設定の場合、SmartScreen warning は想定内として release note に明記する。
- 未署名 preview の場合も、updater bundle の Tauri signature は必須とする。
