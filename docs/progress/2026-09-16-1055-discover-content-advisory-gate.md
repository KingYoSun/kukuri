# #1055 「見つける」の content advisory ゲートの作業記録

## 現在の状態

- PR: [#1071](https://github.com/KingYoSun/kukuri/pull/1071)。
- リスク区分 B（親 #1051 の監査対象）、Scope revision: 2026-09-15、基準 commit: `2786a2d3`。独立監査は親 #1051 の監査に含める。
- ユーザーは実装・Issue 更新・commit・PR・CI 成功後の merge を承認済み（2026-09-16）。実装計画も同日に推奨案で承認された。
- AC / INVAR、固定 inventory と状態遷移の正本は [Issue #1055](https://github.com/KingYoSun/kukuri/issues/1055)。UI 採用条件と before / after は [review record](../ui-reviews/2026-09-16-1055-discover-content-advisory.md)。
- 本番反映と統合手動確認は C5（[#1068](https://github.com/KingYoSun/kukuri/issues/1068)）へまとめる。本 Issue の Close 条件に本番確認は含めない。

## 前提（C2 / #1052 から引き継いだもの）

- C2（#1054、main `7a5a49f5`）が `IndexEntryView.content_advisories` を wire と TS 型へ入れた。本 Issue はその欄を client が読む側を実装する。
- #1052（main `2786a2d3`）が「見つける」の添付表示を `buildPostMediaView` に統一し、表示中の解決済み投稿をプリフェッチ対象へ公開した。本 Issue はその表示経路へ advisory によるゲートを重ねる。#1052 の引き継ぎ「新しい prefetch source は advisory でゲートされた entry も除外対象に含める必要がある」を本 Issue で消化した（下記 AC-3）。

## 修正前の再現

実装前に、advisory 付き応答を返す fixture で失敗する test を 3 系統置き、main（`2786a2d3`）で失敗することを確認した。

| 系統 | 失敗内容（修正前） |
| --- | --- |
| `communityIndexPostCardView.test.ts` | advisory 付き entry でも `adultContentGated` が false のままで、`contentAdvisory` 欄が存在しない |
| `CommunityIndexWorkspace.test.tsx` | 代替表示も説明ブロックも出ず、`post-advisory-gated-*` が見つからない |
| `crates/app-api` / `crates/desktop-runtime` | advisory 付き hash を取得ゲートへ登録する経路が無く、`register_advisory_media_hashes` が存在しない |

ブラウザでも同じ再現を取った。`community-index-advisory.spec.ts` を `2786a2d3` の worktree で実行すると、推定が無視されて画像がそのまま描画される。その状態を変更前の画像として review record の表に記録した。

## 対応

| 条件 | 実装・検証 |
| --- | --- |
| AC-3、INVAR-3 | `AppService` にプロセス内の advisory hash 集合を持たせ、`blob_media_payload` の既存 guard へ `is_adult_media_hash \|\| is_advisory_media_hash` として合成した。判定点は 1 箇所のままで、ON 中は従来どおり ephemeral fetch、OFF へ戻すと再び止まる |
| AC-4、TR-5 | `query_community_node_index` の応答後処理 `apply_content_advisories` で issuer を照合する。advisory を含む応答のときだけ index を返した node の manifest を 1 回引き、`issuer_node_id` が manifest `node_id` と一致しない advisory を落とす。manifest を取得できない場合も採用しない（fail-closed） |
| AC-1、INVAR-1、INVAR-2 | `communityIndexPostCardView` の `adultContentGated` を「解決済みの自己申告ラベル または entry の gating 対象 advisory」へ広げた。advisory は `content_labels` へ書き戻さず、表示用の `contentAdvisory` 欄だけで運ぶ。`IndexEntryView.text` を canonical にしない既存契約と自己申告のゲートは不変 |
| AC-1（未解決） | canonical 解決前は隠すべき本文が無いため、`gatedBodyText` で従来の待機文言を保ったまま代替表示にする。他の表示経路の出力は変えていない |
| AC-2 | `PostCard` の代替表示に説明ブロックを足し、発行元（manifest 名 + 短縮 node_id）・分類・確信度・根拠を出す。異議申し立ては既存 `ReportRoutingDialog` を appeal モードで開き、`planAppealReportRouting` が発行元ノードだけを送信先候補にする。送信 payload に `appeal.risk_signal_id` が入る |
| AC-3、TR-3 / TR-4 | プリフェッチはゲート中の blob hash を shell state（`advisoryGatedMediaHashes`、一時状態）として公開し、単一入口の `usePreviewableMediaAttachments` で除外する。表示設定 ON では除外集合を空にして通常経路へ戻す |

新しい永続化・外部送信 API は追加していない。manifest 取得は既存の公開 endpoint への GET で、advisory を含む応答 1 回につき 1 回だけ行う。

## 実装上の判断（計画から変えた点）

- **プリフェッチ除外は投稿単位でなく hash 単位にした**。計画（作業仮定 2）は「Workspace が公開集合から gated 投稿を外す」だったが、同じ添付が別の表示経路（プロフィールタイムライン等）にも現れうるため、投稿単位では漏れる。ゲート中の hash を除外集合として持ち、プリフェッチの単一入口で止める形に変えた。Workspace 側の公開除外も併せて残している。
- **合成の無効化は runtime の定数で行う**。ADR 0046 §6.4 により、利用規約改訂と再同意（C4 = #1056）が入るまで合成しない。既定 OFF の `CONTENT_ADVISORY_SYNTHESIS_DEFAULT` を置き、OFF の間は応答から advisory を落とし manifest も引かず、取得ゲートにも登録しない。実行時の setter は有効時の挙動を検証する test だけが使い、本番の有効化は C4 がこの定数を切り替えて行う。
- **advisory hash 集合は in-memory とした**（ADR 0028 §8.10 の transient 分類）。決定は `docs/legal/adult-content-display-data-classification.md` へ記録した。

## 既知の限界（C4 まで）

advisory は index 応答でしか判明しないため、同じ添付が「見つける」以外の経路に先に現れた場合、client はその時点で推定を知らず取得を要求しうる。その要求に対しても Rust 側ゲートが `None` を返すため bytes 取得は 0 だが、代替表示と説明は出ない。タイムライン経路の合成は C4（#1056）が一括照会 API で担う。

## oversized-files 対応（分割）

CI の `linux-rust-static`（oversized-files、1000 行上限）に 4 件抵触したため、`9ce97243` で責務の境界に沿って分割した。挙動は変えていない。

| ファイル | 抵触時 | 分割後 | 分割先 |
| --- | --- | --- | --- |
| `PostCard.tsx` | 1037 | 991 | `PostAdvisoryNotice.tsx`（`PostGatedContent` / `PostAdvisoryNotice`） |
| `CommunityIndexWorkspace.tsx` | 1012 | 951 | `useCommunityIndexAdvisories.ts`（発行元名の解決とゲート中 hash の公開） |
| `CommunityIndexWorkspace.test.tsx` | 1199 | 747 | `CommunityIndexWorkspace.testSupport.tsx`（共有 fixture）と `CommunityIndexWorkspace.advisory.test.tsx` |
| `tests/community_node/index_query.rs` | 1059 | 950 | `index_query/content_advisory.rs`（advisory の 4 test） |

lock 分類 contract は新しい path へ付け替えた（`index_query.rs` は 14 のまま、`index_query/content_advisory.rs` に 4 を宣言、合計 136 → 140）。

## 検証結果

- `apps/desktop` の Vitest: 207 files / 1763 tests。対象周辺（`src/components/core` / `src/shell/data` / Explore メディア）は 31 files / 327 tests 成功。全体実行では `DesktopShellPage.developerMode.test.tsx` の 1 件が並行負荷で timeout する既知 flake が出たが、単独実行では 12 件成功する。本変更が触っていないファイルであり、CI の結果を正とする。
- `cargo xtask check`: 成功（fmt / clippy / tauri check / desktop lint / typecheck）。
- `cargo xtask test`: 分割後の通し実行で成功（950 tests passed / 4 skipped、harness 23 tests passed）。`advisory_labeled_media_respects_adult_display_gate` ほか新規 6 件と `community_node::index_query::content_advisory` の 4 件を含む。
- `cargo xtask oversized-files`: 成功（新規違反 0）。
- Playwright chromium: `community-index-advisory.spec.ts` 3 件成功。変更前は同じ spec が `2786a2d3` の worktree で推定を無視して描画することを確認した。

必須検証を省略して成功とみなさない。未実行・失敗・中断は成功と区別して記録する。

## 引き継ぎ

- C4（#1056）は `CONTENT_ADVISORY_SYNTHESIS_DEFAULT` を利用規約改訂・再同意と同じ変更で `true` にする。それまで advisory の合成は有効にならない。
- C4 のタイムライン合成は、本 Issue が置いた `ContentAdvisoryView` と `PostCard` の説明ブロック、`advisoryGatedMediaHashes` の除外集合をそのまま使える。
- C5（#1068）が本番反映と統合手動確認を担う。
