# #1052 「見つける」の解決済み投稿メディア描画の作業記録

## 現在の状態

- PR: [#1070](https://github.com/KingYoSun/kukuri/pull/1070)。
- リスク区分 B、Scope revision: 2026-09-15、基準 commit: `124d8e7c`。独立監査は不要（親 Issue / shared guard なし）。
- ユーザーは実装・Issue 更新・commit・PR・CI 成功後の merge を承認済み（2026-09-16）。
- AC / INVAR、固定 inventory と状態遷移の正本は [Issue #1052](https://github.com/KingYoSun/kukuri/issues/1052)。UI 採用条件と before / after は [review record](../ui-reviews/2026-09-16-1052-discover-post-media.md)。

## 修正前の再現

実装前に失敗する test を 3 系統置き、main（`124d8e7c`）で 11 件が失敗することを確認した。

| 系統 | 失敗内容（修正前） |
| --- | --- |
| `communityIndexPostCardView.test.ts` | 解決済み + 添付ありでも `attachments` が空、`media.kind` が null のままで 5 件失敗 |
| `CommunityIndexWorkspace.test.tsx` | `media-preview-*` / `media-adult-gated-*` が描画されず、解決済み投稿の公開も無く 3 件失敗 |
| `DesktopShellPage.communityIndexMedia.test.tsx` | Explore 経路で添付の取得が起きず、画像もプレースホルダーも出ず 3 件失敗 |

ブラウザでも同じ再現を取った。`community-index-media.spec.ts` を修正前の view builder で実行すると 3 件とも「要素が見つからない」で失敗し、変更後は 3 件成功する。視覚の変更前後は review record の表に記録した。

## 対応

| 条件 | 実装・検証 |
| --- | --- |
| AC-1、INVAR-3 | タイムラインの inline なメディア組立を `shell/viewModels/postMediaView.ts` の `buildPostMediaView` へ抽出し、`communityIndexPostCardView` と `useTimelineViewModels` が共有する。再生診断の `videoProps` はイベントを購読するタイムライン側に残した。解決済み分岐で `attachments` を canonical な `PostView` から渡す |
| AC-2、INVAR-2 | `adultContentGated` を builder へ渡すため const 化。gated では preview src と gallery を空にし `media.state = 'gated'` にする。Rust 側 `blob_media_payload` と ephemeral fetch は変更していない |
| AC-3、INVAR-1 | 未解決・解決失敗の分岐は `attachments: []` のままで、同じ builder でもメディアを描画しない。`IndexEntryView.text` を本文・ラベル・添付の信頼元にしない既存契約は不変 |
| AC-1（取得経路） | 表示中の解決済み投稿を `communityIndexResolvedPosts`（media slice の一時状態）へ公開し、`usePreviewableMediaAttachments` と `gatedAdultMediaHashes` の source に加えた。結果の失効と Column の終了で空配列を通知する |
| INVAR-3 | 抽出 helper の単体 test（7 件）と既存のタイムライン・成人向けゲート test で出力不変を確認した |

`PostMedia` / `PostCard` / Rust / IPC 契約は変更していない。新しい API、永続化、外部送信は追加していない。

## 検証結果

- `apps/desktop` の Vitest: 206 files / 1746 tests 成功（新規 16 件を含む）。
- `apps/desktop` の lint / typecheck: 成功。
- Playwright chromium: `community-index-media.spec.ts` 3 件成功。既存 browser / visual suite の結果は下記の追記節を参照する。
- `cargo xtask check` / `cargo xtask test` / `cargo xtask desktop-ui-check` / `cargo xtask oversized-files` の結果は追記節と PR の CI を参照する。

必須検証を省略して成功とみなさない。未実行・失敗・中断は成功と区別して記録する。

## 引き継ぎ

- #1055（#1051 C3）は本 PR の表示経路へ `content_advisories` の合成を重ねる。追加された prefetch source（`communityIndexResolvedPosts`）は advisory でゲートされた entry も除外対象に含める必要がある。
- Issue 本文の「既存 test」欄にある `CommunityIndexWorkspace.test.tsx`（adult OFF / ON）は着手時点で存在せず、本 PR で新規追加した。Scope は変えていない。
