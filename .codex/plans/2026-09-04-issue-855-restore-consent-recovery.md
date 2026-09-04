# Implementation Plan

## Goal
端末バックアップ復元を、process停止・再起動・cancel・再同意を含む全ての固定状態遷移で旧状態または再同意待ちの新状態へ収束させ、再同意前のruntime／network処理を開始しない。

## Non-goals
- backup形式v2、cloud backup、複数account一括archiveは追加しない。
- 悪意ある別processとの競合や、永続的なdisk故障まで一般化しない。
- backup／復元と無関係なkeyring、runtime、UIのrefactorは行わない。

## Assumptions
- 現行backup形式、KDF、暗号、容量上限、復元対象データの契約は維持する。
- 復元journalは、processをまたぐtransaction状態のSSoTとして利用できる。
- staging検証は、SQLite migrationと永続設定を読む専用のvalidation-only pathで行い、`DesktopRuntime::new`、Iroh endpoint、remote取得taskを構築しない。
- 再同意待ちでは旧runtimeを停止済みのまま保持し、明示的な再同意後にだけ復元runtimeへ置換できる。
- 既存の#855受入条件のうち、今回修正するのはAC-3とAC-4である。

## Definition of Done
- AC-1: 対象データ、除外データ、脅威モデルの既存レビュー結果を維持する。
- AC-2: 暗号化backupを作成し、新規端末相当の環境へ復元できる既存scenarioを維持する。
- AC-3 / INVAR-1: 破損、誤passphrase、容量不足、cancel、journal各phaseの停止、復旧中の再停止、書込み失敗で既存account／registry／秘密情報を壊さず、再実行可能な旧状態または確定済み新状態へ収束する。
- AC-4 / INVAR-2: 復元対象のアプリ同意・年齢自己申告・成人向け表示・Node同意を移行せず、再同意完了前は復元runtime、remote取得、scheduler、background通知を開始しない。
- AC-5: 鍵だけの移行と完全backupの既存文書上の区別を維持する。
- PR headの独立監査、必須CI、merge tree照合で固定inventory 10件と状態遷移9件の未分類・不適合が0件である。

## Risk and Coverage Contract
- リスク区分: C
- Scope revision: `2026-09-04-issue-855-restore-crash-recovery-v1`
- Relevant invariants: `INVAR-1`（永続状態の失敗時不変性と再実行可能性）、`INVAR-2`（明示的な再同意前の復元runtime／network副作用禁止）
- Fixed surface inventory: `INV-1` restore UI／IPC／command、`INV-2` cancel、`INV-3` prepare／staging、`INV-4` staging検証、`INV-5` install directory／secret／journal、`INV-6` registry commit／journal、`INV-7` app consent／年齢file、`INV-8` runtime公開／background処理、`INV-9` recovery／rollback／finalize、`INV-10` startup／consent acceptance
- State transitions: `TR-1` journal前の入力失敗・cancel、`TR-2` 新規accountのInstalling／Installed停止、`TR-3` 置換accountの旧directory移動前後とrecovery再停止、`TR-4` registry更新前後、`TR-5` consent reset前後と失敗、`TR-6` AwaitingConsent中の起動、`TR-7` 明示的再同意後のactivation、`TR-8` activation失敗時のrollback、`TR-9` activation後cleanup失敗と再起動
- Sensitive sinks / shared callers: staging／final／rollback directory、identity／optional secret、account registry、restore journal、app consent／年齢file、Iroh／Docs／Community Node／background通知、frontend localStorage。restore command、process startup、`accept_app_consents`、background pollから順方向を確認し、各sinkから全callerを逆引きする。
- Independent audit: 秘密情報・永続化・同意前外部送信を扱うリスク区分Cのため、PR headとmerge treeで必要。

## Plan
| ID | AC / INVAR IDs | Task | Outcome | Files / Areas | Acceptance Criteria | Validation | Depends On |
|---|---|---|---|---|---|---|---|
| T1 | AC-3, AC-4 / INVAR-1, INVAR-2 | 固定状態遷移のfailing-before testを追加する | 現行のdirectory消失、startup未回収、同意前runtime、cancel競合を再現する | `crates/desktop-runtime/src/tests/device_backup.rs`、Tauri command／startup／frontend tests | TR-1〜TR-9のうち現行未充足遷移が修正前に失敗し、期待結果が明文化される | targeted Rust／frontend testのfailing-before記録 | None |
| T2 | AC-3 / INVAR-1 | journalとrollback／recoveryを再実行可能にする | Installing／Installedは安全にrollbackし、Committed以後は同意待ちactivationへfinish-forwardする | `crates/desktop-runtime/src/backup.rs`、関連tests | 新規／置換とも各停止点とrecovery再停止で旧directoryを誤削除せず、orphan stagingを回収し、journal書込み失敗も確定状態へ収束する | `cargo test -p kukuri-desktop-runtime tests::device_backup -- --test-threads=1` | T1 |
| T3 | AC-4 / INVAR-2 | restore、startup、再同意のruntime境界を実装する | 外部接続なしのstaging検証、startup前recovery、AwaitingConsent、同意後activationを一つのtransactionとして扱う | `apps/desktop/src-tauri/src/commands/device_backup.rs`、`state.rs`、`lib.rs`、`commands/app_consent.rs`、新規restore orchestration module | consent resetがdurableになるまでjournalを残し、AwaitingConsentではruntime／network 0件、明示的再同意後のbuild成功時だけReadyになる | Tauri targeted tests、`cargo xtask tauri-check`、`cargo xtask e2e-smoke` | T2 |
| T4 | AC-3, AC-4 / INVAR-1, INVAR-2 | cancel境界とbackground gateを閉じる | durable install開始後のcancel表示を止め、backend境界の競合をrollbackし、Ready以外のpollを禁止する | backup UI／tests、`commands/background_notifications.rs` | Installing中にCancelできず、境界直前のcancelはinstallせず、Ready以外ではruntime pollを行わない | frontend targeted test、`cargo xtask desktop-ui-check`、Tauri targeted tests | T2, T3 |
| T5 | AC-1〜AC-5 / INVAR-1, INVAR-2 | 全固定inventoryの回帰検証と実装記録を確定する | 既存成功経路を保ち、10 inventory／9 transition／全sink callerの証跡を残す | tests、scenario、`docs/progress/` | 未分類0、不適合0、ACに紐づかない変更0で、対象validationが全て成功する | device backup scenario、`cargo xtask check`、`test`、`rust-test`、`e2e-smoke`、`doctor`、`oversized-files`、`git diff --check` | T2, T3, T4 |
| T6 | AC-1〜AC-5 / INVAR-1, INVAR-2 | PR head独立監査、CI、merge、merge後監査を一貫して行う | 固定対象だけで#855と#853を再評価し、合格時に終了する | PR、GitHub Issues #855／#853 | 独立監査PASS、必須CI成功、merge tree差分一致、#855の全ACと親#853の固定条件を満たす。新しい基準は追加しない | PR checks、merge tree path diff、main上のtargeted test、Issue inventory照合 | T5 |

## Decision Needed / Blockers
None

## Out of Scope
- 全filesystem／hardware故障に対する一般的なdurability証明
- backup対象・形式・暗号方式の拡張
- 今回の固定entrypoint／sinkに属さない一般的なsecurity hardening

## Single Next Action
T6のPR head独立監査からmerge後監査までを完了する。
