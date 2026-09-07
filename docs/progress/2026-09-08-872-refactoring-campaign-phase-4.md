# Issue #872 Phase 4: 親完了監査と比較baseline

## 目的・固定範囲

- Scope revision: `2026-09-04-issue-872-refactoring-campaign-v1`、区分C。
- Phase 4開始・製品実装完了commit: `e98d2025e01332a6dbd4b13f5f4712654a931c65`。Phase 0の`ac9946d6`と区別する。
- #919〜#928の完了済み10子と20候補を親AC/INVAR/INV/TRへ対応させ、machine-readable baselineを確定する。製品コード・新しい候補の実装は対象外。
- #873の判定command、schedule、Issue upsertは後続Issueが所有する。今回のbaselineはその入力であり、実行機構ではない。
- 詳細な実施記録は[Phase 3](2026-09-08-872-refactoring-campaign-phase-3.md)、分類と開始inventoryは[Phase 0〜2](2026-09-08-872-refactoring-campaign-phase-2.md)を再利用する。過去記録の当時の未実施記述は書き換えない。

## 作業と受入確認

| ID | 親AC / INVAR / TR | 対象と受入確認 | 検証 | 依存 |
| --- | --- | --- | --- | --- |
| T1 基点と全記録の照合 | AC1〜4、全INVAR、TR1〜4 | 全10子のhead/merge/CI/監査と6groupを再対応。contract後のcomposition同期はdelta監査に対応 | git tree/path比較、Issue/PRの固定evidence | 完了済みPhase3 |
| T2 構造・大型ファイル再測定 | AC4/5、INVAR3 | owner/site/依存の4成果、scope selectorのmember数/digest、現行16大型fileの実測とratchetを記録 | `cargo xtask oversized-files`、git/正規表現による再計測 | T1 |
| T3 baselineとschema | AC5、TR5 | `xtask/refactoring-audit-baseline.json`とJSON Schema、採用signal/初期値/閾値/根拠・更新規則を固定 | Draft-07 schema、Git commit/集合/digest/metric整合、否定入力の拒否 | T2 |
| T4 親の独立監査 | 全AC/INVAR、INV1〜6/TR1〜5 | 製品group監査とbaseline/完了記録の別担当確認。不適合/未分類/blocker0 | 不変surfaceの既存PASSは再利用し、組合せdeltaを照合 | T1〜3 |
| T5 公開・完了同期 | AC5/TR5 | PR最終head監査/必須CI後にmerge、対象一致後に親Closeと#873への基点引継ぎ | PR/head/tree、Current statusと参照の同期 | T4 |

## baselineの判断

比較起点は既にmain祖先である製品実装完了commit `e98d2025`に固定する。baseline自身のcommit hashを自分の内容へ埋め込む循環を作らず、Phase4記録/設定だけの最終merge SHAはIssueの完了記録で別に保持する。source treeと実測の対象はこの固定commitであり、その後の未監査製品変更を消費しない。

自動判定用の初期signalは、既存規範が開始triggerとして指定するratchet登録pathの新規追加と許容行数の増加に限定する。両方の差分初期値は0、閾値は`>0`。現在行数の美観目標や任意のcommit数を閾値にしない。hotspot/変更量は比較可能な観測データとして残すが、自動発火には採用しない。release/milestone/変更摩擦/sunset等はhuman-onlyとして引き継ぐ。

既存`xtask/oversized-baseline.json`は変更しない。実測と許容上限の差（縮小note3件）は区別して記録する。これはratchetの緩和でも、実測を上限と同一視する変更でもない。

## 検証の選定

製品差分は0。親が明示要求するoversizedと全記録の整合、追加baseline/schemaの検証、独立親監査を実施する。子の同一source treeへ対応するRust/CN/UI/nativeのPASSを再利用し、ローカルで全suiteを再実行したとは記さない。`xtask/**`追加で起動する既存CIは変更・免除せず最終headで確認する。

Phase 3で記録したnative長押し変位/fullmode中移動等の未測定、変更前から再現したfullscreen停止flagを保持する。今回のRegression/固定scopeのblockerと混同せず、修正済みとは記録しない。

## 最終結果

### 親AC/INVARと独立監査

| 条件 | 結果と証拠 |
| --- | --- |
| AC1 / TR1 | #871 merge `f933a286`→開始`ac9946d6`→製品完了`e98d2025`の祖先関係と開始baselineを確認 |
| AC2 / INVAR3 / TR2 | 全20候補は実施4・延期9・却下5・別種別2、固有ID20、未分類0。理由をcompletion baselineにも保存し、延期/却下を実装へ戻していない |
| AC3 / INVAR2 / TR3 | #919→920、#921→922、#923→924、#925→926の契約先行・before PASS・独立監査・単独PR/rollbackを再確認。#927 fix/#928 docsは明示別種別 |
| AC4 / INVAR1 / TR4 | 全10子Closed/PR merged、最終CI/独立監査、head→merge対象一致を確認。4構造成果を再測定。初回FAIL/INCONCLUSIVE/CI failureと解消後delta PASSも保持 |
| AC5 / TR5 | [completion baseline](../../xtask/refactoring-audit-baseline.json)、[schema](../schemas/refactoring-audit-baseline.schema.json)、本記録をreview対象に固定。最終PR CI・merge SHA/対象一致は[親の完了記録](https://github.com/KingYoSun/kukuri/issues/872)に結び、成功後だけCompleteにする |

root実装担当とは別の担当が固定Issue/source/記録から再構築した。INV1/2/5、INV3/4、INV6/baselineへ分け、製品6groupは適合6・不適合0・未分類0・具体blocker0。source依存の計測scopeが最初は曖昧だったため`measurement_scope`へstruct名を明記し、独立再計測6→2とschema否定検証で解消した。最終PR headの確認はPR commentsに保存する。

全audit head→mergeの変更対象pathが一致。merge後の変化は#919の通知contractが#920で受けたcomposition同期1fileだけで、assertion/spies/payloadを変えず#920の監査対象に含まれる。他の対象pathはmerge→`e98d2025`も一致し、組合せdeltaに新しいstate/guard/IPC変更はない。

### 責務・依存・scopeの再測定

| 対象 | before | after | 現行ownerと維持した境界 |
| --- | ---: | ---: | --- |
| 通知取得owner file | 2 | 1 | `useNotificationLoaders.ts`、facade生成1。badge/inboxの異なるpolicy、effectsのevent/interval lifecycle、既読IPCを保持 |
| CN transferのbundle/assign/activate各site | 各2 | 各1 | private `complete_community_node_dome_transfer`、caller2。prepare/no-op・同意再guard・owner保存→remote activate順を保持 |
| sessionのattempt直接代入 | 9 | 0 | `useDomeTransitionAttempt.ts`へ9site、mutable attempt非公開。sequence/参加状態はsession、既存recovery/ack後handoff/cleanupを保持 |
| source解決の到達service依存 | 6 | 2 | `ingest/source.rs::SourceResolver`のDocsSync/optional BlobService借用だけ。scan/metrics/2storeはpipeline、body/media各1入口は元位置 |

beforeは`ac9946d6`、afterは`e98d2025`。JSONにpath/regexとwhole-file/struct-field scopeを保存。行数総和や公開入口数の削減を目的にしていない。API/DTO/route/CSS、signature/wire/identity/schema、authority、P2P三経路はrefactorで変更しておらず、#927だけを承認済みfixの挙動変更として分ける。

| 親group | 開始member | 完了member | 追加・分類 |
| --- | ---: | ---: | --- |
| INV1 | 177 | 177 | tree無変更 |
| INV2 | 309 | 311 | Dome listing/transferのtest追加 |
| INV3 | 539 | 542 | 通知contract/通知owner/attempt owner追加 |
| INV4 | 65 | 68 | attempt owner、Dome listing、transfer contractの横断集合 |
| INV5 | 262 | 265 | source contract/support/resolver追加 |
| INV6 | 402 | 424 | 実行・監査・比較記録と画像。harness/test-support/scenario/xtask/CI実装は無変更 |

groupは重複を含む全tracked集合で、合計を重複なしfile数にしない。別のscanner filter候補は1,556 path。再生成は`git ls-tree -r --name-only <commit>`と保存selector/filterによる。pathをUnicode code point順でsortし、UTF-8/LF結合＋末尾LFのSHA-256を保存し独立再計算で一致した。

### 大型ファイル・初期signal

`cargo xtask oversized-files`はPASS、16件、violation0。全候補を独立走査して16件と一致した。

| path | 開始実測 | 完了実測 | 既存許容上限 |
| --- | ---: | ---: | ---: |
| `useMetaverseRoomSession.ts` | 1272 | 1086 | 1272 |
| `private_channels_game_api.rs` | 1100 | 1093 | 1100 |
| `runtimeApi.ts` | 1035 | 1035 | 1042 |

全16件をJSONに保存。runtimeApiの差7行は開始時からのnoteで、今回の改善へ算入しない。残る13件は実測/上限一致、上限緩和0。

初期signalは`ratchet_new_paths`と`ratchet_increased_caps`、値各0、`operator=gt / threshold=0 / combination=any`。新規登録と既存上限の増加だけを監査検討へ渡し、削除/縮小は数えない。根拠は規範のratchet増加triggerで、現在行数の単一美観閾値ではない。

参考履歴は`2026-05-29T00:00:00Z`以後566 commits。通知effects16、section loader13、room session15、runtime game API16、ingest12、runtimeApi25のpath変更commit。25変更のruntimeApiを延期した事実も踏まえ、commit数/経過日/変更量だけの自動閾値は採用しない。baselineからの差分初期値0は観測用として保存し、release/milestone/変更摩擦/sunset等をhuman-onlyで残す。

### Validation・制約・引継ぎ

- Draft-07 schemaを既存Ajv6.15で検証。現baseline PASS、missing commit/bad SHA/未完baseline/未知schema版・field/異なる閾値/負数/group不足/path traversal/未分類/子CI失敗の11否定入力を拒否。独立担当はAjv6.15とAjv8.18＋既存ajv-formatsでもschema/current/否定入力を検証。依存変更0。
- Git commit/祖先、10 code OID、6group count/digest、1,556候補、16実測/上限、6metric、20候補、10子のhead/merge/CIを独立照合した。JSON Schemaだけでgit/counterの真実性まで証明したとはしない。
- Rust/CNは#926 head `cedfad64`と完了基点の`crates` tree `9dd9733d9155fa022b77310399ecebfee73f792c`、Cargo/Tauri/harness/xtask実装が一致しFast34156245820を再利用できる。UIは#924 Fast34153376771、full UI152 files1209 tests/browser64/visual smoke14と対象treeを対応。slow199 PASSは#927、個別before/after/独立再実行はPhase3に対応する。
- Phase4差分でローカル全Rust/CN/UI/実機suiteを再実行していない。製品変更0で不変の既存PASSを再利用する。起動する最終PR CIは成功後に親evidenceへ記録する。
- 初回Fast34164485657のRust testsはCLIの`game_lifecycle_rejects_invalid_roster_without_mutation`で1失敗（651 PASS、652/910で停止）。Running/score7から、無効roster拒否後のreadが古いWaiting/score0/manifestへ戻った。[元job](https://github.com/KingYoSun/kukuri/actions/runs/34164485657/job/101872714365)と[別fix #942](https://github.com/KingYoSun/kukuri/issues/942)へ観測と未確定の実行順を保存した。local同test1回＋上限20回は全PASSで、CI failureを再現済みとはしていない。
- ScoreGame create/update/validator、CLI、hydration、cache writerは開始SHAから同一。旧recordをblob await後に無条件upsertできる既存経路を確認したが、当該実行のwriter順は未確定。Phase4の製品差分0、#927のMetaverse分岐もこのrowは通らず、今回のRegressionを示す証拠はない。元20候補を拡張せず、既存問題の観測として引き継ぐ。製品/testを変更・弱体化してgreenにせず、最終headの必須CIが成功するまでは親完了を保留する。
- nativeの未測定範囲と既存fullscreen停止状態は[実機record](../ui-reviews/2026-09-08-issue-924-transition-owner.md)を保持し、全native動作PASSや既存UI不具合の修正済みへ拡大しない。
- 現行architecture/runbook/ADR/root規範に旧ownerの現役扱いは見つからなかった。docs/READMEへ完了記録の導線を追加する。#927のCurrentに残った「原因診断未着手」は追跡同期で除去する。

初期実測は`e98d2025`の値であり、Phase4の記録/schema/baseline追加後の最新tree件数とは区別する。初回checkで記録追加分の差分が見えても採用ratchet signalは0のまま。main配置と親Complete後だけ確定入力として採用し、#873のcommand/workflow/upsert/scheduleをこの作業で実装・有効化しない。

RollbackはPhase4のbaseline/schema/導線PRだけを戻す。製品の個別revert手順は保持する。baselineを戻す場合は#873の採用も解除し、未監査commitへ自動前進させない。
