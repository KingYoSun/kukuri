# #1152 添付の状態確認による成人向け media の取得・永続化を止める

## 範囲と判定

- Issue: [#1152](https://github.com/kukuri-app/kukuri/issues/1152)（親 #1051 の INVAR-3、#1068 の AC-6 で発見）。リスク区分 C。Scope revision 2026-09-18、計画 r2（r1 に INV-8 / T7 を追加）。
- 基準 commit: `977ef076`（main）。
- 独立監査: PR head に対して別コンテキストで実施する（PR の comment に記録）。

## 原因

- 投稿の projection 反映（`hydration_support.rs`）と view 生成（`attachment_support.rs`）が、添付ごとに `best_effort_blob_cache_status` / `best_effort_blob_view_status` → `BlobService::blob_status` を呼んでいた。
- `IrohBlobService::blob_status` はローカルに無い blob を `fetch_blob` で remote から取得・永続化して確かめる。成人向けの取得ゲートは `blob_media_payload` にだけあり、この経路を通らない。
- CN advisory は受信・hydration の時点では未判明（一括照会・index 応答で判明）のため、状態確認の中で成人向け判定をしても防げない。
- 実装中に、desktop の本番で使う `ReloadableBlobService`（`crates/desktop-runtime/src/stack.rs`）が `fetch_blob_ephemeral` を実体へ転送していないことも分かった。trait の既定実装が永続化する `fetch_blob` へ委譲するため、表示 ON の成人向け取得（ephemeral のはず）も端末へ保存されていた（Existing-gap、INVAR-1 に反する）。

## 修正

- `BlobService::local_blob_status` を追加した。ローカルの有無だけを返し、remote から取得せず bytes も読まない。既定実装を置かない必須メソッドにした（`blob_status` への委譲で黙って remote 取得へ戻る実装を作らないため）。`IrohBlobService` は pin・metaverse pin tag・`blobs().has()`、`MemoryBlobService` はローカル参照で判定する。
- `best_effort_blob_cache_status` / `best_effort_blob_view_status`（投稿・DM の添付と投稿本文の状態）を `local_blob_status` に切り替えた。remote 取得は、ゲートを持つ表示要求（`blob_media_payload`）と本文取得（`fetch_projection_blob_text`）に限る。
- `ReloadableBlobService` に `fetch_blob_ephemeral` と `local_blob_status` の転送を追加した。
- Dome の preset asset 確認と game / live の manifest は成人向けゲートの対象外で、remote で利用可能なことを確かめる意味を持つため `blob_status` のまま。
- 添付の状態値（`AttachmentView.status`、`blob_objects`）は画面の取得判断に使われていない（frontend は本文の `content_status` だけを参照し、`blob_objects` は書き込みのみ）ため、表示への影響はない。

## 修正前の再現

- `projecting_remote_adult_labeled_post_does_not_fetch_attachment_while_display_disabled`: 表示 OFF で remote の self-label 付き投稿を timeline へ反映すると、添付の状態が `Available`（remote 取得・保存済み）になって失敗（期待 `Missing`）。
- `projecting_remote_posts_fetches_attachments_only_on_ungated_display_request`: ラベルの無い remote 投稿 2 件を反映しただけで、添付 2 件が remote から取得されて失敗（期待 0 回）。
- `reloadable_blob_service_keeps_ephemeral_fetch_and_local_status_non_persistent`: 実 Iroh 2 ノードで、`ReloadableBlobService` 越しの ephemeral 取得の後に受信側の状態が `Available`（永続化）になって失敗（期待 `Missing`）。
- 修正後は 3 件とも成功。`local_blob_status_does_not_fetch_or_persist_remote_blob`（実 Iroh 2 ノード、`IrohBlobService` 単体）も成功。

## 対応する条件

| 条件 | 実装 | test |
| --- | --- | --- |
| AC-1 | `object_persistence_support.rs` の状態確認 2 関数を `local_blob_status` へ | `projecting_remote_*` 2 件 |
| AC-2 | 上記の失敗 test を先に置いた | 同上、`reloadable_blob_service_keeps_*`、`local_blob_status_does_not_fetch_*` |
| AC-3 | 通常 media は表示要求で取得・保存される | `projecting_remote_posts_fetches_attachments_only_on_ungated_display_request`、既存 `media_adult_gating.rs` / timeline / DM test |
| AC-4 | `blob_status` の他の呼び出し元（Dome preset asset、game / live manifest）は変更しない理由を上記「修正」に記録 | 既存 game / dome test |
| INVAR-1 | OFF の状態確認で取得しない、ON の ephemeral 取得を desktop でも永続化しない | 上記 4 件、既存の成人向けゲート test |

## 検証

- `cargo xtask check`（fmt / clippy / 型生成を含む）: 成功。
- `cargo xtask rust-test`: nextest 1032 passed / 4 skipped、doc test 成功（blob-service / app-api / desktop-runtime を含む）。
- `cargo xtask cn-check`: 成功。`cargo xtask cn-test`: 711 passed（cn-indexer の test double 更新を含む）。
- `cargo xtask scenario pairwise_dm_offline_text_image_video_delivery_and_local_delete`: pass（DM の画像・動画添付の状態表示）。
- frontend: view の型（`AttachmentView` / `BlobViewStatus`）と IPC は変えていないため、`REFACTORING.md` の path 別規則により desktop の UI test は対象外。

## 範囲外として記録したもの

- `ReloadableBlobService` は `unpin_blob` も転送していない（既定実装は no-op）。Metaverse の blob cache 解放に関わり、成人向けゲートとは無関係のため別タスクとして扱う。
- 既に端末へ保存された blob の削除は Non-goal。
