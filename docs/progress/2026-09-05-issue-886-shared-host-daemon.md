# Issue #886 共通client hostとLinux常駐プロセス

## Scope

Scope revisionは`2026-09-04-issue-886-shared-host-daemon-v1`。公開command protocolとdomain command mappingは#887、#888が所有するため、本Issueでは共通host lifecycle、CLI profile、単一所有、認証済みUnix socket、購読期待状態、event供給点、graceful shutdownを固定する。

## 実装結果

- `kukuri-desktop-runtime::ClientHost`をGUIと常駐プロセスのruntime ownerとし、同意／年齢gate、account初期化、既定Community Node、scheduler、observer、account switch、runtime event、shutdownを共通化した。
- GUI／CLI kind markerとprofile leaseを永続state読取り前に取得する。CLI profile pathは`$XDG_DATA_HOME`または`$HOME/.local/share`配下へ分離し、同profileの二重owner、selector競合、kind不一致、未分類legacy directoryを型付きerrorで拒否する。`KUKURI_APP_DATA_DIR`単独指定ではpath digestからsocket用profile名を導出し、異なるdirectoryのlocal socketを分離する。
- `kukuri-cli daemon run|start|stop|status`とsystemd user instance unitを追加した。foreground daemonは`$XDG_RUNTIME_DIR`だけを使用し、runtime directory 0700、socket 0600、peer UID一致を要求し、TCP listenerを作らない。
- 同意未成立時はprofile leaseとlocal socket以外を開始しない。同意成立後だけ共通hostを構築する。Secret Serviceを利用できない新規profileは既存の0600 file fallbackを使用できる。
- account別のversion付き`kukuri.subscriptions.json`をatomic writeし、topic／channelの購読期待状態をhost起動とaccount切替時に復元する。破損、未知version、永続化失敗、購読再開失敗は型付きerrorとする。
- `ClientHost`のevent receiverはremote runtime event、notification更新、直近のsync statusを継続配信する。SIGINT／SIGTERMとdesktop tray終了は共通host shutdownの完了を待つ。
- device backupへ購読期待状態を含め、profile lock、marker、Unix socket、endpoint secretは含めない。

## 固定surfaceとtransitionの対応

| ID | 実装／test |
| --- | --- |
| `INV-1` | `ClientHost::start_if_consented`、`switch_account`、Tauri adapter、既存GUI regression |
| `INV-2` | `ProfileLease`、`kukuri-cli daemon run`、Linux process test |
| `INV-3` | Unix socket permission、`peer_cred` UID検証、TCP listener 0件 test |
| `INV-4` | `DesiredSubscription` store、host restart／identity test、backup restore validation |
| `INV-5` | host operation guard、冪等shutdown、SIGTERM restart test、desktop tray shutdown |
| `TR-1` | 未同意時にaccount registryを生成しないhost test |
| `TR-2` | 同profile二重owner testとLinux duplicate daemon test |
| `TR-3` | 別profile同時lease／daemon test |
| `TR-4` | 同一identityと購読期待状態のhost restart test |
| `TR-5` | SIGTERM後のsocket cleanup／再起動testと冪等host shutdown test |
| `TR-6` | profile selector／kind error、既存identity fallback test、runbook |

## Validation

ローカルでは`cargo xtask check`、`cargo xtask test`（`rust-test`を含む）、`cargo xtask tauri-check`、`cargo xtask e2e-smoke`、`community_node_public_connectivity`、`desktop_device_backup_restore`、`cargo xtask oversized-files`、`git diff --check`がPASSした。Linuxでは`cargo test -p kukuri-cli`を実行し、同意済みruntimeのprocess testを含む7件がPASSした。PR headでは独立監査により固定inventory、transition、sensitive sink callerを再構築し、GitHub CIと併せて最終判定する。
