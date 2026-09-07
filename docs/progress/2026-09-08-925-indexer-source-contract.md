# Issue #925: source解決前後のI/Oとindex結果を固定

- Scope revision `2026-09-08-872-C-CN-v1`、区分C。before `2229fefaa479064bf8080bc55e2984acf025fc66`。
- 新規integration testと同配下support、本記録のみ。製品IngestPipeline/既存test/公開API/wire/schemaを変更しない。後続#926の前に現在の挙動を固定する。
- 既存のMemoryDocsSync、MemoryIndexEntryStore、MemoryIndexProjection、SafetyScanService、MockSafetyProviderを組み合わせ、境界traitを薄い計測wrapperで包む。既存sourceの複製や実装helperのmockはしない。

| 作業 | AC / INVAR | 対象と受入確認 | 依存 |
| --- | --- | --- | --- |
| T1 変更前baseline / 全caller | AC-5、INVAR-1/2 | 既存blob/ingestion/transmission3 suite、publicPipelineとworker/participant、source/provider/2storeの逆引き | なし |
| T2 body / manifest保護 | AC-1/2/3、INVAR-3 | 新規source_resolution_contractsと計測support。下表の拒否条件/禁止I/Oと許可deindex | T1 |
| T3 mixed / reingest保護 | AC-4、INVAR-3 | 複数状態混在、同store/Docsでpipeline再生成、valid memberの継続と削除集合 | T2 |
| T4 検証 / 独立監査 / CI | AC-5、全INVAR | targeted、cn-check/cn-test、固定head監査とmerge tree。結果はPR/Issueへ | T3 |

## INV / TR / test対応

| 対応 | 新規test・観測 |
| --- | --- |
| INV-R2/3/6、TR-R2 | `unverified_blob_metadata_has_zero_fetch_scan_and_index_upserts`: envelope missing/malformed/signature/ID/author/payload/declared oversizedの7条件。ephemeral/durable fetch・provider・2store upsert0、許可された各storeのremove1を別計測 |
| INV-R3/6、TR-R3 | `rejected_blob_bytes_use_only_ephemeral_fetch_and_never_reach_a_provider`: missing/fetch error/size/hash/UTF8/character limitの6条件。ephemeral1だけ、durable/put/pin0、provider0、2store remove。正常index済み→取得不能もdeindexし古いentryを残さない |
| INV-R1/3/6、TR-R1 | `unicode_body_at_the_character_limit_remains_scoped_and_ephemeral`: 10,000文字/30,000bytesの本文をpublic/許可済みprivate scopeでindex。本文・scope・provider入力・ephemeralだけの取得、別scopeへ出ないことを確認 |
| INV-R4/5/6、TR-R5 | `invalid_media_manifest_never_sends_media_to_the_provider_or_upserts_indexes`: missing/invalid signature/wrong kind/wrong author/malformedの5条件。post text scan1は許可し、media scanとupsertを0、deindexを別計測 |
| INV-R4/5、TR-R5 | `duplicate_media_hashes_preserve_first_mime_including_a_thumbnail_without_mime`: attachment→manifest重複、競合mime、thumbnail→後続item重複。同hash1scan、trimmed先勝ちmime、先勝ちNoneと対象順を保持 |
| INV-R1/2/3/6、TR-R4/7 | `mixed_scope_reingest_never_fetches_withdrawn_prevented_deleted_or_invalid_members`: 最初6件index後、valid/署名withdrawal/deleted/tombstoned/legal prevented/invalid signature/nonpostを混在。pipeline再生成の2passとも7 scanned/1 indexed/1 skipped/4 deindexed、validのhash/providerだけ、2store削除5、他scope削除0 |
| INV-R5/6、TR-R6 | 既存ingestion/query/worker/provider/CN e2eがnonallow・provider/artifact・2store failureを保護。新規source testへ同内容のassertを複製しない |

`fetch_blob_ephemeral`は明示overrideし、trait defaultのdurable fetch委譲で誤検知しないよう分離計測する。MemoryIndexEntryStoreは同じmoderation artifact storeを参照し、既存allow制約を維持。upsertとremove、scope全体のremoveを混同しない。計測対象は外部trait I/Oと保存結果であり、private helper名を固定しない。

## 全callerと凍結境界

CodeGraphでrootのingestion/source/testを確認し、indexの無いworktreeでは同一sourceを直接読み補完した。
`rg -n 'ingest_scope|ingest_all_supported|resolve_body_text|media_scan_targets|resolve_media_manifest|fetch_blob_ephemeral|upsert_entry|remove_entry|remove_object' crates/cn-indexer crates/cn-core crates/cn-e2e`でworker/participant/direct tests、source→scan→store、別query/legal deindex経路を分類。
本Issueの製品INV-R1〜6に増減なし、未分類0。shared docs/blobs、private capability、wire/署名、same-pass envelope map、query policy、Postgres→projectionの順、error/summary/metricsの変更は0。

## 実行記録

- before: `cargo test -p kukuri-cn-indexer --test blob_text_ingestion_contracts --test ingestion_contracts --test transmission_prevention_contracts` →3+17+2=22 tests PASS。
- 新規: `cargo test -p kukuri-cn-indexer --test source_resolution_contracts` →6 tests PASS（0.02秒）。
- 最終は既存3suiteとの一括targetedと、必須`cargo xtask cn-check` / `cargo xtask cn-test`をCIで確認する。in-memory targetedの成功を実Postgres/Redis/ArcadeDB成功と混同しない。
- worktree専用`CARGO_TARGET_DIR=.../target/issue-925`で実行し、別worktreeの埋込みpathを持つxtaskを再利用しない。
- 最終結果・独立監査head・CI・merge tree一致はPR/Issueに保存する。

Rollbackはtest-only PRを単独revert。#926実装後は先にrefactorを戻し、保護網だけを外さない。既存仕様との不一致が見つかったらfixへ分類し、assertionを弱めて抽出を開始しない。
