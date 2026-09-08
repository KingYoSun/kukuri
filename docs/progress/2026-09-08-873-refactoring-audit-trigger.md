# #873 リファクタリング監査トリガーの継続チェック

## 状態・固定範囲

- 記録時点の状態: 実装済み、全体validationの一部が実行中。固定PR headの独立監査・CI・merge後確認の最終結果は [#873 Current status](https://github.com/KingYoSun/kukuri/issues/873) と対応PRのコメントへ結ぶ。この記録だけで未完了工程をPASSと判断しない。
- Scope revision: `2026-09-04-issue-873-refactoring-audit-trigger-v1`。
- リスク区分: C。正本は [#873 Current status](https://github.com/KingYoSun/kukuri/issues/873) のAC-1〜5／INVAR-1〜3／INV-1〜5／TR-1〜5。
- 実装開始HEAD: `188c1df4ab81301a6eafb40645ffb24a4aaa931e`。
- 比較起点: `e98d2025e01332a6dbd4b13f5f4712654a931c65`。#872のbaseline公開mergeと混同せず、baseline JSON／schema／ratchet設定は変更していない。
- 依存: #871が規範、#872が初期baseline、本Issueが継続check／Issue集約を所有する。生成Issueは読み取り中心の有限監査から始め、個別実装を自動で開始しない。
- 今回の承認: 計画と実装、commit／PR／mergeまで。区分Cの独立監査PASSと必須CI成功をmerge条件とする。

## 実装判断

1. 判定の正本は `scripts/refactoring-audit/check.py`。Python標準のGit subprocess／regex／日時処理と固定した `jsonschema` のDraft-07検証を使う。Rustの `refactoring_audit_check` はshellを介さず同じ処理へ引数を渡す。Python sourceは常に実行するcheckoutに固定し、baseline・製品情報は評価commitのGit objectから取得する。
2. 既存xtaskのharness／tokioを既定有効featureへ移す。週次は `--no-default-features` で軽量xtaskだけをcompileする。標準commandとscenarioの既定挙動は維持する。依存追加はPython validatorのみで、Rust lockfileは不変。
3. 自動signalは保存済み2項目、各 `> 0`、any。現在のratchetに対する新規path数／上限増加件数を保存上限から計測する。commit数・hotspot等を新しい閾値へ昇格させない。
4. schemaに加え、commit／祖先、code tree、source policy blob、候補集合、groupのbefore/after集合とdigest、ratchet snapshot、実測行数、計測scope付き構造metric、候補分類を検証する。子IssueのCIや製品テストを週次で再実行しない。
5. `workflow_check.py` がforce／reasonを引数へ渡し、成功したJSONのみをrunner temp、boolのjob output、summaryへ公開する。checkにtokenを渡さない。入力理由をshellやworkflow commandへ埋め込まない。
6. Issue writeの正本は `upsert.mjs::upsert`。全ページのOpen marker検索、PR除外、重複検出、機械領域更新を一か所に置き、REST adapterは自動retryしない。workflow全体を共通concurrency groupで直列化する。人間のscope・判断本文と機械観測を分け、scopeを自動拡張しない。
7. 週次workflowはmain限定、checkはcontents:read、upsert jobだけissues:write。既存nightly／required check設定は変更せず、変更時のtestは別の `Kukuri Refactoring Audit Tests` に置く。

当初計画の新規Rust判定module案は、軽量実行とbaselineに保存されたPython互換regexの再現性から「Rust入口＋Python判定」に具体化した。fixture／CLI／schema testsはPython、API mockはNodeで実装し、Rustは既存xtaskの回帰とfeature分離を検証する。固定scopeや品質条件の削除はない。

## 変更前の証拠

### Feature Data Classification

- Feature 名: 開発支援用のリファクタリング監査発火判定・Issue集約。
- Durable / Transient: reviewed baselineとGitHub Issueはdurable。計測結果は再生成可能で、workflow間のartifactは7日保持。Actions summary/logはrepositoryのActions保持設定に従う。tokenはwrite stepの環境変数のみ。
- Canonical Source: baseline/schemaと比較対象のGit commit。監査scope・人間判断はtracking Issue。判定はこれらを自動変更しない。
- Replicated?: kukuriのreplication対象外。GitHubにはcommit ID、技術的なpath／signal、手動理由、監査の記録を送る。製品利用者の投稿・identity・秘密値は入力にしない。
- Rebuildable From: baseline/current commit、評価日時、force/reasonからJSONとMarkdownを再生成する。
- Public Replica / Private Replica / Local Only: 製品replicaなし。ローカルではGit read/stdout、workflowではGitHubのrepository権限内でsummary／Issueを扱う。
- Gossip Hint 必要有無: 不要。
- Blob 必要有無: 製品blob不要。Git object readとActionsの一時artifactのみ。
- SQLite projection 必要有無: 不要。
- 必須 contract: 計画で固定したGit fixture、schema／意味検証、no-write、実CLI exit、API mock、workflow adapterの契約。
- 必須 scenario: 製品scenarioの追加なし。Git履歴→判定→summary→guarded upsertを専用契約testで確認する。

CodeGraphで `xtask/src/main.rs`、`oversized.rs`、`scenario.rs`、`rust.rs`、`exec.rs::artifacts_dir` とcallerを確認した。開始HEADにはcommandと専用workflowがなく、baselineとschemaだけが存在した。

先にGit fixture契約を置き、`python -m unittest discover -s scripts/refactoring-audit -p test_check.py` が `check.py` 未実装で失敗した。upsertも契約testを先に置き、`node --test scripts/refactoring-audit/upsert.test.mjs` がmodule不存在で失敗した。実装後、同じsequenceを正常／異常／禁止writeで検証した。

## AC / INVAR と証跡

| 条件 | 実装・検証 |
| --- | --- |
| AC-1 / AC-5 | #872のbaseline/schemaは維持。`validate_baseline` のschema・集合・metric検証、`test_baseline_corruption_semantics_and_unknown_schema`、`test_reviewed_baseline_update_is_accepted_without_automatic_advancement`、runbookのreviewed更新contract |
| AC-2 | `check` / `render`、Git fixtureで未commit差分排除・再現性、各signal単独／複合、真偽、JSON／Markdown、実CLI exit。`test_threshold_boundaries_and_invalid_negative` は0閾値に存在しない未満ケースを正のfixture閾値で検証 |
| AC-3 | `workflow_check.main`、週次／manual専用workflow、main限定、force理由必須。`test_arguments_preserve_manual_input_and_outputs_are_fixed`、`test_failure_never_publishes_success_output_or_artifact`、actionlint |
| AC-4 | `upsert` のfalse早期return、marker全ページ検索、create/update/idempotency、複数match拒否、検索失敗とcreate応答喪失後rerunのAPI mock |
| INVAR-1 | 判定はGit object readのみ。workflow adapterのwriteはrunner temp/output/summaryのみ。upsertは機械領域だけを更新。baseline／product codeのwrite callerなし |
| INVAR-2 | trueもexit 0。既存nightlyとrequired設定は差分0。`cargo tree -p xtask --no-default-features --depth 1` にharness／tokio／製品crateなし。週次で製品build／testなし |
| INVAR-3 | check/write job権限分離、mainの同一SHA checkout、tokenはwrite stepのみ、checkout credentials非保持。理由・pathを引数／JSONとして渡す。REST adapterのJSON送信・no-retry test |

## 固定surface inventoryとsink逆引き

| ID | 入口・trigger／shared helper | 読み書き・sink／guard | transitionと証跡 |
| --- | --- | --- | --- |
| INV-1 | `main.rs` command → `refactoring_audit_check` → `check.py::main/check/validate_baseline`。local／workflowは同じ入口 | git読込／stdout。異常では結果を出さずnonzero。HEAD/index/worktree/baseline不変 | TR-1〜3。Git fixture・real CLI・dirty/untracked・schema/metric/非祖先/取得失敗 |
| INV-2 | schedule／dispatch → `workflow_check.main` → 上記command | main限定、成功した結果だけtemp artifact/summary/bool output。通常contents:read | TR-1〜4。adapter正負test、actionlint、専用CIの実repo評価 |
| INV-3 | check job成功かつtrue → upsert job → `upsert.mjs::main/upsert` → `githubApi.create/update` | marker検索後だけPOST/PATCH。共通concurrency、複数match／管理領域破損はwrite 0。人間領域は保持 | TR-2/4/5。create/update/pagination/idempotency/duplicate/search失敗/応答喪失/403 |
| INV-4 | `audit_required=false` → summary。`upsert(false)`も早期return | Issue API呼出0、baseline/commit変更0 | TR-1。Python false adapterとNode no-write test |
| INV-5 | 完了監査後のreview付きbaseline更新PR → 次回check | check自身にbaselineの書込みなし。schema＋意味検証、更新手順と監査証拠のreview | TR-5。reviewed update fixtureとread-only前後比較 |

追加された製品入口は0。新規入口はINV-1 command、INV-2のschedule/dispatch、INV-3共有upsert。既存 `artifacts_dir` のcallerは `scenario.rs::e2e_smoke` のみで、scenarioと同じharness featureへ限定した。

Issue mutation sinkの逆引きは `githubApi` のPOST/PATCH → `upsert` のcreate/update分岐 → CLI main → 専用workflowのupsert step。testからの呼出はmockのみ。全実writeがschema/true/marker guardを通る。false／判定errorはworkflow jobもskipし、API検索失敗／複数markerはcreate分岐へ進めない。retryは新しいworkflow runとして検索から再開し、同一run内の無条件再送はない。

## Validation

実施済み:

- Python fixture／実xtask CLI／workflow adapter／実JSON→Node接続: 計18 tests PASS（Windows、追加・変更箇所はtargeted再実行）。初回のWindows fixtureで改行を含むファイル名が作成不能だったため、Windowsは空白／日本語／shell類似文字、Linuxは追加で改行を含むpathを検証するよう修正した。
- Node API mock／REST adapter: 10 tests PASS。
- `cargo test -p xtask --no-default-features`: 44 PASS、既存AppImage署名metadata test 1 ignored（今回追加のskipなし）。
- `cargo clippy -p xtask --all-targets --no-default-features -- -D warnings`: PASS。
- actionlint 1.7.7: 専用2workflow PASS。
- 現行baselineの実Git評価: 2signalとも0、false、exit 0。比較起点と公開mergeを区別したJSON/Markdownを確認。

- `cargo xtask check` PASS（non-CN Rust clippy、operator-neutrality、Tauri compile、frontend lint/typecheck）。
- `cargo xtask test` のnon-CN／非harness部分: 886 PASS、既存skip 4。harness／doctest／frontendは記録時点で未完了。
- `git diff --check`、新規文書の相対リンク、Python依存整合はPASS。

記録時点の未完了／再実行対象:

- 上記全体testの残り、PR CI、PR head独立監査、merge後確認。
- `cargo xtask oversized-files` は全体testと並行したbuildで、実行中 `target/debug/xtask.exe` の置換がWindowsのファイルロックに拒否された（OS error 5）。ratchet判定には未到達。全体test完走後に同commandを再実行する。製品の検証失敗や成功へ読み替えない。

## 独立監査・完了条件

固定PR headとAC/INVAR evidenceを別担当へ渡し、全INV/TR、Issue sinkの全caller、false/error/retry/duplicate、権限、baseline非更新を独立確認する。FAIL/INCONCLUSIVEは解消し、差分変更後はdeltaもPASSにする。PRは `Refs #873` とし、必須CI成功と独立監査PASS後にmerge、対象tree一致を確認してからIssueをComplete/Closeする。

## 残存事項

既存baselineの採用signalや規範は変更していない。新規閾値、LLM判定、Issue乱造、製品refactor、既存required CIの緩和は対象外。現時点の未完了項目は上記validationと独立監査／merge工程であり、成功済みとして扱わない。
