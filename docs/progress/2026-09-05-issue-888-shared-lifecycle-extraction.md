# Issue #888 T2: 同意・復元処理の共有部分抽出

## 対象と維持する挙動

- 種別: `refactor:extract`。#888のCLI機能追加に先立つ独立した抽出PR。
- Scope revision: `2026-09-04-issue-888-cli-command-parity-v1`
- リスク区分: C
- 基準commit: `6f89fae049170f5b77aa6ffb95da052f3fb05dfe`
- 対応条件: AC-2／AC-5、INVAR-1／INVAR-3、INV-1、TR-5／TR-7。#888全体の完了を主張しない。
- 観測した制約: 同意の検証・記録と復元の状態遷移・取消制御がTauri crateにあり、CLIから同じ処理を呼べない。
- 目的: これらのTauri非依存部分だけを`desktop-runtime::host`へ移し、Tauri側は再exportまたは同じ引数を渡すadapterにする。
- 対象外: CLI登録・dispatcher・台帳・socket・E2E追加、バックアップ本体の移行、UI、OS通知、署名・wire・保存形式・同意要件の変更。

## 入口・共有処理・sinkの対応

| 共有処理 | 既存caller | 副作用・保護する境界 |
| --- | --- | --- |
| `ClientOperationState`（Tauriでは`DesktopOperationState`のalias） | Tauri起動時のstate登録、app consent、identity switch、device backup create／restore／cancel、背景通知のevent／poll／action処理 | 同一processの排他、install前の取消gate。cancelはswitch lockを取らない。背景通知のruntime読取も同じswitch lockで直列化 |
| `runtime_access_allowed`／`require_runtime_operation_ready` | invoke gate、identity switch、backup create／restore、背景通知のevent／poll／action処理 | Ready以外ではruntime操作を許可しない |
| `recover_device_restore_before_startup`／`restore_startup_action` | Tauri startup | journal回復を同意読取より先に行う。Committedは同意reset、AwaitingConsentは明示同意後だけ起動 |
| `advance_committed_restore_to_consent` | Tauri startup | 同意reset保存後にAwaitingConsentを保存。reset失敗時はCommittedを維持 |
| `orchestrate_restore_activation`／`persist_restore_activation_phase` | Tauri startup、app consent、`activate_pending_restore` | 起動・activation・rollbackの順序を維持。Activated以降はfinish-forward |
| `require_consent_acceptance_state`／`validate_app_consent_documents` | app consent accept | ConsentRequiredのみ受付。必要文書・版・未知slugの判定とエラーを維持 |
| `record_app_consents`／`app_consent_status`／同意DTO | app consent accept／status | 同じ同意pathへ同じ文書・年齢申告を記録し、同じJSONを返す |

登録点`tauri::generate_handler!`は変更しない。共有処理からの逆引きは上表のcallerに限定され、新たな製品入口は追加しない。
AppHandleを必要とするruntime公開、通知cursorのreset、path解決、復元activation adapterはTauri側へ残す。

## 変更前後の検証

抽出前に次の7テストの成功を確認し、同じテストを共通処理のmoduleへ移した。testの削除・skip・assertion弱体化は行わない。

- `runtime_access_is_allowed_only_for_ready`
- `startup_action_covers_every_safe_restore_boundary`
- `install_cancel_gate_serializes_cancel_at_the_boundary`
- `restore_consent_reset_is_persisted_before_consent_required_status`
- `consent_reset_failure_leaves_committed_phase_unchanged`
- `accept_activation_success_and_phase_write_failure_use_the_same_rollback_boundary`
- `consent_acceptance_is_only_allowed_from_consent_required`

| 検証 | 結果 |
| --- | --- |
| 抽出前のTauri復元6テスト・同意1テスト | 成功 |
| `cargo test -p kukuri-desktop-runtime --lib host::` | 28成功。移動した7テスト、同意前初期化禁止、購読とidentity再起動、台帳の既存テストを含む |
| Tauri libraryの全unit test | 29成功。移動した7テストは上のhostテストで実行 |
| `cargo check -p kukuri-desktop-runtime` | 成功 |
| `cargo xtask tauri-check`／`cargo xtask e2e-smoke` | 成功。e2eは単一runtimeの投稿永続smokeであり、CLI複数process検証の代替ではない |
| `git diff --check`／`cargo xtask oversized-files` | 成功。大型ファイルbaselineの引上げなし |
| 全Rust test・PR CI・独立監査 | PR作成前時点で未完了。未完成のCLI差分を含む作業treeでは全Rust testを成功扱いにせず、抽出PRの固定headでCIを確認する |

Tauri全体の`cargo fmt --check`には基準commitから存在する未変更ファイルの整形差分がある。今回の変更ファイルのみ整形し、無関係な整形を混ぜない。

## 差し戻しと完了判定

保存形式や製品データを変更しないため、この抽出commitの差し戻しだけでTauri内の元配置へ戻せる。
PR headの独立監査と必須CI成功後にマージする。#888は後続の承認済み実装・E2E・最終監査が終わるまでOpenを維持する。
