# Issue #942: ScoreGame projection の退行防止 実装プラン

## 目的・対象外

有効な更新を一覧で確認した後、遅着した古い hydration が ScoreGame の状態・score・manifest 参照を以前の値へ戻さないようにする。無効 roster／非 owner の更新拒否では、対象の manifest・docs state・projection を変更しない。

- 対象 Issue: [#942](https://github.com/KingYoSun/kukuri/issues/942)。#872／PR #941 の構造整理とは独立した既存不具合として扱う。
- 対象: ScoreGame の create/update/list、bootstrap・docs event・hint・再取得の hydration、Memory／SQLite projection、および共有 writer の互換確認。
- 対象外: public API／wire／署名／DB schema、時刻の意味、Metaverse lifecycle／readiness、CLI command 形状の変更。LiveSession など別機能への一般化、大型ファイル分割、任意の性能改善も含めない。
- 計画作成時の依頼は調査と計画ファイル作成まで。その後、2026-09-09 にユーザーが本計画の実行、コミット、PR作成、CI成功後のマージを承認した。区分Cの独立監査も必須条件として実施する。

## 前提・完了条件

- 計画作成日: 2026-09-09。
- 状態: T1～T6の実装・ローカル検証を完了。現在の証跡は [作業記録](../docs/progress/2026-09-09-942-score-game-projection-freshness.md)、T7の独立監査・CI・merge判定は [PR #951](https://github.com/KingYoSun/kukuri/pull/951) に集約する。
- リスク区分: C（共有 projection／永続 cache と拒否更新の mutation 境界）。
- Scope revision: Issue の `2026-09-08-score-game-projection-freshness-v1` を維持。
- Issue の基準 commit: `e98d2025e01332a6dbd4b13f5f4712654a931c65`。
- 今回の調査 HEAD: `58328f035a391f8f321bffdf6287cd339228fec7`。調査開始時の tracked 差分はなし。Issue 基準からこの HEAD までの `crates/app-api`、`crates/store`、`crates/kukuri-cli/tests/content_live.rs` に差分なし。
- 規約: [PLANS.md](../PLANS.md)、[Issue lifecycle](../docs/runbooks/issue-lifecycle.md)、[path 別検証](../REFACTORING.md#path別検証マトリクス)、[開発手順](../docs/runbooks/dev.md)。
- 製品契約: [ADR 0006](../docs/adr/0006-game-room-data-classification.md) の docs pointer + manifest blob が正本、cache は再構築可能、owner 限定更新と固定 participant。[ADR 0036](../docs/adr/0036-spatial-context-dome-instance-move.md) の Dome 専用 authority と共有 read model の境界を維持する。

AC／INVAR の正本は Issue 本文。以下は作業と証跡の対応に使う要約であり、独立した完了 checkbox は設けない。

| 条件 | 固定する内容 | 対応 Task |
| --- | --- | --- |
| AC-1 | old record 取得 → 有効 update → old hydration write の順を制御した修正前失敗と writer 特定 | T1 |
| AC-2 | 同じ再現条件で、最新の正当な projection を old hydration が上書きしない | T2、T3 |
| AC-3 | CLI の before/after 不変 assert を維持し、無効／非 owner 更新による禁止 mutation が 0 | T4 |
| AC-4 | writer 全 caller、Memory／SQLite、再取得、複数 room、必須 validation、独立監査 | T2、T5、T6、T7 |
| INVAR-1 | API／wire／schema、owner／roster guard、同 timestamp の既存有効更新を維持 | T2～T7 |
| INVAR-2 | test の削除／skip／弱体化・任意 sleep を使わず、version の意味は既存 canonical state から定める | T1～T7 |

## 調査結果

### 確認できた事実

1. [game.rs](../crates/app-api/src/game.rs) の `update_game_room` は `ensure_topic_subscription` → canonical state／manifest 取得 → owner、transition、roster の検証 → manifest 保存 → projection upsert → hint publish の順。roster 拒否は保存より前であり、拒否処理自体を古い値の writer と断定できない。
2. [hydration_support.rs](../crates/app-api/src/service/hydration_support.rs) の `hydrate_game_rooms_from_replica`（471行～）と `hydrate_game_room_from_record`（515行～）は別々の upsert 経路。どちらも取得済み record を保持し、blob status／manifest fetch の await 後にその record 由来の row を書く。片方だけの修正では不足する。
3. [Memory writer](../crates/store/src/memory/live_game.rs) は map に無条件 insert、[SQLite writer](../crates/store/src/sqlite/live_game.rs) は `ON CONFLICT(room_id) DO UPDATE` に freshness 条件を持たない。古い状態で新しい row を置換できる構造である。
4. [object_persistence_support.rs](../crates/app-api/src/service/object_persistence_support.rs) の `game_projection_row_from_state` は `updated_at = state.updated_at`、`derived_at = 現在時刻`、`projection_version = 1`。`derived_at` は hydration 実行時刻、`projection_version` は固定値で、更新の新旧を表さない。実際の key は `sessions/game/<room_id>/state`。
5. [live_game_support.rs](../crates/app-api/src/service/live_game_support.rs) の `persist_game_room_manifest` は state の `updated_at` をミリ秒で採番する。manifest 側の時刻とは別の採時で、同一ミリ秒になり得る。[backend_parity/live_game.rs](../crates/store/src/tests/backend_parity/live_game.rs) は同一 room・同一 timestamp `100` の `Waiting` → `Paused` 上書きを既に検査する。単純な `incoming.updated_at > existing.updated_at` は既存契約を壊す。
6. [DocRecord](../crates/docs-sync/src/types.rs) は key/value/content_hash/content_len を持つが、順序付け可能な revision は公開していない。manifest hash や content hash は同一性の判定に使えても、辞書順を更新順として扱えない。
7. `list_game_rooms_scoped` は既存 row があれば原則 cache を返し、空の場合に `hydrate_scope_projection` を呼ぶ。したがって常時 list 再取得や CLI 側の待機追加ではなく、writer 境界を修正する必要がある。
8. bootstrap は `spawn_subscription_task` → `hydrate_subscription_state`、docs event／hint は `hydrate_subscription_event`／`hydrate_subscription_hint` → key retry に流れる。event／hint の hydrate 件数が 0 の場合には一括 hydration へ戻る経路もある。個別 hydration の戻り値変更はこの fallback と合わせて確認する。

### 仮説・未確認

- Issue 記載の CI では score7 確認後に Waiting／score0／旧 manifest／古い updated_at へ退行した。旧 record hydration の遅着と整合するが、CI で実際に走った writer の順序を今回独立には確定していない。
- Issue にある「ローカル1回＋20回 PASS」は過去の観測であり、今回の実行結果ではない。今回は静的調査のみで、CLI test・追加再現・製品 suite は未実行。
- 制御可能な fixture でこの候補が失敗しなければ AC-1 未達として writer の trace を調べ直す。CI の原因が証明されたと見なして見込み修正を進めない。

## 実装方針

まず T1 の再現を確定し、T2 で「何を新しい状態とするか」と「比較から書込みまでの競合防止」を一緒に決める。正本は現在の docs state が指す manifest とし、時刻を version に置換する設計や schema 追加は行わない。

第一候補は、ScoreGame の同一 room に対するローカルの更新と hydration commit を共通の排他境界に置き、hydration の blob await 後に現在の docs state と候補を照合してから projection を書く方法とする。更新側は canonical persist と projection 反映を同じ境界で扱い、hydration 側の「照合後、upsert 前」に新しい writer が割り込めないようにする。初回 record 取得や遅い blob fetch を全 room 共通の lock で囲まない。同じ timestamp でも canonical pointer が変わった正当な更新は反映する。

T2 で AppService と subscription task の共有状態の持ち方を確認し、既存 public constructor／ServiceHandles の公開形状を維持できる最小配置を選ぶ。既存 writer と fresh read を共通境界で扱えない場合は、既存 row の同一性を条件にした atomic な書込みなどと比較し、採用理由を残す。新しい public store API が必要になった場合は、対象外との衝突を隠して進めず判断点として示す。

採用案は次を全て満たすことを T2 の終了条件とする。

- 同じ room の old hydration と local update／別 hydration が、比較と write の間でも競合しない。docs の remote 更新自体をローカル lock が止めるとは仮定しない。
- 古い候補を破棄した場合、latest の再取得または既に最新であることの確認へ進む。単なる成功扱いで missing cache を残したり、fallback で古い候補を再適用したりしない。
- 初回 cache 作成、再取得、同 timestamp の異なる manifest、restart 後の保存済み cache、room 間の独立性を維持する。
- `updated_at >=` にするだけでは同 timestamp の旧 hydration を判別できず、docs 再確認だけでは確認後の割込みが残る。どちらも単独では競合対策の根拠にしない。
- ScoreGame に必要な制御を最小限に置き、Dome／Metaverse の共有 upsert の既存契約を維持する。共有関数の重複をまとめる場合も、同じ bug の両 write 経路を保護するための範囲に限定する。

## 固定 surface inventory

Issue の INV-1～3 を以下に展開する。追加の製品要件ではなく、既存 caller の名前と副作用を具体化したもの。

| ID | 入口・trigger | shared helper | 読み書き・外部副作用 | guard／invariant | transition | test／証跡 |
| --- | --- | --- | --- | --- | --- | --- |
| INV-1a | CLI `commands/live_game.rs`、Tauri `commands/live_game.rs` → Runtime `runtime/sync_live_api.rs`／`runtime/private_channels_game_api.rs` → create/update | `create_game_room_in_channel`、`update_game_room`、`persist_game_room_manifest` | blob put/pin、docs apply、blob status、room cache、hint publish | owner／transition／roster 検証が mutation より前。canonical persist と cache の競合防止 | TR-1、3、4 | CLI 2本、T1／T4 の spy、既存 AppService game tests |
| INV-1b | `list_game_rooms`／`list_game_rooms_scoped`、空一覧、scope 再取得 | `ensure_scope_subscriptions`、`hydrate_scope_projection` | cache read、必要時 bootstrap／docs/blob 取得と cache write | audience／mute filtering 維持、再取得で退行しない | TR-1、2、4 | T3／T5、persist scenario |
| INV-2a | startup／restart／public・private subscription bootstrap、明示再取得、一括 fallback | `hydrate_topic_state` → `hydrate_subscription_state` → `hydrate_game_rooms_from_replica` | docs prefix query、各 blob fetch/status、room cache write | 取得済み records の遅着を拒否し、別 room を変更しない | TR-2、4、5 | 一括経路の制御可能 test、restart／private channel 回帰 |
| INV-2b | docs event、`SessionChanged(game-session)` hint、retry | `hydrate_subscription_event`／`hydrate_subscription_hint` → `hydrate_game_room_from_key_with_retry` → key → record | exact query、blob fetch/status、room cache write、0件時 fallback | retry／早期 return と排他解除、古い候補破棄後の収束 | TR-2、4、5 | 個別経路の制御可能 test、hint 再取得 test |
| INV-3a | 全 `upsert_game_room_cache` caller | `game_projection_row_from_state`、`LiveGameProjectionStore`、Memory／SQLite 実装 | room_id 単位の map／DB row mutation | 同 timestamp、異なる room、restart 後の同じ契約 | TR-1～5 | backend parity、row roundtrip、T5 |
| INV-3b | 共有する Metaverse create/update/event/asset、Dome move | 下記 production caller 一覧 | 同じ cache、関連 canonical state／blob | 今回の制御で既存 Dome lifecycle／readiness を変更しない | TR-4 | AppService game／dome_listing／dome_move tests、該当 scenario |

### writer の逆引き基準

調査 HEAD の production upsert は次の11箇所。test fixture の直接書込みは別に分類する。

- `crates/app-api/src/game.rs`: `create_game_room_in_channel`、`create_metaverse_room_in_channel`、`update_game_room`、`update_metaverse_room`、`publish_metaverse_room_event`、`import_metaverse_room_asset`（6箇所）。
- `crates/app-api/src/dome_move.rs`: `move_dome` 内の3箇所（258／313／371行）。各分岐の意味・source／target の副作用は T2 で確認する。
- `crates/app-api/src/service/hydration_support.rs`: `hydrate_game_rooms_from_replica`、`hydrate_game_room_from_record`（2箇所）。
- test の直接 upsert: `tests/dome_listing.rs`、store の `tests/row_mapping_roundtrip_live_game.rs`、`tests/backend_parity/live_game.rs`。`tests/row_mapping_edge.rs` は SQL による読み戻しの異常値確認も行うため store writer 変更時に確認する。

再列挙は CodeGraph を優先し、trait 経由の自由関数 caller も拾うため `rg -n 'upsert_game_room_cache|hydrate_game_room(s)?_from|game_projection_row_from_state' crates -g '*.rs'` を併用する。forward は subscription の登録点と CLI／Tauri／Runtime、reverse は upsert と `persist_game_room_manifest` → `store_manifest_blob`／`persist_game_room_state` から辿る。数だけで適合判定しない。

期待する inventory 差分は、ScoreGame writer の共通制御と2つの hydration 経路の同じ freshness 判定への接続。公開入口・room 種別・DB table の追加／削除はない。内部 helper の増減と実際の caller を T3 後に更新する。

### sensitive sink と拒否時の測定

- `store_manifest_blob` 経由の対象 manifest の `BlobService::put_blob`／pin。
- `persist_game_room_state` 経由の対象 state key への `DocsSync::apply_doc_op`。
- `upsert_game_room_cache` 経由の対象 `game_room_cache` mutation。
- 更新成功後の game-session hint publish と blob status 更新も観測する。

拒否 test は初期 hydration を同期してから計測区間を切り、対象 room の docs bytes／hash、manifest ref／bytes、projection 全列と各 sink の呼出し数を比較する。背景 hydration に許される read／blob status refresh と、拒否された操作由来の mutation を区別する。別 test では old hydration を並走させても projection が退行しないことを確認する。単なる一覧一致だけを「blob／DB 書込みゼロ」の証拠にしない。

## 状態遷移と再現

| ID | 事前状態 | event／sequence | 期待状態 | 許可する I/O | 禁止する副作用 | test／証跡 |
| --- | --- | --- | --- | --- | --- | --- |
| TR-1 | room／cache なし → Waiting | Alice/Bob create → Running／round1／Alice7 → list | 正当な state・manifest・score7 | 正当な manifest／docs／cache write と hint | 他 room 更新 | 既存 CLI、game replicate、T1 |
| TR-2 | old record A を取得済み | A の blob await を停止 → valid update B 完了 → list B 確認 → A を再開 | B の全 projection／manifest 参照を維持 | A の read、必要な最新 state read | A による対象 row の退行 | 一括／個別の Memory／SQLite 再現。別 hydration B との競合も確認 |
| TR-3 | B が保存・表示済み | 空 roster／重複・未知 participant 等の既存拒否条件／非 owner update → list | 対象 canonical と projection が不変 | 既存 state／manifest read | 拒否操作による対象 manifest put/pin、docs apply、cache mutation、更新 hint | CLI 2本＋sink spy。TR-2 と並走する場合も別に検査 |
| TR-4 | 同 timestamp A/B、複数 room、保存済み cache | 正当な B 更新、A 遅着、別 room 更新、再取得、restart | canonical B を採用し、他 room は不変。restart から最新を復元 | 対象の read／正当 write | 時刻 tie だけで B 拒否、他 room の巻込み、Dome の互換破壊 | store parity、AppService 制御 test、Runtime restart、persist scenario |
| TR-5 | missing/corrupt state、blob 未到着、I/O 失敗 | hydrate → 早期 return／error／既存 retry → blob 到着・再取得、または task cancel | 古い候補を反映せず、成功時に最新へ収束。排他が解放される | 既存方針の read／retry、成功時 cache write | 失敗時の部分的な旧 row 適用、無限待機、fallback による再退行 | T3／T5 の fault injection、同期点と timeout による失敗判定 |

TR-5 は AC-4 の既存 retry／再取得への影響確認として TR-4 を展開したもの。新しい retry 方針や一般的な障害回復機能は追加しない。

T1 の fixture は blob service のラッパーで旧 hash の fetch だけを `Notify`／channel 等の明示的な同期点で止め、新 hash は通す。test 側で「旧 record が確実に読み終わった」「B が cache に反映された」「旧 write を再開した」を順に確認する。timeout はハング検出にのみ使い、sleep／確率的な反復を再現の根拠にしない。writer trace は入口名、room／source key、state updated_at、manifest hash、write 順を記録し、payload や秘密値を出さない。

## 作業

| ID／Phase | AC／INVAR・inventory／transition | 作業・対象 path | 受入条件 | 検証・証跡 | 依存 |
| --- | --- | --- | --- | --- | --- |
| T1／1 再現 | AC-1、INVAR-2、INV-1a／2、TR-1／2 | `crates/app-api/src/tests/` に ScoreGame freshness 専用 test module と必要最小の test double を追加。既存 CLI test も変更前に実行 | 一括／個別で A取得→B確定→A再開を制御し、修正前に B 不変 assert が失敗。writer を記録 | Memory／SQLite の失敗ログ、fixture の同期点、旧／新 row。CI原因と再現できた競合を区別 | なし |
| T2／1 方針確定 | AC-2／4、INVAR-1／2、INV-1～3、TR-2／4／5 | hydration、`service/mod.rs`、subscription、game、store、Dome move の caller と candidate freshness を照合。上記第一候補の境界・共有方法を確定 | canonical 同一性、排他／atomic write、同 timestamp、戻り値と fallback、全 caller の適用／非適用が根拠付きで決まる | この計画または issue progress に判断を追記。未分類0、schema／public API変更なし | T1 |
| T3／2 修正 | AC-2、INVAR-1／2、INV-1～3、TR-2／4／5 | `crates/app-api/src/game.rs`、`service/hydration_support.rs` と内部 coordination／subscription 配線を必要最小限変更。store 修正は T2 で必要と判定した場合だけ | T1 の同じ test が PASS。同 timestamp の古い候補、確認後の割込み、fallback、cancel でも退行しない | T1＋比較直後にも同期点を置いた競合 test。writer inventory 差分 | T2 |
| T4／2 拒否契約 | AC-3、INVAR-1／2、INV-1a／3、TR-3 | AppService test double で blob／docs／projection mutation を計測し、`crates/kukuri-cli/tests/content_live.rs` の既存 assert を保持 | invalid／非 owner で禁止 mutation0、対象全列不変。背景 old hydration があっても退行しない | CLI invalid roster／non-owner、sink counter と before/after | T3 |
| T5／3 互換・永続化 | AC-4、INVAR-1／2、INV-1b／2／3、TR-1／4／5 | store parity／row mapping、AppService game／Dome、hint retry、Runtime restart と private scope を確認。必要な regression test のみ追加 | Memory／SQLite、同 timestamp、複数 room、再取得／restart、Dome shared writer の契約が維持 | 下記 targeted tests、game persist、共有 writer 変更に応じた Dome scenario | T3、T4 |
| T6／3 検証・記録 | AC-1～4、INVAR-1／2、全INV／TR | path別 validation、`docs/progress/2026-09-09-942-score-game-projection-freshness.md`（新規予定）へ before/after・判断・証跡を集約。実装した freshness 契約を ADR 0006 に最小限追記 | AC/INVAR→Task→test/evidence の孤立0、必須検証の成功／失敗／未実行を区別 | diff、test結果、inventory適合／未分類、正本文書との対応 | T5 |
| T7／4 独立監査・完了判定 | AC-4、INVAR-1／2、全INV／TR | PR 作成が依頼された段階で固定 PR head を別担当または別コンテキストで監査。実装時の自己点検と分離 | PASS＋必須CI成功。merge承認がある場合のみmergeし、対象tree一致／delta監査後にIssue更新・Close | commit、Scope revision、全入口／sink逆引き、blocker0。PRは `Refs #942` | T6、PR作成／mergeの依頼範囲 |

## 検証コマンドと選定

実装開始時は既存 CLI contract を1回実行し、T1 の制御可能 test の失敗を先に保存する。その後は修正した同じ条件を再実行し、次の関連検証へ広げる。下記は実行予定であり、今回の PASS 報告ではない。

```powershell
cargo test -p kukuri-cli --test content_live game_lifecycle_rejects_invalid_roster_without_mutation -- --exact --nocapture
cargo test -p kukuri-cli --test content_live game_update_by_non_owner_is_rejected_without_changing_replicated_state -- --exact --nocapture
cargo test -p kukuri-app-api --lib game_projection_freshness -- --nocapture
cargo test -p kukuri-app-api --lib tests::game -- --nocapture
cargo test -p kukuri-app-api --lib tests::dome_listing -- --nocapture
cargo test -p kukuri-store game_rooms_match_between_backends -- --nocapture
cargo test -p kukuri-store game_room_roundtrip_preserves_all_columns -- --nocapture
cargo test -p kukuri-desktop-runtime restart_restores_game_room_manifest -- --nocapture
cargo xtask scenario desktop_smoke_game_room_persist
cargo xtask rust-test
cargo xtask oversized-files
git diff --check
```

- `game_projection_freshness` は T1 で追加する module 名の予定。他は調査した既存 symbol／scenario。新規 test filter が実際に test を実行したことを件数と名前で確認する。
- `crates/app-api/**` の必須は `cargo xtask rust-test`。store の永続化挙動も変更する場合は game persist scenario を必須とする。本件では再取得／restart の証拠として app 層だけの変更でも同 scenario を実行する。
- 共有 store writer／Dome 経路へ変更が波及した場合は `tests::dome_move`、`desktop_smoke_metaverse_dome_persist`／`desktop_smoke_metaverse_dome_move` も実行する。private／hint／failure の新規 test 名は T5 で evidence に固定する。
- `game_room_score_update_replicates` は `iroh-integration-tests` feature が必要であることを確認済み。`cargo test -p kukuri-app-api --features iroh-integration-tests game_room_score_update_replicates -- --nocapture` で実 peer の互換性を確認する。0 tests を成功証拠にしない。実行前の追加確認で、`xtask/src/rust.rs` の feature 指定は `app-api-slow-test` 専用と判明したため、通常の `rust-test` と分けてこの targeted command を実行する。
- ADR 0006 に記載の `late_joiner_backfills_game_room_manifest` は今回の `crates` 検索では同名 test を確認できなかった。T5 で既存 replication test の実際の sequence に late join 相当の検証があるか確認し、不足分のみ freshness test または対応 contract に補う。
- payload／UI／Tauri、docs-sync の実装、connectivity を変更しない想定。変更が必要になれば REFACTORING.md の対応行を追加適用する。
- `game.rs` は現時点で990行であり、1000行以上への増加は oversized-files の対象。無関係な分割を混ぜず、必要な helper を既存責務に置く。baseline 緩和を黙って行わない。
- PR前には可能なら `cargo xtask check` + `cargo xtask test`。重い検証の中断／未実行は開発手順と REFACTORING.md に従って理由・完了済み範囲・CI等での補完方法を記録する。

## 未決事項と次の一手

- 計画ファイルの確認: repository 内リンクの存在確認と `git diff --check`、新規ファイルの `git diff --no-index --check` を実施し、エラーなし。製品 test の実行証拠とは区別する。
- 既知: 無条件 upsert の2つの hydration writer、拒否が保存前に行われること、同 timestamp の既存 parity 契約。
- 作業仮定: blob await による旧 record 遅着を T1 で制御できる。T1 が成立しない場合は、固定した入口の trace に調査を戻す。
- T2で決めること: room 単位の共有排他の配置と全 writer の包含、canonical照合後の再取得／fallbackの扱い。比較と書込みを別々の非atomic操作で終わらせない。
- ユーザー判断が必要な条件: 調査の結果、固定した API／wire／schema／時刻／Metaverse の対象外を変更する必要が判明した場合のみ。現時点で計画作成を妨げる質問はない。
- 次の一手: 最終PR headの独立監査と必須CIの成功を確認し、承認済みのマージ・対象tree照合・Issue Closeを行う。追加発見は Existing-gap／Regression／New-requirement／Optional-hardening に分類し、固定 scope にない要件を完了条件へ追加しない。
