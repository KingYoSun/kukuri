# Linux Debの導入・更新

対象はGUIのamd64（x86_64）Deb。公開済みかは対象Preview Releaseのasset一覧で確認する。#905の実装・CIだけでは公開済みとしない。build／自動配置検査はUbuntu 22.04、GUI実機検証の結果は[#905作業記録](../progress/2026-09-07-issue-905-linux-deb-updater.md)を正とする。追加distribution、arm64 GUI、APT repositoryは対象外。

## 初回導入と起動

公開元・`SHA256SUMS`を確認する。以下の`X.Y.Z`は選んだReleaseの実versionへ置き換える。署名は同ReleaseのDeb entry・`.deb.sig`・配布公開鍵を[公開検査手順](release.md)で照合する。AppImageの署名をDebへ流用しない。

```bash
sudo apt install ./kukuri_X.Y.Z_amd64.deb
dpkg-query -W -f='${Package} ${Version} ${Architecture} ${db:Status-Status}\n' kukuri
kukuri-desktop-tauri
```

GUIは通常のログインユーザーで起動し、`sudo`を付けない。アプリ一覧のkukuriからも起動できる。Debは`/usr/bin/kukuri-desktop-tauri`、desktop entry、icon、deep-link登録、`/usr/share/doc/kukuri/`のnoticeを配置する。WebKitGTK／GTK／AppIndicator等は宣言したシステム依存であり、AppImageの同梱runtimeとは異なる。CLIは同梱しない。

## アプリ内更新

Release画面で更新を確認し、download完了後に「再起動して更新」を選ぶ。定期確認も同じ署名検証経路を使う。Deb版は`linux-x86_64-deb`だけを選び、entry欠落時にAppImageへ切り替えない。署名検証済みbytesだけをOSのpackage managerへ渡す。

OSの認証画面で適用を承認するとpackage更新後にアプリが正常終了・非root再起動する。passwordはOS画面へ入力し、kukuriやチャットへ渡さない。取消・拒否・認証agent不在では別の認証方法へ切り替えず停止し、自動再起動しない。再試行は利用者が状態を確認して明示的に行う。

## 失敗時の確認・手動回復

```bash
dpkg-query -W -f='${db:Status-Status} ${Version}\n' kukuri
```

署名・形式不一致ではinstallしない。認証後にpackage適用が失敗した場合は、旧版が完全に残るとは保証しない。packageがunpacked／half-configured等になり得るため、上記の状態とOSのpackage診断を確認する。失敗を更新成功として再起動しない。

信頼できる同じ／新しいversionのDebを取得して署名・checksumを照合し、kukuriを終了した後に`sudo apt install ./kukuri_X.Y.Z_amd64.deb`で明示的に回復する。同じ版の再配置は`sudo apt install --reinstall ./kukuri_X.Y.Z_amd64.deb`を使う。依存解決や認証agentの問題はOS側で解消し、認証を無効化する設定やroot GUI起動で回避しない。

## 削除とユーザーデータ

```bash
sudo apt remove kukuri
```

install／reinstall／removeはGUIのidentifier・保存先・identity・鍵・DBを変更・削除しない。AppImageと同じGUI保存先を使うため、両形式を同時起動しない。CLI専用profileへの移行は行わない。ユーザーデータ削除はpackage削除とは別操作であり、本手順には含めない。
