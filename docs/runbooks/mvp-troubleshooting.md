# MVP Troubleshooting

## OSが日本語なのに英語で表示される

初回の規約画面にある「Language」から「日本語」を選べます。通常画面では「Control Center → Settings → Language & theme」の先頭にある言語選択を使ってください。切替だけで規約へ同意することはなく、日本語を選べば同梱の日本語正文を読めます。

保存済みの言語はOS設定より優先します。以前の起動で自動的に英語が保存された場合も含み、OSだけを変更しても既存の言語設定は書き換えません。言語が未保存の場合は、OSのUI言語、WebViewの言語候補、英語の順で決めます。Linuxでは起動元の `LANGUAGE` と、`LC_ALL`／`LC_MESSAGES`／`LANG` の優先順位も確認してください。

「保存できませんでした」と表示された場合、そのsessionの表示は切り替わっていますが、再起動後の保持は保証されません。「言語の保存を再試行」を使ってください。同意保存中は表示言語を固定するため、言語選択と保存の再試行は完了または失敗後に操作できます。

## Community Node の見方

見つけるの「規約への同意は保存されています」は同意保存の成功を示し、接続や検索の成功を意味しない。以下の操作を使い分ける。

- 「規約を確認する」: 未同意・撤回・規約更新の場合に対象Nodeの文書を読み、明示的に同意する。初回説明で「あとで」を選んだ場合もここから再開できる。
- 「状態を再確認」: 状態/公開Node情報の取得をやり直す。接続失敗の場合は同意済みNodeの接続も再確認する。再試行時刻が示されている間は待つ。
- 招待コードが必要: 「コミュニティノード設定を開く」から対象Nodeの「招待コード」を確認する。参加禁止など、招待コードでは解決しない拒否は別Nodeの選択やそのNodeの参加条件確認が必要。
- 検索・発見機能が非提供: 接続補助は別機能。検索を提供するNodeを設定・選択する。同意だけで提供機能は増えない。
- 明示選択先で検索停止: 検索文を別Nodeへ勝手に送らないための停止。見つけるのNode選択で別Nodeを選ぶか、「自動選択に戻す」。Node情報が取得できない場合も無断で既定Nodeへ切り替えない。

状態の取得失敗やquery失敗を「結果0件」とは扱わない。検索を実行して成功した場合だけ空結果を表示する。

検索成功で0件のときは、検索先Node、検索した範囲（このNodeが索引する公開トピック全体）、照合対象が索引済み投稿の本文であることを表示する。ユーザー名や公開鍵は投稿本文に含まれない限り一致しない。索引の反映状況はアプリから確認できないため、「まだ索引に反映されていない」「トピックがそのNodeの索引対象外」を可能性として示し、再検索、タイムライン、索引登録申請（トピック一覧）、接続診断、Community Node設定への導線を置く。公開鍵をそのまま検索した場合はそのユーザーを直接開ける。

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

## Linux配布・CLI

- 公開済みLinux成果物の有無はRelease asset一覧で確認する。AppImage／Debはx86_64、CLIはx86_64／aarch64で、別architectureのbinaryを実行しない。
- Debの認証取消／拒否後は再起動や別認証の自動要求をしない。適用失敗時は`dpkg-query -W -f='${db:Status-Status} ${Version}\n' kukuri`で実状態を確認し、[Deb手順](linux-deb.md)に沿って明示回復する。旧版への自動rollbackは保証しない。
- AppImageの実行権限、FUSE／展開実行、X11の前提は[quickstart](./mvp-user-quickstart.md)を確認する。追加Ubuntu／Debianやnative Wayland-onlyは確認済みとしない。
- CLIの接続失敗は、同じ`--profile`のdaemonが動いているか、同意状態、XDG runtime directoryを確認する。[foreground例](./linux-cli.md)はsystemdの導入を要求しない。
- timeout／切断で変更結果が不明ならstatusを確認し、変更要求を無条件に再送しない。GUIのdata directoryをCLIへ共有したり、鍵取得失敗時にidentityを削除したりしない。
- 配布担当者: native source取得／hash検証やasset集約が失敗した候補は公開しない。同一候補の再開とCDN確認は[release runbook](./release.md)に従い、既存assetの上書きで修復しない。

## Device Backup / Restore

- `wrong passphrase or corrupted data` の場合は、入力したパスフレーズとファイルの転送完了を確認する。部分ファイルや改変されたファイルは復元しない。
- `unsupported ... version` の場合は、そのバックアップを作成した kukuri と同じか新しい対応版で復元する。未知版を強制的に展開しない。
- 同じ公開鍵のアカウントがある場合、置換確認なしでは復元しない。置換を選ぶ前に現在の状態も別ファイルへバックアップする。
- 容量不足・復号失敗・DB検証失敗・`Installed`までの切替失敗では、現在のアカウントを残してエラーを表示する。再起動時に`Installing`／`Installed`のjournalがあれば旧アカウントへ自動rollbackする。registry commit後の`Committed`／`AwaitingConsent`は新アカウントの再同意待ちへfinish-forwardし、`Activated`後のcleanup中断は次回起動で完了する。
- 復元後に規約同意と年齢申告が再表示されるのは仕様である。Community Node の文書同意と認証、成人向け表示設定も端末側で再設定する。
- 復元途中でアプリを終了した場合は、次回起動時に未完了の復元を先に回収する。回収に失敗した場合はruntimeを開始せず起動失敗として表示するため、同じbackupから再試行する前に表示されたerrorを確認する。復元したfrontend設定はactivation完了後にだけ適用し、適用または確認応答に失敗した場合は旧設定へ戻して次回起動で再試行する。
- 端末バックアップは旧端末、relay、接続相手、P2Pネットワーク上のコピー削除を行わない。

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
