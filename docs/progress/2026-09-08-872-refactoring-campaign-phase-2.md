# Issue #872 Phase 0–2: 監査・候補分類・個別Issue化

## 対象と開始trigger

- 親: [#872](https://github.com/KingYoSun/kukuri/issues/872)。既存Scope revision `2026-09-04-issue-872-refactoring-campaign-v1`を維持する。
- 今回の依頼範囲: Phase 0〜2。変更前基準、有限監査、全候補の分類、個別Issue作成。Phase 3のcontract実装・refactor実装・PR、Phase 4の完了監査・Close・#873用完了baselineは後続工程。
- trigger: 2026-05-28の責務別整理後、Column、Dome、CN legal/indexing、backup、GUI/CLI共通基盤とpreview配布が追加された。2026-05-29以降の履歴は554 commits（rename追跡を加えない`git rev-list --count --since=2026-05-29 HEAD`）。
- リスク: 親はC。今回の製品差分は0で、監査記録と追跡Issueを作成する。子refactor/contractはそれぞれC、別種別の文書同期はA。
- 採用根拠は観測した責務・変更履歴・呼出経路・既存test。行数や一般論だけを根拠にした抽出は採用しない。

## 作業と受入条件の対応

| 作業 | 親AC / INVAR / TR | 成果・終了条件 |
| --- | --- | --- |
| T1 変更前基準固定 | AC-1、INVAR-1/2、TR-1 | #871のmergeを確認、main SHA・責務map・validation・既知failを記録 |
| T2 有限監査 | AC-2、INVAR-3、TR-2 | INV-1〜6の登録入口・sink・caller・前回mapとの差と候補20件を分類 |
| T3 一意図のIssue化 | AC-3のIssue分割部分、INVAR-1/2、TR-3/4 | refactor4件、先行contract4件。別種別2件。各子のAC/INVAR/INV/TR、before/after、検証・rollbackを固定 |
| T4 記録・追跡の確認 | AC-2/3の今回範囲、INVAR-3 | Issue作成結果・依存・本文・対象pathを再読。独立DoRレビューの指摘を反映。Phase 3/4の未実施を明示 |

## 変更前baseline

- 監査開始commit: `ac9946d66aaf197204f669a10701fac8a13877e9`。開始時にlocal main / GitHub mainが一致、作業treeはclean。
- #871の規範は [PR #894](https://github.com/KingYoSun/kukuri/pull/894)、`f933a286bfdef4c2325036af93139746a17f6f3e`、2026-09-04 12:02:21 UTCにmainへmerge済み。ancestor確認済み。
- 機械記録: [監査開始baseline](2026-09-08-872-audit-baseline.json)。`record_kind=issue-872-phase-0-2-audit-start`、`completed_campaign_baseline=false`。#873用完了baselineへ昇格させていない。
- 大型ファイル: `cargo xtask oversized-files`はPASS、16件。起票時の14件は当時の値。`runtimeApi.ts`はratchet1042行/実測1035行で縮小noteあり。ratchet自体は更新しない。

### 検証結果と既知失敗

| 検証 | 対象・結果 | 実行/再利用の範囲 |
| --- | --- | --- |
| `cargo xtask oversized-files` | ac9946d6 / Windows、PASS。初回build 3分34秒 | 16件、violation 0。行数のみで採用しない |
| `cargo xtask doctor` | ac9946d6 / Windows、PASS | Rust 1.92、Node/pnpm入口。全外部serviceの健全性証明ではない |
| `cargo xtask desktop-lint` | ac9946d6 / Windows、PASS | eslint 45.5秒、typecheck 9.4秒 |
| `cargo xtask desktop-test` | ac9946d6 / Windows、PASS | 151 files / 1186 tests、Vitest 325.61秒。jsdomのcanvas/media/navigation未実装メッセージあり |
| Metaverse実irohイベントtestのtargeted再現 | ac9946d6 / Windows、FAIL、exit101 | `game.rs:141`、`Dome preset manifest is unavailable`。0 PASS/1 FAIL/192 filtered、0.30秒、compile2分06秒 |
| [Fast 34094310735](https://github.com/KingYoSun/kukuri/actions/runs/34094310735) | 73273ef5、9 jobsすべてsuccess | Rust check/test、Tauri、CN check/integration/e2e、public/index connectivity、desktop lint/Vitest/Storybook/browser/visual、Windows package/smoke |
| [Nightly 34056551815](https://github.com/KingYoSun/kukuri/actions/runs/34056551815) | fe156251、slow testだけfailure | slow featureは192 PASS/1 FAIL。同じ失敗は[前日Nightly](https://github.com/KingYoSun/kukuri/actions/runs/33988557825)でも発生。通常Fast成功に含めない |

targeted再現command:

```powershell
cargo test -p kukuri-app-api --features iroh-integration-tests metaverse_room_events_replicate_between_iroh_peers -- --nocapture
```

FastのSHAと監査SHAの`crates` / `apps` / `xtask` / `harness`のtree objectはそれぞれ一致する。`git diff --name-only 73273ef5..ac9946d6`の差分はrelease scripts/workflowと文書だけである。これを主要pathの参考baselineとして採用し、監査SHAで全suiteを再実行したとは主張しない。
今回は製品変更がないため、`cargo xtask check/test/rust-test/cn-check/cn-test/e2e-smoke/desktop-ui-check`全体、実CN/DB/relay/scenario、OS/WebView操作をローカルで追加実行していない。子の着手時に同じpath matrixで必要なbefore/afterとCIを取得する。Windows localのvisual smokeをLinux snapshot比較の代わりにしない。

## 有限inventoryと責務map

機械記録のmember_pathsは`git ls-tree -r --name-only ac9946d6`へ各selector_regexを適用した固定集合である。独立担当が6群すべて再生成し、差分0を確認した。INV-4はINV-2/3にまたがるDome/Metaverseの横断集合であり、合計件数を重複なしファイル総数と扱わない。

| 親group | 固定path数 | 入口・sink・逆引き・前回mapとの差の記録 | 選定結果 |
| --- | ---: | --- | --- |
| INV-1 domain/data plane | 177 | [core/CN監査](2026-09-08-872-audit-core-cn.md)。署名→wire、store→両backend、docs/blob→Iroh、peer/replica/capability。iroh-node共有ownerは[runtime監査](2026-09-08-872-audit-runtime-metaverse.md)でも確認 | 既設subtrait/peer/node/export境界を維持。cache/row分割延期 |
| INV-2 application/runtime/IPC/CLI | 309 | [runtime監査](2026-09-08-872-audit-runtime-metaverse.md)。Tauri141登録、CLI登録簿、ClientHost、restore/startup/secret、Dome副作用 | RT-1のみ実施。共通host再統合延期、legacy削除却下 |
| INV-3 desktop frontend | 539 | [frontend監査](2026-09-08-872-audit-frontend.md)。route/store/data/action/view/API、全通知callers、SQLite既読sink、event/interval/section切替 | F-1のみ実施。route/API全面分割延期、既済test/mock整理却下 |
| INV-4 Metaverse/Dome | 65 | [runtime監査](2026-09-08-872-audit-runtime-metaverse.md)。scene→session→prepare/commit/abort→host、authority/persistence/physics逆引き | MV-1のみ実施。physics分割延期、client/authority guard統合却下 |
| INV-5 Community Node | 262 | [core/CN監査](2026-09-08-872-audit-core-cn.md)。全14 CN crate、HTTP/CLI/tick/restore→auth/consent/DB/Redis/scan/index/relay | C-CN-1のみ実施。admin guard延期、旧capability説明はdocs |
| INV-6 harness/tests/docs/CI | 402 | [harness監査](2026-09-08-872-audit-harness.md)。19 YAML/10kind、runner/fixture/cleanup/13 test-support公開member、CI/規範 | runner分割延期、旧waiter/slow分離却下、既知failをfixへ |

scopeは登録単位・責務境界の有限監査であり、全source行・全公開APIの全状態の正しさを証明する工程ではない。個別子は変更する入口とsensitive sinkの全callerを再固定して負の経路を検証する。
`infra/terraform`等の配布インフラ、archive、画像/binary、ローカル設定、CodeGraph index、依存cache、未登録の将来surfaceは製品構造変更の対象外。docs/CI/設定は現行の実行・規範・責務mapの参照として確認し、全履歴文書の修正や全配布監査を行わない。前回progressは当時の記録を保持する。

### 大型ファイルsignalの扱い

| 対象 | 監査上の扱い |
| --- | --- |
| `cn-operator/{src/config.rs,src/docs.rs,tests/generate.rs}` | INV-5-Gの設定/生成/contract所有を維持。サイズだけの分割はしない。C-CN-3は規範の説明だけ |
| `docs/THIRD_PARTY_NOTICES.md` | 既存告知内容の保持。ライセンス本文を行数都合で分割・削除しない |
| `useMetaverseRoomSession.ts` | MV-1。非同期attempt ownerの縮小が成果で、1272行は補助signal |
| `harness/scenarios/{community_node.rs,desktop_smoke.rs}` | H-01/H-02として延期 |
| `metaverse-host/src/{lib.rs,tests.rs}` | MV-2延期。authority stateと既存testを維持 |
| `app-api/src/{private_channels.rs,service/private_channels_support.rs}` | INV-2A。write-through callbackとdomain境界が既設、既存契約維持 |
| `desktop-runtime/src/runtime/private_channels_game_api.rs` | RT-1。3段副作用sequenceだけ対象 |
| `app-api/src/dome_hosting.rs` | INV-4Bの署名済みowner authority。runtimeのCN通信とは共通化しない |
| `runtimeApi.ts` | F-4延期。command/request/mockの現行正本を維持 |
| `presentation.test.ts` | F-3の既存保護網維持。テストの長さだけで分割しない |
| `desktop-runtime/src/identity.rs` | RT-3却下。legacy/keyring/file互換の無根拠撤去禁止 |

## 候補分類と作成Issue

20候補を**実施4・延期9・却下5・別種別2**へ分類、未分類0。優先順位P1/P2は作業順の目安で、実装開始は先行contractの成功と対象差分の確認を条件にする。

| 候補 | 分類 | 根拠・目標 | 個別Issue / 依存 |
| --- | --- | --- | --- |
| F-1 通知取得/state反映 | 実施 P1 | 2 owner、background inbox修正が4製品fileへ波及。modeを保ちowner2→1 | [#919](https://github.com/KingYoSun/kukuri/issues/919) → [#920](https://github.com/KingYoSun/kukuri/issues/920) |
| RT-1 CN Dome transfer | 実施 P1 | assignment→owner activation→CN activateが2入口で重複。各3 helper callsite2→1 | [#921](https://github.com/KingYoSun/kukuri/issues/921) → [#922](https://github.com/KingYoSun/kukuri/issues/922) |
| MV-1 Dome transition attempt | 実施 P1 | ack喪失fixの波及。元hookのprotocol state直接更新9→0、専用owner1 | [#923](https://github.com/KingYoSun/kukuri/issues/923) → [#924](https://github.com/KingYoSun/kukuri/issues/924) |
| C-CN-1 indexer source解決 | 実施 P2 | source検証とscan/metrics/2storeを同ownerが所有。source依存6→2 | [#925](https://github.com/KingYoSun/kukuri/issues/925) → [#926](https://github.com/KingYoSun/kukuri/issues/926) |
| F-2 route/Column再抽出 | 延期 | 既存分離からさらに縮める正本/依存を未立証 | 新たな具体的変更圧力で再評価 |
| F-4 runtimeApi全面分割 | 延期 | 25変更commitだけで取り違え/契約漏れとの因果を示せない | domain単位の必要性を確認後 |
| RT-2 startup全面集約 | 延期 | ClientHost/同意/復元の抽出は実施済み | 新しい共通判断の重複発生時 |
| MV-2 physics/budget分割 | 延期 | 同じauthority stateを分ける成果が未立証 | state依存による変更阻害時 |
| C-DATA-1 blob cache policy | 延期 | policy同居は観測、変更摩擦の独立証拠不足 | 次回cache policy変更時 |
| C-DATA-3 row mapping | 延期 | parity/subtrait既設、NULL不具合が同居に起因する証拠なし | domain mappingの具体的摩擦時 |
| C-CN-2 admin guard | 延期 | HTTP否定経路保護不足、現行error/検証順に意味の差 | admin変更時のcharacterizationから |
| H-01 CN connectivity runner | 延期 | 同意変更圧力はあるが2 identity modeとcleanupを保つ成果未確定 | 次のCN scenario変更時 |
| H-02 desktop smoke runner | 延期 | Dome step増加、現行DSL集約による具体的漏れ未観測 | 次のDome scenario変更時 |
| F-3 旧test/mock/store/view分割 | 却下 | 既に実施済み。旧follow-upを再起票しない | 現行境界維持 |
| RT-3 legacy identity撤去 | 却下 | sunset条件と鍵保持契約に反する | 任意cleanupにしない |
| MV-3 client/host guard統合 | 却下 | preflightとauthority再検証は別責務 | 両側guard維持 |
| C-DATA-2 core/peer再整理 | 却下 | 既済の境界整理、secret/fetch policyの差を保持 | 再抽象化しない |
| H-03 waiter/test/slow再分割 | 却下 | domain分割/poll共通化/slow laneは実施済み | 現行検証網維持 |
| H-04 Metaverse実iroh失敗 | 別種別 fix P1 | 2 Nightlyと現行Windowsで同じmanifest未取得を再現 | [#927](https://github.com/KingYoSun/kukuri/issues/927)。原因未確定、refactorへ混在禁止 |
| C-CN-3 capability文書 | 別種別 docs | 既にAvailableへ昇格したcapabilityを規範がPlanned3件と記載 | [#928](https://github.com/KingYoSun/kukuri/issues/928)。製品の提供状態は変更しない |

4組は独立して進められる。共通して同じ元SHAからの差分を確認し、各contractを先にmerge・PASSにしてから対応refactorへ進む。refactor4件はそれぞれ単独PR・単独rollback。今回発見したfix/docsは別種別として追跡し、選定refactorへ混ぜない。既存UI不具合 #913〜#918も既存Issueを参照し、重複起票しない。

## 独立レビューと今回の終了判定

これは子の実装完了監査ではなく、Phase 0〜2成果物とIssueの開始条件の独立レビューである。

- baseline/harness: 作成担当とは別担当が固定commitから再列挙。JSON6群の差分0。artifactの不正確なscreenshot記述を修正し、test-support全memberとfixture cleanup/identity sinkを補った。
- frontend: 別担当が現行sourceから再構築し、activePrimarySection変更時のeffect再実行、active inbox時のbadge list抑止をINV/TRへ補った。
- CN indexer/fix: 別担当がsource/既存contractから再構築。禁止index writeをupsert追加と明確化し、許可deindex removeと区別。testの既存assertionはseq選択/署名/variantに訂正した。
- runtime/Metaverse: 別担当がsourceと4草案を照合。owner-host新規layout更新はrevision/operation更新を許可するようRT TR-4を分離。MV TR-1の禁止副作用を取消対象attemptに限定し、別join/leave本来の副作用を維持した。C/R双方へ同じ修正を反映。
- root担当はRT/MVのcaller、境界、先行contractと後続のAC/INVAR/table対応、全Issue本文とsourceリンクを確認する。

Phase 0/1とPhase 2のIssue化を完了した。親AC-1/2はこの開始基準・有限監査へ対応する。AC-3は**子Issue分割と先行contractの計画まで**で、contract実装・PRは未実施のため全体として未完。AC-4/5、親Close、#873完了baselineは未完のまま維持する。
監査記録は作業treeへ保存。今回コミット・push・PR・mergeの依頼はないため実施しておらず、GitHubには子Issueの自足した本文と親の追跡表を保存する。子の本文が実装時の固定追跡面であり、担当報告の「草案」は起票時の証拠として保持する。

最終確認: GitHub上の10 Issueのtitle/body/Open状態、親のCurrent status単一性とAC-1/2完了・AC-3未完、8 sub-issueと4依存組を再読して一致確認した。Markdown5件のwhitespace・相対リンク18件、JSON6群の固定commitでの再列挙、候補20件の分類、製品差分0を確認済み。監査記録追加後の`cargo xtask oversized-files`と`git diff --check`もPASS。
