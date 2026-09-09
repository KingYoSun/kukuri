# #942 ScoreGame projection の退行防止

## 現在の状態

- 状態: 実装中。修正前の制御可能な再現を確認済み。
- Issue: [#942](https://github.com/KingYoSun/kukuri/issues/942)。区分C。
- Scope revision: `2026-09-08-score-game-projection-freshness-v1`。
- 調査・実装開始基準: `58328f035a391f8f321bffdf6287cd339228fec7`。
- 承認: 2026-09-09 のユーザー指示により、計画の実行、コミット、PR、CI成功後のマージを実施する。独立監査PASSとmerge対象一致をClose条件とする。
- 計画とAC／INVAR・固定inventory／transition: [実装プラン](../../.codex.plans/2026-09-09-issue-942-score-game-projection-freshness.md)。

## 修正前の再現（AC-1）

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

## 検証・監査

修正後のtest、path別validation、独立監査、CI、merge tree照合は未実施。結果は実行後にこの記録へ追加する。
