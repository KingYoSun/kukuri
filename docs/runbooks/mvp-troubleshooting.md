# MVP Troubleshooting

## Community Node の見方

- `Session Phase`
  - `connecting`: node 到達と session 準備を開始
  - `authenticating`: challenge / verify を実行中
  - `accepting`: ローカルで明示同意済みの required 文書を server の同意記録へ同期中
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
- Node の auth endpoint と consent endpoint が有効か確認する

### `accepting` で止まる

- `Last Error` を確認する
- `Consents` を開き、表示された現行版の文書へ明示的に同意済みか確認する

### `restart required` が出る

- まず `Refresh` を試す
- それでも消えない場合だけ app restart を fallback にする

## Manual Actions

- `Authenticate`: ローカル同意済み Node の token を明示的に取り直す
- `Consents`: Node の公開文書を表示し、required 文書への同意・再同意・撤回を行う
- `Refresh`: bootstrap metadata と connectivity assist を再取得する
- `Clear Token`: 該当 node の token を破棄し、次回 auth をやり直す

preview の primary UX は明示同意後のセッション確立・維持を自動処理しますが、上の操作は troubleshooting 用に残しています。

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
