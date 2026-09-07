# Issue #926: CN indexerのsource resolverを読み取り境界へ抽出

- Scope revision `2026-09-08-872-R-CN-v1`、区分C。before `7c9cba850d572c8d358d5f529798a89c3c84eb06`。
- 先行#925はPR#935、最終head `a2dbe8e2`。独立監査・delta監査・最終17 checks・merge対象subtree一致PASSを経て開始。
- 一意図はsource解決の依存境界縮小。public IngestPipeline/IngestSummary、wire/schema、provider・readiness・query gateを変更しない。

| 作業 | AC / INVAR / INV・TR | 証拠 | 依存 |
| --- | --- | --- | --- |
| T1 before contractとscope確認 | AC3/5、全INVAR、INV-R1〜6/TR-R1〜7 | #925 finalとmergeのcn-indexer subtree一致、下記39 tests | #925 |
| T2 source moduleへ抽出 | AC1/2/4、全INVAR | DocsSync/BlobServiceだけを借用するprivate SourceResolver、元2callsiteの結線 | T1 |
| T3 同contract/Clippy/差分確認 | AC3/4/5、全INVAR | 同39 tests、型/Clippy、全caller/依存逆引き | T2 |
| T4 独立監査/CI/merge | AC5、全INVAR | 固定head、cn-check/cn-test、merge対象一致。最終結果はPR/Issueへ | T3 |

## before / afterの責務と依存

| owner | before | after |
| --- | --- | --- |
| 本文/manifest/target解決 | pipelineの6serviceに到達可能 | SourceResolverのDocsSyncとoptional BlobServiceの2借用のみ |
| scope traversal、withdrawal/status/prevention | pipeline | pipelineのまま |
| safety scan、metrics、publish/deindex | pipeline | pipelineのまま。sourceへ逆依存0 |
| body/media解決callsite | pipeline内各1 | 同じ位置のSourceResolver各1、wrapperなし |
| public Pipeline/Summary/API | 現行公開契約 | 差分0 |

新規`ingest/source.rs`にbody/manifest解決3method、dedupe/mime2helper、上限定数、private PostObjectView/MediaScanTargetを移動した。`mod source`はprivateで、親だけが使う型/field/2解決methodは`pub(super)`、manifest解決はprivate。trait/crate/dependencyの追加なし。sourceは他のownerやpipelineを受け取らない。

pipelineは既存のwithdrawal→status→transmission preventionを完了してからresolverを構成する。body解決後にpost scanし、allow後に同じresolverでmediaを解決する。構成自体にはI/O・clone・mutationがない。bodyとmediaを先読み・一括解決せず、失敗時のdeindex/summary/metrics/2store順を維持する。

`BlobText`は同一scope passのenvelope mapから検証し、元と同じephemeral fetchだけを使用する。manifestは同replicaのExact query/LocalThenRemoteのまま。InlineTextの検証追加、canonical key/serde/signature/size/UTF8/error文言の変更なし。first-wins mime/thumbnail Noneとpost scan後のmedia判断をそのまま移す。`text_with_tags`はpublish側に残す。

## 全callerと対応

- worker startup/event/retry→participant→public ingest_scope→ingest_object_recordの入口は変更0。
- resolve_body_text / media_scan_targetsのproduction callerはingest_object_record各1、resolve_media_manifestはsource内media_scan_targetsだけ。
- source field/import/引数へSafetyScanService、IndexerRuntimeState、IndexEntryStore、IndexProjectionが0件。provider/metrics/upsert/removeはpipeline/既存participant/queryに保持。
- `rg -n 'resolve_body_text|resolve_media_manifest|media_scan_targets|scan_and_record_with_metrics|deindex_object|fetch_blob_ephemeral|upsert_entry|remove_entry|remove_object' crates/cn-indexer crates/cn-core crates/cn-e2e -g '*.rs'` とdiffで順・逆方向を再列挙する。
- 固定inventory6/transition7は増減0、未分類0。No permanent blob storage、private scope、two-store fail-closed、HTTP/authority/three-pathに変更なし。

## 検証

beforeの先行最終headとmergeのsubtree一致を確認。以下の同commandを変更前後に実行し、各6suiteの3/17/6/6/2/5、合計39 testsがPASSした（failed/ignored 0）。既存test/fixture/import/assertionの変更0。

```powershell
cargo test -p kukuri-cn-indexer --test source_resolution_contracts --test ingestion_contracts --test blob_text_ingestion_contracts --test transmission_prevention_contracts --test worker_contracts --test query_contracts
```

最初の抽出compileではpublish側に残る`bail!`のimport漏れを検出し、元のimportを保持して解消した。期待値やwarning規則を弱めていない。worktreeは先行と同じ絶対path/専用CARGO_TARGET_DIRを使い、別worktreeのxtask binaryを共有しない。

必須`cargo xtask cn-check` / `cargo xtask cn-test`はCIの対象SHA/jobへ対応する。in-memory targetedの成功を実DB/stack成功と扱わない。独立監査とmerge対象一致まで完了判定を保留する。

Rollbackは抽出PRだけrevertし#925 contractを保持。sourceとpublish policyを同時変更する必要が生じた場合は別fix/featureへ分離する。
