# #942 ScoreGame projection の退行防止

## 現在の状態

- 状態: 修正とtargeted contract追加が完了し、関連validationを実行中。
- Issue: [#942](https://github.com/KingYoSun/kukuri/issues/942)。区分C。
- Scope revision: `2026-09-08-score-game-projection-freshness-v1`。
- 調査・実装開始基準: `58328f035a391f8f321bffdf6287cd339228fec7`。
- 承認: 2026-09-09 のユーザー指示により、計画の実行、コミット、PR、CI成功後のマージを実施する。独立監査PASSとmerge対象一致をClose条件とする。
- 計画とAC／INVAR・固定inventory／transition: [実装プラン](../../.codex.plans/2026-09-09-issue-942-score-game-projection-freshness.md)。

## 修正前の再現（AC-1）

修正前test commit: `681ef2361502dba50b3bb23f60a337077f46174d`。製品の書込み処理は実装開始基準と同一で、test用のcrate内re-export以外はtestと文書のみ。

`crates/app-api/src/tests/game_projection_freshness.rs` のfixtureは、対象topicの自動購読を停止し、一括／個別hydrationを明示的に実行する。実際のAppService create/updateと実際のMemory／SQLite storeを使い、旧manifestのblob fetchだけを`Notify`で停止する。旧record取得 → blob待機 → Running／round1／Alice7更新 → row確認 → 旧fetch再開の順をtestが所有する。

| test suffix | backend／writer | 修正前の結果 |
| --- | --- | --- |
| `late_record_hydration_keeps_valid_update_memory` | Memory／record | FAIL: score7 → score0、Running → Waiting、manifest hash退行 |
| `late_record_hydration_keeps_valid_update_sqlite` | SQLite／record | 同じFAIL |
| `late_replica_hydration_keeps_valid_update_memory` | Memory／replica | 同じFAIL |
| `late_replica_hydration_keeps_valid_update_sqlite` | SQLite／replica | 同じFAIL |

実行: `cargo test -p kukuri-app-api --lib game_projection_freshness -- --nocapture`。`0 passed; 4 failed; 0 ignored`、exit 101。SQLite／recordの例は`updated_at=1788931230591`→`1788931230583`、replicaは`1788931230591`→`1788931230582`。どちらも`metaverse: None`、`room_kind: ScoreGame`で、全row一致assertが失敗した。sleep・確率的反復・既存assert変更は使っていない。

既存CLI baseline: `cargo test -p kukuri-cli --test content_live game_lifecycle_rejects_invalid_roster_without_mutation -- --exact --nocapture` は1 passed（0.18秒）。自然発生のCI実行順そのものを同定したとは主張せず、CI観測と一致する退行を生む2 writerの実行順をfixtureで確定した。

## 採用判断（T2）

- canonical sourceは既存のdocs stateが指すmanifest。stateのミリ秒timestampやhashの辞書順を新しいversionとみなさない。
- `ServiceHandles`の既存フィールドは`pub(crate)`で、公開constructorは引数を維持したまま内部のroom単位排他を追加できる。cloneされるhandlesを通してAppServiceとsubscription taskで同じ排他を共有する。
- ScoreGame create/updateのcanonical保存からprojection書込みまでと、hydrationのcanonical再照合からprojection書込みまでを共通のroom境界で保護する。hydrateの最初のblob fetchは境界外で行い、違うroomを止めない。
- hydrationの再照合は`LocalOnly`。現在のrecordと候補が違えば、古い候補は書かず現在のrecordを取得・照合し直す。同timestampでも正当なcurrent pointerの更新を採用する。
- 同じprojectionの再取得は`derived_at`だけを更新しない。最新rowを保持し、既存のCLI before/after全値一致契約を維持する。
- storeの公開trait・Memory／SQLite writer・DB schemaは変更せず、既存production ScoreGame callerの前段を保護する。Metaverse用hydrateは既存の無条件upsertを維持する。
- 共有Dome moveの3 writerはsource relationship detach、target active化、source tombstone化。ScoreGameの制御対象には入れず、既存Dome testsで互換性を確認する。

## 実装・inventory差分

- `service/game_projection_support.rs`: room単位の共有排他とhydrate commitを所有。使用中のroomだけを`Weak`で保持し、処理完了／cancel後に使われていないlockを再利用対象から除く。
- `service/mod.rs`: privateな`ServiceHandles.game_room_projections`を追加し、constructor内部で初期化。公開引数・wire・DB schemaは不変。
- `game.rs`: ScoreGame create/updateの保存～cache writeに同じ排他を適用し、hint publish前に解放。updateは更新元のstate読込みより前に排他を取得する。
- `service/hydration_support.rs`: 一括／個別のgame hydrationを同じrecord helperに接続。各既存entryのfetch policyと、未取得時の既存key retry／fallbackを維持する。
- `service/game_projection_support.rs` の再照合でpointerが動いた場合、現在のrecordを再解決する。移動し続けるpointerへの再解決は既存`session_projection_retry_attempts()`で上限を付け、未完了は`false`で既存retry/recoveryに返す。実際に最新rowを確立するか、既存rowの一致を確認するまで成功扱いにしない。
- `crates/store`、Dome move、Metaverse writerは変更なし。新しいSQLx dev dependencyは既存workspace依存を使ったtest用SQLite triggerの観測だけに使用し、production依存は増やさない。

production upsert呼出しは11→10箇所。一括／個別hydrateの2 sinkを新helperの1 sinkへ統合し、game.rsの6箇所とdome_move.rsの3箇所は残る。ScoreGame制御はcreate/updateと新helperのScoreGame分岐、Metaverseの残り4+3 callerと新helperのMetaverse分岐は互換確認対象。公開入口の追加／削除はない。

| 固定inventory | 実装上の適用・互換確認 |
| --- | --- |
| INV-1a | `create_game_room_in_channel`／`update_game_room`のroom排他。owner／transition／roster guard後に保存。CLI／Tauri／Runtime登録点に変更なし |
| INV-1b | listの空cache hydrationが共通helperに到達。public/private scope・mute・Dome readinessの判定は変更なし |
| INV-2a | bootstrap／scope再取得／fallback→一括helper→同じrecord commit。prefix query policyは維持 |
| INV-2b | docs event／hint→key retry→同じrecord commit。current確認の追加readはLocalOnly。早期return／error／cancelはguardを解放 |
| INV-3a | Memory／SQLiteのwriter契約は維持。app層の同じ境界で両backendを検証 |
| INV-3b | Metaverse create/update/event/asset、Dome source-detach／target-active／source-tombstoneは既存writerのまま |

## 条件と追加testの対応

以下のtest名は`service::tests::game_projection_freshness::`配下。両backendをloopするtestは個別に明記する。

| AC／INVAR・TR | test／観測 |
| --- | --- |
| AC-1／2、INVAR-2、TR-2 | `late_{record,replica}_hydration_keeps_valid_update_{memory,sqlite}`。修正前4 FAIL→修正後4 PASS、同じsequence／全row assert |
| AC-3、INVAR-1、TR-3 | `rejected_updates_do_not_mutate_{memory,sqlite}`。空／未知／重複／label変更と非owner。docs apply、blob put/pin、hint、SQLite insert/update/delete回数不変。canonical bytes・manifest bytes・全row不変。成功create/updateがtriggerで2 writeとして観測されることも確認 |
| AC-2／4、INVAR-1／2、TR-4 | `docs_event_resolves_current_pointer_at_same_timestamp`／`hint_resolves_current_pointer_at_same_timestamp`（両backend）。旧fetch停止中にtimestamp100の新manifestをdocsに置き、cache未反映から最新へ収束。その後の旧record replayで全rowとmutation回数不変 |
| AC-2／4、TR-2／4 | `comparison_and_commit_exclude_same_room_writer_{memory,sqlite}`。canonical snapshot取得を止めた状態で同room updateをpollし、更新元readに進めないことを確認。別owner・別roomのupdateは完了し、そのrowを巻き込まない |
| AC-4、TR-5 | `canonical_read_failure_and_cancel_release_the_room`（両backend）。LocalOnly再照合のエラー／cancelでrow不変、次updateが完了 |
| AC-4、TR-5 | `missing_blob_retry_refreshes_without_overwriting_the_cache`（両backend）。実際の既存key retryを通し、未取得の観測通知後にblobを利用可能にする。任意sleepは追加しない |
| AC-4、TR-5 | `missing_and_corrupt_canonical_state_do_not_apply_the_candidate`（両backend）。消失／破損は旧rowを変更せず、正しいstateへ戻した再取得はwrite不要 |
| AC-4、TR-5 | `cache_write_failure_is_recoverable_from_canonical_state`。SQLite書込み拒否triggerでvalid updateがcache保存に失敗してもlockが残らず、docs/blobから再取得可能 |
| AC-4、TR-4 | `restart_and_missing_cache_resolve_docs_at_the_same_timestamp`。SQLite接続を閉じ、lockを新規作成して再open。同timestampの最新canonicalから保存済み旧cacheを更新し、cache削除後も再構築 |

## 検証・監査

- `cargo test -p kukuri-app-api --lib game_projection_freshness -- --nocapture`: 15 passed、0 failed、0 ignored（1.05秒）。
- CLI／store parity／既存game・Dome／Runtime restart、scenario、path別validationは実行中。独立監査、CI、merge tree照合は未実施。
- ADR 0006の`late_joiner_backfills_game_room_manifest`という同名testは現行にはない。現行`game_room_score_update_replicates`はsender側create/update完了後にreceiverが初めて`list_game_rooms`でsubscribeするsequenceを持つため、latest stateのlate-join取得証跡として同testをfeature有効で実行する。
