# Issue #855 復元transaction・再同意・crash recovery再実行

## 対象

- Scope revision: `2026-09-04-issue-855-restore-crash-recovery-v1`
- 基準 commit: `1b071f5140290ceb0f361a9be7fd6a64ce134ac4`
- リスク区分: `C`
- 記録時点: PR head確定前のworking tree。実装とlocal validationの証跡を固定し、PR head独立監査とCIへ引き渡す段階。
- Goal: 端末バックアップ復元を、process停止、再起動、cancel、再同意を含む固定状態遷移で、再実行可能な旧状態または再同意済みの新状態へ収束させる。明示的な再同意前は復元runtimeとnetwork処理を開始しない。
- Non-goals: backup形式v2、cloud backup、複数account一括archive、悪意ある別process、永続的なdisk／hardware故障、固定surface外の一般的なsecurity hardening。

今回の修正対象は既存の`AC-3`／`INVAR-1`と`AC-4`／`INVAR-2`だけである。`INV-1`〜`INV-10`と`TR-1`〜`TR-9`を固定し、監査中に新しい完了条件を追加しない。

## failing-before

固定状態遷移のtestとUI contractを実装より先に置き、次を確認した。

| 対象 | 修正前の観測 |
| --- | --- |
| restore phase／startup recovery | `DeviceRestorePhase`、pending phase照会、AwaitingConsent／Activated遷移など、状態遷移testが要求するAPIが存在せず、targeted Rust testが`E0432`でcompileできなかった。既存journalはInstalling／Installed／Committed後の再同意activationを表現できなかった。 |
| cancel境界 | `hides cancellation once restore installation begins`を先に追加すると、`Installing`通知後もCancelが表示されたままで失敗した。backendにもcancel受付終了を線形化する境界がなかった。 |
| crash／rollback | journal保存直後、旧directory移動後、registry更新後、rollback途中の再停止を個別に再現するfailure pointと回帰testがなく、旧状態不変性を証明できなかった。 |
| 全体testの分類contract | test追加後の最初の`cargo xtask test`は、実装失敗ではなく共有`IdentityStorage` lockの宣言件数が旧値のままで1件失敗した。testを削除・弱体化せず、分割後の実取得点へ分類contractを合わせた。 |

## PR前の予備独立監査

最初の修正案は予備独立監査で`FAIL`となった。指摘は固定済み`AC-3`／`AC-4`とinventory内の次の4件であり、working tree上で修正済みである。

1. staging検証に`DesktopRuntime::new`を使い、Iroh endpointとremote hydrate taskを作っていた。
   - `validate_prepared_device_restore`を追加し、SQLite migration、FileOnly identity、Community Node／discovery設定、private channel／gossip保存状態だけを検証するvalidation-only pathへ置換した。
2. 同一processのAwaitingConsentで停止済みの旧`DesktopState`が残り、一部commandだけの個別guardでは別のapp IPCからHTTP／runtimeへ到達できた。
   - `generate_handler!`の全app commandを`with_desktop_startup_gate`で包み、非Ready時のallowlistをstartup status、consent status、consent accept、cancelの4件へ限定した。`accept_app_consents`自身も`switch_guard`取得後に`ConsentRequired`を再確認する。background subscribe／pollも`Ready`と同じlockを要求する。
3. 復元commandの応答直後、再同意とactivationより前にfrontend localStorageへ復元値を適用し、apply／ack失敗時のrollbackとprocessをまたぐ再実行状態がなかった。
   - frontend stateをjournalから`Activated`後のdurable markerへ移し、Ready確認後だけ適用する。許可済みkeyのsnapshotを取り、applyまたはack失敗時は旧値へ戻してmarkerを残す。
4. ADR／runbookがCommitted／AwaitingConsentをrollbackする説明のままで、Risk Cの状態遷移とsink逆引きの証跡がなかった。
   - ADR 0048、利用者向けquickstart、troubleshooting、開発runbook、データ分類を、Installing／Installedはrollback、Committed／AwaitingConsentは再同意待ち、Activatedはfinish-forwardという現行contractへ更新した。本書に固定inventory、transition、sensitive sinkの対応を残す。

予備監査の外部記録は[Issue #855 comment](https://github.com/KingYoSun/kukuri/issues/855#issuecomment-5536966433)に残している。この記録は最終PR head独立監査の代替ではない。

## 固定surface inventory

集計: 合計10 / 適合10 / 不適合0 / 未分類0。

| ID | 入口・実装 | guard／副作用 | test / scenario |
| --- | --- | --- | --- |
| `INV-1` | `DeviceBackupPanel.handleRestore` → `restoreDeviceBackup` → `restore_device_backup_command` | 同一公開鍵は明示的置換確認を要求する。commandは`switch_guard`と`Ready`を確認し、旧runtime停止後にprepare→validation→install→commit→consent resetの順で進む。 | `requires explicit confirmation before replacing the same account`、`encrypted_device_backup_restores_one_account_as_one_file`、`desktop_device_backup_restore` |
| `INV-2` | `cancelDeviceBackup` → `cancel_device_backup` → `DesktopOperationState::cancel_device_backup` | `close_device_backup_cancel_gate`がinstall境界を線形化し、`Installing`後はUIからCancelを除く。 | `hides cancellation once restore installation begins`、`install_cancel_gate_serializes_cancel_at_the_boundary`、`device_restore_cancellation_can_be_rechecked_at_the_install_boundary` |
| `INV-3` | `prepare_device_restore`／`PreparedDeviceRestore` | 先行recovery、pending transaction／frontend marker拒否、passphrase・形式・容量・path・hash・identity検証、staging cleanupを所有する。 | `restore_failures_preserve_the_existing_account_registry`、`storage_exhaustion_during_restore_preserves_existing_state`、`recovery_removes_only_exact_restore_staging_prefix_orphans` |
| `INV-4` | `validate_prepared_device_restore` | SQLite、FileOnly identity、永続設定だけを読み、runtime、Iroh endpoint、remote taskを構築しない。 | `validation_only_checks_all_restored_inputs_without_creating_runtime_artifacts`、`file_only_optional_secret_ignores_keyring_shadow_and_failure` |
| `INV-5` | `install_prepared_device_restore` | Installing journalを先に保存し、旧directory退避、optional secretのkeyring scrub、staging→final rename、Installed journalを行う。途中errorは`error_with_restore_rollback`へ集約する。 | `replacement_crash_after_journal_before_old_move_preserves_original`、`replacement_stop_after_existing_directory_move_recovers_identity`、`new_account_install_stop_rolls_back_without_changing_existing_state`、`post_journal_move_error_and_installed_journal_error_rollback_immediately` |
| `INV-6` | `commit_device_restore` → `register_restored_account` | restore側でregistryを変更する唯一の経路。registry更新後にCommitted journalを保存し、途中失敗は旧registryへ戻す。 | `registry_commit_stop_recovers_old_active_account_before_path_resolution`、`committed_restore_remains_pending_until_consent_and_activation` |
| `INV-7` | `reset_app_consent_after_device_restore`、`advance_committed_restore_to_consent`、`accept_app_consents` | app consent／年齢fileのdurable write成功後にのみAwaitingConsentへ進む。 | `restore_consent_reset_is_persisted_before_consent_required_status`、`consent_reset_failure_leaves_committed_phase_unchanged` |
| `INV-8` | `build_runtime`、`build_desktop_state`、`publish_desktop_state`、background subscribe／`poll_once` | 復元runtimeは再同意後の`orchestrate_restore_activation` → `activate_pending_restore`だけが公開する。background処理は`Ready`と`switch_guard`を再確認する。 | `runtime_access_is_allowed_only_for_ready`、`consent_required_rejects_network_sink_and_ready_allows_it`、`non_ready_allowlist_is_minimal_and_restore_frontend_state_remains_gated` |
| `INV-9` | `recover_interrupted_restore_inner`、`rollback_from_journal`、`finalize_from_journal` | Installing／Installedはrollback、Committed／AwaitingConsentは保持、Activatedはfinish-forwardする。frontend markerをcleanupより先にdurable化する。 | `replacement_rollback_can_resume_after_original_directory_was_restored`、`replacement_rollback_resumes_after_new_directory_was_removed`、`activated_frontend_state_survives_finalize_restart_until_acknowledged` |
| `INV-10` | `run::setup`、`initialize_desktop_state`、`accept_app_consents`、`with_desktop_startup_gate` | startupは同意file読取・runtime構築より前にrecoveryを行う。再同意commandはbackend状態を再確認し、startupと同じactivation orchestrationを使う。 | `startup_action_covers_every_safe_restore_boundary`、`consent_acceptance_is_only_allowed_from_consent_required`、`accept_activation_success_and_phase_write_failure_use_the_same_rollback_boundary`、`previous_runtime_restart_requires_clean_journal_and_original_registry` |

## 固定状態遷移

集計: 合計9 / 適合9 / 不適合0 / 未分類0。

| ID | 結果 | evidence |
| --- | --- | --- |
| `TR-1` | journal前の誤passphrase、破損、未知版、容量不足、cancelはregistryと既存app dataを変えず、stagingを除去する。 | `restore_failures_preserve_the_existing_account_registry`、`storage_exhaustion_during_restore_preserves_existing_state`、cancel gate 2 test |
| `TR-2` | 新規accountのInstallingでfinalが無ければ何も削除せず、Installedでfinalがあれば未commit accountだけを削除してregistry snapshotへ戻す。 | `new_account_install_stop_rolls_back_without_changing_existing_state`、`rollback_from_journal`の`rollback_dir=None`分岐 |
| `TR-3` | 置換accountの旧directory移動前後とrollback中の再停止で、復元済み旧directoryを誤削除しない。 | replacement crash／move／rollback再停止test 4件 |
| `TR-4` | registry更新後・Committed journal前の停止はInstalledとしてrollbackし、Committed保存後は新accountの再同意待ちへ進む。 | `registry_commit_stop_recovers_old_active_account_before_path_resolution`、`committed_restore_remains_pending_until_consent_and_activation` |
| `TR-5` | consent reset失敗ではCommittedを維持し、reset成功後だけAwaitingConsentへ進む。 | consent reset 2 test、Committed recovery test |
| `TR-6` | AwaitingConsentかつ未同意ではruntime初期化をspawnせず、app IPCとbackground sinkを停止する。 | startup action、runtime access、invoke gate test |
| `TR-7` | `ConsentRequired`からの明示的な再同意だけがruntime構築、Activated保存、cleanup、runtime公開へ進む。 | consent state guard、activation orchestration success、frontend marker test |
| `TR-8` | runtime構築またはActivated保存の失敗は旧directory／registryへrollbackし、旧runtime再構築の検証に成功した場合だけReadyへ戻す。Activated後はrollbackしない。 | activation phase write failure、rollback再停止test、previous runtime verification test |
| `TR-9` | Activated後のfrontend marker／cleanup中断はstartupで再実行し、markerはfrontend ackまで保持する。 | `activated_frontend_state_survives_finalize_restart_until_acknowledged`、frontend apply／ack failure test |

## Sensitive sinkの逆引き

集計: 合計7 / 適合7 / 不適合0 / 未分類0。

| sink | production callerと支配guard |
| --- | --- |
| staging／final／rollback directory | 変更入口は`restore_device_backup_command`だけ。`prepare_device_restore`、`install_prepared_device_restore`、startup recovery、command失敗rollback、activation失敗rollbackへ全callerを分類した。 |
| identity／optional secret | restore書込みは`prepare_device_restore::persist_restored_secrets`と`install_prepared_device_restore::{make_account_file_self_contained,scrub_keyring_optional_secrets}`だけ。validationはFileOnlyでkeyring shadowを参照しない。 |
| account registry | restore mutationは`commit_device_restore` → `register_restored_account`だけ。rollbackはjournal内`registry_before`をatomicに復元する。 |
| restore journal | `write_restore_journal`のcallerはinstall、commit、phase transitionの3系統。read、rollback、finalizeは同じrestore recovery module内へ閉じている。 |
| app consent／年齢file | `save_app_consent_store`のproduction callerはrestore resetと`accept_app_consents`。restore resetの入口はrestore commandとCommitted startup recoveryだけ。 |
| Iroh／Docs／Community Node／background通知 | Tauri内の`DesktopRuntime::from_env`は`build_runtime`一箇所。callerはnormal startup、明示再同意activation、検証済みrollback後の旧runtime再構築、Ready中のaccount switch／backup再開に分類した。backgroundのevent subscribeとpollは`Ready`と`switch_guard`を要求する。 |
| frontend localStorage | 復元値のwriterは`applyPortableFrontendState`、callerは`applyPendingDeviceRestoreFrontendState`だけ。そのproduction callerはAppのReady startupと再同意commandがReadyを返した後の2箇所。apply／ack失敗は許可済み6 keyを旧snapshotへ戻す。通常のtheme／workspace／draft／Community Index preference writerはReady effectまたはReady後のshell配下にある。 |

## AC / INVAR判定

- `AC-3`／`INVAR-1`: working tree上で適合。入力拒否、容量不足、cancel、Installing〜Activatedの停止点、registry境界、rollback再停止、frontend apply／ack失敗が、再実行可能な旧状態または確定済み新状態へ収束する実装とtestへ対応した。
- `AC-4`／`INVAR-2`: working tree上で適合。validation-only path、startup先行recovery、Committed consent reset、AwaitingConsent無runtime、全app IPC gate、background gate、再同意後activation、Activated後frontend markerの順序を固定した。

この判定はPR head独立監査前の実装evidenceであり、IssueをCompleteにする最終判定ではない。

## 非blocker分類

activation前の失敗でrollbackと旧runtime再構築が成功すると、backendは`Ready`へ戻す一方、`accept_app_consents`は元のactivation errorを返すため、現在のReact画面はreloadまで同意画面に残る。

固定入口から到達し、一時的なUX影響はある。しかしjournal、registry、旧runtimeはすでに旧状態へ収束し、reloadまたはprocess restart後は利用可能でrestoreも再実行できる。固定`INVAR-1`は同じReact render cycle内でshellへ戻すことを要求しておらず、データ損失、禁止network、権限拡張、誤った永続mutationもない。このためBlocker四条件のうち固定AC／INVAR違反を満たさず、Optional-hardeningとしてClose条件へ追加しない。

AppのReady判定、Tauri gate、coreの実file停止点、frontend marker rollbackを一体で起動する単一testはないが、各owner境界のtestと静的caller逆引きで固定条件を構成的に確認できる。未知のbug不存在や同一test binaryへの統合は完了条件にしない。

## 記録時点のvalidation

次は現在のPR前working treeで成功済みである。

- `cargo fmt --all -- --check`: 成功。
- core device backup targeted test: 19件成功。
- frontend targeted test（App、DeviceBackupPanel、device backup API）: 15件成功。
- `cargo test -p kukuri-desktop-tauri --lib`: 36件成功。
- `cargo xtask doctor`: 成功。
- `cargo xtask check`: 成功。format、workspace clippy、Tauri compile、frontend lint／typecheckを含む。
- `cargo xtask test`: Rust 789件、harness 22件、frontend 141 files／1093件、doc test成功。Rust 3件は既定どおりskip。
- `cargo xtask scenario desktop_device_backup_restore`: 4 step成功。
- `cargo xtask rust-test`: Rust 789件、harness 22件、doc test成功。Rust 3件は既定どおりskip。
- `cargo xtask desktop-ui-check`: lint、typecheck、frontend 141 files／1093件、Storybook build、browser 58件、visual 14件が成功。
- `cargo xtask tauri-check`: 成功。
- `cargo xtask e2e-smoke`: `desktop_smoke_post_persist` 6 step成功。
- `cargo xtask oversized-files`: 成功。今回の変更による新規違反なし。
- `cargo xtask ipc-types --check`: 型export testと生成済み型の差分検査が成功。
- `git diff --check`: 成功。Windows checkoutのLF→CRLF warningのみ。

次は本書記録時点で未完了である。成功したとみなさず、結果をPRとIssueへ記録する。

- PR headの独立監査
- 必須CI
- merge後treeと監査対象treeの比較、および差分がある場合のdelta監査

## Merge／Close gate

リスク区分Cのため、PR本文は`Refs #855`とし、自動Close文言を使わない。固定PR headに対する独立監査`PASS`と必須CI成功の両方を確認してからmergeする。merge後は対象treeをPR headと照合し、Issue本文のCurrent status、`AC-3`、`AC-4`、親Issue #853との対応を更新してからCloseする。

関連: #853、#855、#859、ADR 0048
