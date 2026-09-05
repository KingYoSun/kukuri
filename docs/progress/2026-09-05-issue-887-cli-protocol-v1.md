# Issue #887 CLI protocol v1と安全なI/O

## 後続の仕様改訂

以下は #887 完了時の実装・検証記録である。2026-09-05のユーザー決定により、
入力間の重複排除、操作キーの必須化、永続台帳、復元後の保護期間は撤回された。
この部分の仕様は改訂後の [ADR 0049](../adr/0049-linux-gui-cli-control-plane.md) を正とし、
実装移行と検証は #888 で行う。当時のPASSを改訂後の実装完了の証拠として流用しない。

## Scope

Scope revisionは`2026-09-04-issue-887-cli-protocol-v1`。#886の共通`ClientHost`と認証済みUnix socketを利用し、版管理されたprotocol、command登録簿、dispatcher、安全なsecret I/O、durable idempotency ledgerを実装する。全domain commandのmappingは#888が所有するため、本Issueのproduction登録簿はprotocol introspection、client status、event streamに限定する。

## 実装結果

- `kukuri-cli`をlibrary／binary構成にし、protocol v1のrequest、success、error、stream envelope、安定error／exit code、JSON Schema、pagination、timeout、frame上限を共通型へ集約した。
- `CommandRegistry`の一つのentryにhandler、input/output schema、read／write／destructive、secret input/output、idempotency、stream、guard metadataを保持する。`protocol.schema`と`protocol.commands`はdispatcherと同じ登録簿だけを参照する。
- dispatcherはdecode／schema validation、profile／version、command lookup、product guard、secret frame受信、idempotency claim、handlerの順で処理する。未同意時の`events.watch`はsecretを含むbodyの受信前に拒否し、`client.status`とintrospectionはruntimeを開始せず利用できる。domain guard evaluatorは注入可能な境界を持つ。
- CLIの`call`は通常入力をstdin／owner-only file／FDから上限付きで受け、成功・失敗ともstdoutへJSONを一つ出す。event streamだけをNDJSONとし、responseの`more`で継続を判定する。SIGINT時も`interrupted` envelopeと終了code 130を返す。daemon不在、protocol mismatch、usage、invalid input、timeoutも型付きerrorへ変換する。
- daemonはSIGINT／SIGTERM receiverをreadiness通知前に登録し、通知直後の停止要求も取りこぼさずgraceful shutdownする。
- secretは通常JSON payloadとは別の長さ付きframeを通し、headerのpreflightに成功した相関付き受信許可後だけ送信する。専用の`SecretInput`／`SecretOutput`は通常の`Serialize`を実装せず、`Debug`を常にredactする。sourceはstdin／FD／owner-only file、sinkは事前宣言した標準FD以外のFD／新規0600 fileに限定する。#887当時のsecret出力のread-only制限は、#888で招待exportが鍵世代を更新する既存挙動に合わせて撤去する。secret-bearing handlerのerror detailを外部へ出さず、secret outputと同じbytesを通常JSONへ含めず、stdout／stderrへ平文を出さない。
- profile／account／command／UUIDv7 key／canonical payload digestを結ぶ専用SQLite ledgerを追加した。mutationを直列化してin-progressを先にcommitし、同一payloadはsanitized resultを再生、異payloadは`idempotency_conflict`、未完了は`operation_outcome_unknown`とし、自動再実行しない。
- secretを含むpayload照合はledger固有saltのkeyed BLAKE3 digestだけを保存する。terminal recordは30日、profile／accountごとに最大10,000件とし、上限到達時は最古の完了済みrecordだけを整理する。未確定recordは自動削除しない。
- 存在するidempotency ledgerをdevice backupへ含め、台帳を含まない旧backupでもrestore validation時に台帳とrestore markerを生成する。復元時刻から5分のclock skewまでに作成された欠落keyは新規mutationとして扱わず、それより未来のkeyも拒否する。socket、lock、PID、session、token、endpoint secretは従来どおりbackup対象外である。

## 固定surfaceとtransitionの対応

| ID | 実装／test |
| --- | --- |
| `INV-1` | `CallArgs`、bounded JSON／secret frame、stdin／file／FD、stdout envelope、exit-code test |
| `INV-2` | `CommandRegistry`、`Dispatcher`、guard-before-handler test、Unix socket process test |
| `INV-3` | `SecretInput`／`SecretOutput`、専用frame、0600 source/sink、sentinel leak test |
| `INV-4` | `IdempotencyLedger`、mutation mutex、keyed canonical digest、replay／conflict／crash／restore test |
| `INV-5` | `protocol.schema`／`protocol.commands`、registry uniqueness／pagination／metadata test |
| `TR-1` | success／error envelope、invalid JSON／schema／command test |
| `TR-2` | ready host statusとfixture mutation success test |
| `TR-3` | consent guard拒否時のmutation counter 0件 test |
| `TR-4` | durable same-key replay test |
| `TR-5` | different-payload conflict、restart中in-progress、restore marker test |
| `TR-6` | daemon unavailable、protocol mismatch、実timeout、SIGINT、oversized／partial frame、接続上限／backpressure処理 |

## #888への境界

#888は既存Tauri commandを複製せず、`CommandRegistration`へdomain handlerとv1対応subset内のDTO schemaを追加し、注入可能な非同期`GuardEvaluator`を既存product guardへ接続する。write／destructive commandはUUIDv7 idempotency keyを必須とし、secret-bearing commandは専用secret frameを使う。`apps/desktop/src-tauri/src/lib.rs`の登録と既存GUI result/error semanticsは#887では変更していない。

## Validation

- Windows: `cargo test -p kukuri-cli --lib`（21件成功）
- Windows: `cargo test -p kukuri-desktop-runtime host::idempotency --lib`（9件成功）
- Windows: `cargo test -p kukuri-desktop-runtime encrypted_device_backup_restores_one_account_as_one_file --lib`（成功）
- Linux: `cargo test -p kukuri-cli`（38件成功）
- Linux: readiness直後のSIGTERM／再起動境界test（10回連続成功）
- Linux: 64接続の購読開始を確認した接続上限／backpressure境界test（5回連続成功）
- Windows／Linux: 対象crateの`cargo clippy --all-targets -- -D warnings`（成功）
- `cargo xtask check`、`cargo xtask test`、`cargo xtask e2e-smoke`、`cargo xtask oversized-files`、`git diff --check`（成功）
- Risk Cの独立監査は、secret、guard順序、schema／dispatcher整合、接続上限、冪等性、backup／restore、#888との責務境界を再確認し、PASSと判定した。
