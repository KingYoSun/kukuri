# #1180 Kukuri Release の Namespace 移行

Issue: https://github.com/kukuri-app/kukuri/issues/1180 ／ リスク区分 C（配布用の署名鍵を受け取る job の実行場所が変わる）

## 判断（2026-09-19 ユーザー）

- build / verify と署名する job を Namespace へ移す。#1148 の「配布鍵を渡す run は GitHub-hosted」を改める。
- release の経路では cache を使わない。Linux の job は Ubuntu 22.04 の `namespace-profile-kukuri-linux-release`（8 vCPU / 16 GB、Cache Volume なし）で動かす。
- 独立監査の指摘（Cache Volume 付きの `namespace-profile-kukuri-win` では PR run と tool cache 等を共有する）を受け、ユーザーが `namespace-profile-kukuri-win-release`（Windows Server 2022、8 vCPU / 16 GB、Cache Volume なし）を作成した。windows-package はこちらで動かす。
- windows-package / linux-package は linux-verify を待たずに並行実行する。公開は全 job の成功が条件のまま。
- 検証は v0.2.8-preview.1 の実 release で行ってよい。

規則の正本は [release runbook の「Runnerとcache」](../runbooks/release.md)。

## 変更前の実測（run 35321581063、v0.2.7-preview.1、全体 2 時間 31 分）

| job | 所要 | 備考 |
| --- | --- | --- |
| validate-release-inputs | 4 分 30 秒 | |
| linux-verify | 47 分 40 秒 | 開始前に runner 待ち 28 分 |
| windows-package | 65 分 50 秒 | linux-verify の完了後に開始。`Release version gate` 11 分、`Post Cache Rust` 7 分 |
| linux-package | 25 分 | windows と並行 |
| 末尾 4 job | 4 分 | |

## 変更後の実測

v0.2.8-preview.1 の release run で記録する。
