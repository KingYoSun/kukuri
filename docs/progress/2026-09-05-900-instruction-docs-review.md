# #900 開発指示書の棚卸しと整理

## 対象とPR提出時点の判定

- リスク区分: A（開発指示・設定・雛形のみ。製品契約、製品コード、依存、CI必須設定は変更しない）。
- Scope revision: `2026-09-04-instruction-docs-review-v1`。
- 基準commit: `a1c2696b6f65aa33375e9eb99d1cd392c8e23139`。着手時のlocal mainとorigin/mainの一致、作業treeに既存変更がないことを確認した。
- 作業範囲: ユーザーが承認したT1〜T4。PR作成とCI成功後のマージも依頼済み。
- 記録時点の判定: 文書更新・代表例・機械確認済み。PR headのCIとマージ後照合は提出後のT4で行う。この節と下記判定はPR提出時点の証跡として保持する。
- 独立監査: 区分Aのため不要。製品の状態遷移・sensitive sink一覧も対象外。
- 正本: 更新後の[文書案内](../README.md)、[Issue運用手順](../runbooks/issue-lifecycle.md)、[PLANS.md](../../PLANS.md)、[REFACTORING.md](../../REFACTORING.md)。本書は作業証跡であり新しい規範ではない。

## 承認済み作業と条件

| Task | 対象条件 | 作業・成果物 | 確認 |
| --- | --- | --- | --- |
| T1 | AC-1・2、INVAR-4 | 初期15ファイルと直接参照から補足した8ファイルを棚卸し。正本・適用範囲・発見事項を本書に記録 | 下記の対象一覧とDOC-1〜11の判定 |
| T2 | AC-3〜6、INVAR-1〜4 | 正本の規則、入口、要約、雛形、履歴への案内を同じ差分で同期 | 発見事項と文書差分の対応、製品・CIへの差分なし |
| T3 | AC-7、AC-3〜6、INVAR-1〜4 | 代表例6組に対する適用結果と機械確認 | 下記の代表例、リンク・path・command・設定構文・ミラー検査 |
| T4 | AC-8、全AC / INVAR | PR headのCI結果、マージ後のtree一致、Issue現在判定を照合 | PRとマージの証跡。未完了部分は完了扱いにしない |

## 対象一覧・参照関係

`git ls-files`で追跡対象を列挙し、AGENTS / CLAUDE / GEMINI / COPILOT、rules / instructions、
`.codex/` / `.claude/` / `.cursor/` / Kilo設定の入口を確認した。追跡される下位AGENTS等はなく、
追加のツール別ファイルは下記の`.codex/plans/`3本だった。非追跡ファイルを現行の共通規則へ昇格しない。
`rg`で対象文書名への参照元も確認し、文書・設定は直接読んだ。既存の機械検査を探す際はCodeGraphを先に使い、
目的の検査が返らなかった箇所を検索で補った。全ADR・全履歴の再監査は行わない。

各行にDOC-1〜11を適用した。発見事項は次節のF番号で示す。それ以外は表の役割・維持理由に照らして変更不要と判定した。
「初期」はIssueの15ファイル、「追加」は対象規則へ直接関わる8ファイル。支援する実装・設定は表の後に区別する。

| 対象 | 役割・適用条件と正本への経路 | 判断 |
| --- | --- | --- |
| 初期: `AGENTS.md` | 人間・AIの共通入口 → docs/README → 作業別規則。通信経路の定義は既存ADR・architectureから参照される | F1〜5・7。短いポインタへ整理。通信経路・console.error禁止・必要最小限の移植規律は維持 |
| 初期: `CLAUDE.md` | Claudeの入口 → AGENTS | F1。追跡された共通ファイルをimportし、任意ファイルの無条件importを解消 |
| 初期: `AGENTS.local.md.example` | 個人設定を作りたい人向け。共通規則はAGENTSへ委譲 | F1。CodeGraph規則の複製を除去。個人の実ファイルは編集しない |
| 初期: `kilo.json` | Kiloのinstructions → AGENTS → 存在時だけlocal | F1。必須読み込み先を追跡されたAGENTSに変更。schema URLは不変 |
| 初期: `PLANS.md` | 計画の要否・粒度・記録形式。承認と工程はIssue運用手順へ委譲 | F2・3・4・8・12。長い一般論、重複例、禁止見出し・件数の強制を整理 |
| 初期: `REFACTORING.md` | 構造改善の凍結境界・成果物。検証マトリクスと中断は全変更で適用 | F4・6・7・8・11・12。固有の凍結境界・互換パス・ratchet・参照逆引きは維持 |
| 初期: `DESIGN.md` | UI製品・視覚契約。作業手順はADR 0014、配置はUI実装配置 | F8・9。1回利用helperの一律制限を目的で判断。token表・marker・製品設計値は維持 |
| 初期: `docs/adr/0014-uiux-dev-flow.md` | UIの分類・証跡・確認・例外。共通承認・再現はIssue運用手順、製品契約はDESIGN | F4・5・7・8・9・12。重複した順位・実装規則を正本への参照に変更 |
| 初期: `docs/architecture/desktop-ui-implementation.md` | CSS cascadeとstate / dataの配置。DESIGN・ADR 0014の要約を含む | F9・11。要約の正本と同期を明示。配置とportal / shell差の理由は維持 |
| 初期: `docs/README.md` | 現行スコープの読む順、正本の責務、変更要求と履歴の扱い | F2・3・9・10。読む順番と規則の優先関係を分離 |
| 初期: `docs/runbooks/dev.md` | commandの実行方法、Linux / Windows等の環境差 | F6・7。検証選定表の独立管理を止め、REFACTORINGへ案内。既存commandと視覚baseline規律は維持 |
| 初期: `docs/runbooks/issue-lifecycle.md` | 人間・AIの共通工程、承認、再現、A/B/C、scope固定、監査、Close | F2・4・5・7・12。Aの省略と承認済み実行を明確化。B/Cのinventory・禁止I/O・監査条件は維持 |
| 初期: `.github/ISSUE_TEMPLATE/engineering-change.yml` | 実装Issueの入力補助 → Issue運用手順 | F2・4・7。AでB/C専用欄を省略可能にする。B/Cの必須性は説明と必須確認欄で維持 |
| 初期: `.github/PULL_REQUEST_TEMPLATE.md` | PR証跡の入力補助 → Issue運用手順・ADR 0014・REFACTORING | F4・5・7・9。非該当欄の削除、実機再現、既存証跡への参照を同期。未確認と監査欄を維持 |
| 初期: `docs/ui-reviews/README.md` | UI採用記録のschemaと履歴。追加条件の正本はADR 0014 | F7・9。追加条件の重複管理を参照へ置換。過去の証拠本文は不変 |
| 追加: `.github/ISSUE_TEMPLATE/preview-feedback.md` | 利用者向けの報告入口。実装担当者が運用手順に従い条件を補う | 維持。期待結果・再現と秘密情報を貼らない注意は必要。利用者に開発者用の全欄を要求しない |
| 追加: `docs/progress/2026-04-16-mvp-builder-preview-plan.md` | 現行マイルストーン状態 → 個別ADR・runbook | 維持。状態の読む順を規則の優先順位にしないREADMEの明確化で足りる |
| 追加: `docs/progress/2026-03-10-foundation.md` | capability baselineと当時の実行証跡 | 維持。日付付き実績を今回の検証結果と混同しない。新しい共通規則ではない |
| 追加: `docs/progress/2026-03-24-shell-ui-production-migration.md` | 現行案内から参照される移行履歴。Applicable Rulesに廃止済みFigma指定が残る | F10。該当節だけSupersededと後継先を追記し、過去本文は保持 |
| 追加: `docs/progress/2026-05-27-kukuri-ai-refactoring-environment-improvement-plan.md` | AGENTS・REFACTORING導入時の規則案・雛形 | F10。旧規則案の失効と後継を冒頭に明示。過去の本文は保持 |
| 追加: `.codex/plans/2026-09-02-community-node-auto-approve-removal.md` | 特定の同意・設定整理の計画 | 維持。個別の変更契約であり共通指示の入口ではない。現在の全作業へ形式を強制しない |
| 追加: `.codex/plans/2026-09-03-issue-858-adult-content-safe-display-rerun.md` | #858の固定条件を持つ個別再監査計画 | 維持。製品の表示・境界条件は今回変更しない |
| 追加: `.codex/plans/2026-09-04-issue-855-restore-consent-recovery.md` | #855の復元と再同意に限った個別計画 | 維持。具体的なtransaction / 禁止I/O条件は一般的な形式制約と区別する |

支援する確認対象は `.gitignore`（local設定の非追跡）、`xtask/src/main.rs`（command登録）、
`apps/desktop/package.json`（frontend実行入口）、`apps/desktop/src/styles/design-contract.test.ts`と
`tokens.css`（ミラー）、`.github/workflows/kukuri-fast.yml`（path filter・手動実行）、
`kukuri-terraform.yml` / `kukuri-cn-images.yml`（今回の非該当path）であり、変更しない。
通信経路の参照元であるADR 0010・0025とP2P責務境界文書は定義を移動しないため、製品内容の変更対象にしない。

## 発見事項・対応・理由

変更前の根拠は基準commitの該当ファイル・節で再取得できる。製品bugではないため、以下の
同じ依頼への指示の適用差を変更前の再現証拠とし、恒久的な評価コードや失敗testを新設しない。

| ID / 観点 | 変更前の該当箇所と起こりうるずれ | 対応・理由 | 確認 |
| --- | --- | --- | --- |
| F1 / DOC-1・2・3・10 | AGENTS「まず読む」、CLAUDEの`@AGENTS.local.md`、Kiloのlocalのみのinstructions、local雛形のCodeGraph複製。local欠落時に停止し、Kiloは共通規則へ届かない | 共通入口をAGENTSへ統合。任意localを条件付きで読む。CodeGraphが使えない場合の代替をAGENTSで定義 | 入口のJSON・import・path確認、代表例6b |
| F2 / DOC-1・5・9 | AGENTSのIssue / PR / sessionをSSoTにしない指定が、承認済み変更要求まで無効と解釈される | 既存事実の根拠と今回の承認範囲をREADMEで区別。仕様変更は正本へ反映 | 代表例5、正本表・入口・雛形を照合 |
| F3 / DOC-1・5 | READMEの参照優先順とREFACTORING「古い文書を更新」が、progressや日付だけでADRを上書きする読み方を許す | 読む順と責務を分離。日付や表現の強さだけで決めない | READMEの責務表と各文書の委譲先が一致 |
| F4 / DOC-3・6・8・9 | PLANSの変更禁止とIssueの計画承認gateが実行依頼でも再承認を要求。コミット要求との関係も未明示 | 承認をIssue運用手順へ集約。計画のみは停止、実行依頼は継続、PR依頼は必要なcommit / pushを含む | 代表例5、範囲外判断と必要CIは残る |
| F5 / DOC-1・3・6・9 | AGENTSの自動失敗test必須とADR 0014の再現手順の選択肢が不一致 | 自動化可能なら失敗test、実機・視覚のみなら先に条件と観測証拠。境界testの代替にはしない | 代表例2・3・4、PR雛形も同じ再現規則を参照 |
| F6 / DOC-1・3・6・8 | REFACTORINGのfrontend / Tauri行が包含関係で、backendのみでもUI全体検証が自動追加される | frontend行からsrc-tauriを除外し、IPC等の影響があれば両行を適用。同じcommandは一度 | 代表例3a、凍結境界の個別検証は不変 |
| F7 / DOC-1・6・8・9 | AGENTSの長時間test原則完走とREFACTORINGの重い検証代替が実行前後を区別しない。Aにも多数の対象外理由を要求 | 選定・中断・未確認を分離。実行不能は成功にしない。Aと非該当欄は省略し、必要な未確認は残す | 代表例1・6c、Issue・PR・UI記録の雛形照合 |
| F8 / DOC-4・6・7・8 | PLANSの件数・質問上限とOut of Scope移動、DESIGNの1回利用helper禁止、REFACTORINGの行数による追加分割・同じ意図のrename / move / extractionの分離・全事前testの別PR指定が手段を目的化 | 計画件数は目安。必須要件と重大な未決事項は全件記録。抽象化・分割は現在の責務と検証の必要性で判断し、ratchetは維持。同じ構造上の意図は追跡できればまとめ、小さな同一責務の事前testは変更前証拠付きなら同じPRを許容。挙動変更の混在と凍結境界の省略は許さない | 代表例6a、製品設計値・CI設定への差分なし。変更前証拠と差し戻し単位の条件を照合 |
| F9 / DOC-2・5・7 | UIの判断順位と実装原則、review追加条件、検証選定が複数文書で独立管理。token表は機械ミラーなので同列に削除できない | 正本へ参照し、配置上の短い要約は同期方針付きで保持。token同期testへのリンクを追加 | ミラー既存test成功、marker領域の差分なし |
| F10 / DOC-1・5・10・11 | shell移行履歴のApplicable RulesにFigma指定、2026-05-27導入案に旧形式・旧配置規則が残り再流入しうる | 失効範囲と後継先を追記。日付が古いだけの証拠本文は書き換えない | 2履歴ファイルの差分が追記のみであることを確認 |
| F11 / DOC-1・5・10 | REFACTORINGがCSS配置の説明で存在しないDESIGN 4.7を参照 | 現行UI実装配置のPortalとscoped overrideへ参照を修正 | path・見出し存在確認、実効値差を保つ注意は維持 |
| F12 / DOC-4・6・7・9・11 | PLANS末尾が制約の強化だけを許す。多数の一般論・同じchecklistと報告形式が独立転記を招く。UIの原則1回再確認が絶対回数にも読める | 見直しをREADMEへ集約し効果・負担を根拠に緩和・撤廃も許す。重複例・転記形式を整理。必要な再確認は回数で止めない | DOC別判定、代表例1・5・6、境界固有の品質規則は残る |

## DOC-1〜11の最終判定

| 観点 | 判断と残した理由 |
| --- | --- |
| DOC-1 矛盾 | F1〜7・10・11を整理。UIの実機例外と一般の再現は同じ正本に接続し、残るscope外の製品契約を変更しない |
| DOC-2 重複 | 承認・再現はIssue運用、計画粒度はPLANS、検証選定はREFACTORING、実行方法はdev、UI製品規則はDESIGN、記録schemaはUI READMEに所有を分けた。入口の要約とtokenミラーは同期元があるため保持 |
| DOC-3 適用対象 | 人間・AI、計画のみ・実行依頼、A/B/C、UI変更分類、frontend・Tauri、文書のみ、任意localを明示。一般規則を個別製品計画へ逆流させない |
| DOC-4 自明な指示 | PLANSの良い/悪い例・禁止見出し・一律helper制限、REFACTORINGの重複checklistを整理。境界逆引き・禁止I/O・署名/互換性・portal差・秘密を貼らない注意は具体的失敗を防ぐので残す。AGENTSのconsole.error禁止と移植前contractは既存の開発規律として短く保持し、今回の文章整理を理由に撤廃しない |
| DOC-5 正本 | READMEで読む順と規則の責務を分けた。個別計画は対象Issueの契約、過去記録は当時の証拠。参照の同期を日付の優先だけで決めない |
| DOC-6 必須・推奨・例外 | 件数・再確認回数・分割PRは目的付きの原則。担当者が扱える手順調整と、ユーザーの判断が必要な範囲変更・品質免除を区別。必須CI・C監査の免除はない |
| DOC-7 目的と手段 | 短さ・削除行数・検査scoreを成果にしない。記録形式は使い回せるが、AC / INVARの証拠は省略できない。数値のratchetと製品token値は別の既存契約として維持 |
| DOC-8 規模 | Aは対象・期待結果・確認に縮約。文言だけではcontrastや全製品suiteを無条件追加しない。認証・同意等を規定する文書はCのまま |
| DOC-9 工程 | 承認済み実行の続行、未承認の判断だけの確認、失敗・中断・再開とCI後のマージを明確化。全条件が満たされた後は同じ監査・polishを繰り返さない |
| DOC-10 環境 | local欠落とCodeGraph不能の代替、正本リンク・現行command・重複pathを確認。Windowsの視覚baseline比較skip、Linux生成、CI強制という既存制約は維持 |
| DOC-11 見直し | READMEへ維持・強化・緩和・統合・撤廃の判断を集約。旧規則案には失効を示すが、過去の結果は保持。定期監査基盤は追加しない |

固定した23ファイルの指示適用・参照関係について、未判定事項は0。これは全製品・全履歴に不具合がないという主張ではない。

## 代表例6組の適用確認

以下は更新後の規則を具体的な依頼へ適用した文書上の確認であり、例示した製品変更や実機testを実行した結果ではない。

| 例 | 必要文書 | 判断・承認 | 検証・終了条件と変更前からの解消 |
| --- | --- | --- | --- |
| 1a: runbookの誤字を直す | AGENTS → README → Issue運用A、REFACTORINGの文書確認 | 修正依頼内で進行。独立計画・非該当欄の理由は不要 | 対象文言・参照とdiff確認で終了。全製品testを自動要求しない |
| 1b: 通常UIの局所文言を直す | 上記＋DESIGNの文言、ADR 0014の文言確認 | 表示の修正は依頼内で進行。同意文の意味を変えるならCとして扱う | 対象locale・文言・overflowと関連test / preview。色不変ならcontrast再測定不要。大きなUI review recordは不要 |
| 2: 一つの責務内の通常bugを直す | Issue運用B、関連仕様とtests、対象pathのマトリクス | 固定条件を満たす方法は担当者が判断 | 修正前のsequence失敗test→修正後成功、適用INV / TR・回帰検証を対応。shared guard等の監査条件を確認し、満たせば終了 |
| 3a: Tauri backendのみの挙動変更 | REFACTORINGのTauri行、dev、関連仕様 | pathの選定は担当者。IPCの影響があればfrontendも対象 | tauri-check + e2e-smoke。frontendへ影響しなければ包含pathによるUI suiteの二重適用なし。境界別の追加testは維持 |
| 3b: Windows WebViewだけのnative input bug | 上記＋ADR 0014、Issue運用の修正前再現 | 実機のみの再現理由を示して依頼内で進行。確認環境がなければ該当部分は未確認 | OS / WebView / 入力条件と変更前観測を固定し同条件で再確認。自動化可能な周辺はtest保護。browser成功だけで実機成功にしない |
| 4: 認証境界の変更 | 関連ADR・tests、Issue運用C、PLANS、path検証 | 製品変更依頼の承認範囲に限定。品質免除は推定しない | 入口→helper→sinkと逆引き、適用TRの失敗・retry・restart等、禁止I/O / mutationの0または不変、PR head独立監査PASS + 必須CIが必要。Aの短縮形で迂回できない |
| 5a: 計画だけ作る | PLANS、Issue運用の承認 | 計画のみなので提示で停止 | AC / INVAR、作業、検証、不明点を示し、ファイル変更やPR作成を始めない |
| 5b: 承認済み計画を実行し、PRとCI後マージを行う | 上記＋対象pathの規則 | 必要なcommit / pushまで含めて進行。計画や作業手順の再承認不要。範囲外判断だけ確認 | 対象test・CIと必要な監査を満たしてマージ。変更要求を正本へ反映し、既存事実の証拠と区別する。本Issueの依頼にも適用 |
| 6a: 必須Taskが9件、重大未決事項が4件ある | PLANSの粒度・Issue運用のscope | 件数超過だけの許可取りは不要。scope削減が必要ならユーザー判断 | 統合しても必要なら理由付きで全件記録。重大事項は隠さず、解消が必要な部分だけ待つ。8件/3件へ切り捨てない |
| 6b: local設定がない、CodeGraphも利用不能 | CLAUDE / Kilo → AGENTS、README | 任意localの作成・提供を要求しない。index新規作成は勝手に行わない | 共通AGENTSから継続し、利用不能を記録してrg / ファイル読みへ切替。追跡設定の読み込み先が存在することを構文確認 |
| 6c: 重いtest、資源枯渇、未確認の必須CI | REFACTORINGの選定・中断、dev、Issue運用 | 狭い検証の事前選定は担当者判断。長い再リンクだけで停止しない。資源枯渇等の具体的根拠があれば中断 | 完了範囲・未確認・再開条件を記録し、CI等で補う。中断をPASSにせず、必要条件が揃うまでマージしない |

## 検証結果

- 変更前・変更後の `cargo xtask oversized-files`: 成功。既存14ファイルのwarningと`runtimeApi.ts`縮小noteは同じ。baselineは変更していない。
- `npx pnpm@10.16.1 test src/styles/design-contract.test.ts`（`apps/desktop`）: 1ファイル・1test成功。製品token表を保持したことを確認。
- `git diff --check`: 成功。
- リンク・path・command: 本記録を含むローカルMarkdownリンク40件（anchor含む）、REFACTORINGの具体的なrepository path33件は存在。正本へリンクした見出しも一致。変更した現行規則・devの`cargo xtask` command20種は登録済み。
- 設定: `engineering-change.yml`を既存の`js-yaml`でparseし、body10要素、重複しないID9個、共通必須6欄、Aで省略する2欄、B/Cの記入必須説明と確認checkboxを検査して成功。KiloのJSON、AGENTS読み込み先の存在、CLAUDEのimportを確認。専用の雛形検査は対象ソース・scripts・workflow検索で見つからず、構文・必須欄と代表例で確認した。
- 差分: token marker領域が基準commitと同一、過去記録2本が追記だけ、全変更pathが文書・雛形・Kilo設定に限定されることを確認した。一回の確認に用いた補助scriptは非追跡の作業領域に置き、恒久的な評価基盤・製品testは追加しない。
- 補助環境の確認: Python標準環境にPyYAML、Nodeの直接探索先にyaml packageがなかったため、インストールせず既存eslintが使う`js-yaml`へ切替。これは製品testの失敗ではない。
- ローカル未実行: 全Rust / frontend / browser / Tauri suite。文書・設定・雛形のみのためローカルはtargeted確認を選定。ユーザー依頼に従い、PR headで既存Kukuri Fastを手動実行する。CIのpath filter・job・必須設定は変更しない。
- 実機未実施: 製品UI挙動の変更はない。代表例3bは文書の適用確認であり、実機成功とは報告しない。Claude / Kiloの実アプリ起動も実施せず、設定・参照解決までの確認とする。

## AC / INVARと証拠

| 条件 | 対応する差分・証拠 | 判定 |
| --- | --- | --- |
| AC-1 | 対象一覧23ファイル、READMEの正本表、入口設定 | 確認済み |
| AC-2 | F1〜12とDOC-1〜11の判定、未判定0 | 確認済み |
| AC-3 | 正本への委譲、frontend / Tauriの区別、ミラーの同期test | 確認済み |
| AC-4 | PLANS・REFACTORING・DESIGNの形式/件数制約整理、DOC-4の保持理由 | 確認済み |
| AC-5 | 承認・再現はIssue運用、選定/中断はREFACTORING、見直しはREADME | 確認済み |
| AC-6 | 入口・Issue/PR/UI記録雛形・旧規則への案内、path / command / 構文確認 | 確認済み |
| AC-7 | 上記6組の適用確認（複合例を小例へ分け、全組を網羅） | 確認済み |
| AC-8 | PR head CI、マージcommitとtree、Issue現在判定の照合 | マージ前のため未完了 |
| INVAR-1 | 製品コード・契約・通信経路・設計値・token markerの差分なし | 差分確認、マージ後も照合 |
| INVAR-2 | 境界test / 禁止I/O、Cの独立監査・CI条件、ratchetを維持。test / CI設定を変更しない | 差分確認、マージ後も照合 |
| INVAR-3 | 承認範囲と判断権限の規定、固定AC / INVARを維持 | 差分確認、マージ後も照合 |
| INVAR-4 | 履歴2本は後継案内の追記のみ、既存UI記録本文・token表は不変 | 差分確認、マージ後も照合 |

対象外は製品実装、全ADR・履歴の書き直し、モデル比較・定期監査基盤。固定範囲の規則整理に関する
未判定事項は残さず、未実行のCI・マージ照合だけをT4へ残す。
