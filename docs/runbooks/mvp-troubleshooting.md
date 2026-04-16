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
