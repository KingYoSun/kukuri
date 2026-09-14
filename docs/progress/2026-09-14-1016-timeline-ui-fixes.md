# #1016 UI修正の作業記録

## 現在の状態

- PR: [#1017](https://github.com/KingYoSun/kukuri/pull/1017)、最終検証中。
- リスク区分B、Scope revision: 2026-09-14-v2。基準commit: `94d49aabe8f6c77a4468287975705a6765bcaf37`。
- ユーザーは実装・Issue・commit・PR・CI成功後のmergeを承認。途中の「見つける」の3ボタン改善をAC-5として追加承認。
- AC/INVAR、固定inventoryと状態遷移の正本は[Issue #1016](https://github.com/KingYoSun/kukuri/issues/1016)。UI採用条件と実機画像は[review record](../ui-reviews/2026-09-14-1016-timeline-layout.md)。

## 対応と修正前の再現

| 条件 | 実装・検証 | 修正前→修正後 |
| --- | --- | --- |
| AC-1、INVAR-1/2 | PostCardの親contextをarticle外へ移動。3段返信の直前親・author/reply callback、欠落/制限/Thread抑制、添付のみをcomponent test | 既存articleが親本文を内包して失敗→親が先行する兄弟になり成功 |
| AC-2、INVAR-3 | Canvas下端paddingとmobile操作群のbottom。`timeline-ui-fixes.spec.ts`、`shell-bottom-bar.spec.ts`、resize tests | gap不足で失敗→余白成立。途中のmobile下辺13pxずれは同scopeのRegressionとして修正し既存test成功 |
| AC-3、INVAR-3/4 | センターopenでclusterを非表示、close後のeffectでfocus復元。browser/nativeの実pointer、Tab、Escape、再open | 背景account triggerがvisibleで失敗→hidden・操作抑止・復元test成功 |
| AC-4、INVAR-4 | 専用formatPostDateTimeとtime要素。既存時刻formatterの既定値は不変 | 日付付きtimeなしで失敗→年月日・秒、年跨ぎのlocale別test成功 |
| AC-5、INVAR-3/4 | Explore tabsの既存定義（shell-phase1-part3.css）を変更。`explore-compact-tabs.spec.ts` と既存index-layout tests | 3言語×2幅の6件が高さ/改行で失敗→6件と既存19件が成功 |

CodeGraphからPostCardのcallerを確認: TimelineFeed、CommunityIndexWorkspace、ComposerPanel、PostCard stories/tests（初回profile setup testのmock参照も含む）。TimelineFeedはprimary workspaceのTimeline/Thread/Profile等へ共有され、articleの投稿IDとfocus属性は維持した。formatPostDateTimeはPostCardだけで使用し、既存formatterのcallerを変更しない。新規API・IPC・永続化・sensitive sinkはなく、inventoryの新規surfaceは追加承認したINV-5のみ。

## 検証結果

- 対象unit: PostCard 24件＋formatter 3件、計27件成功。
- 返信layout: ja/en/zh-CN × 390/759/760/1440pxと余白・開閉、計14件成功。
- Explore compact/layout: 計25件成功。
- mobile bottom barの既存2件、account-menu、column-resize-contextはtargeted runで確認。
- 最初の`cargo xtask desktop-ui-check`: lint/typecheck成功後、unit/shell計1658件中1655件成功・3件失敗。トピックとリポストの5秒timeout、および後続testの状態混入。Rust build等と同時実行した際の結果で、topics全10件の単独再実行は成功。残る再実行とgate構成要素の結果は以下へ追記する。
- `cargo xtask check`: Rust clippy/Tauri check成功、追加したtestのTesting Libraryに非対応の`exact` optionでtypecheck失敗。正規表現の名前一致へ修正済み。最終desktop-lintの結果を追記する。
- `cargo xtask test`、Storybook build、全browser/visual、最終CI: 実行中。
- Linux visual baseline: workflow run `34794515269`成功。追加のExplore変更後も同workflowで再生成する。

必須検証を省略して成功とみなさない。既存suiteのtimeoutは閾値を緩めず対象再実行で切り分け、AC/INVARの対象差分がない成功項目を理由なく繰り返さない。

### 最終確認の追記

- Windowsのfocus再確認: Escapeでセンターが閉じtriggerに可視focusが戻り、Enterで再openできた。Ubuntu24も同じ操作が成功。両OSで追加のコンパクトなExplore tabsを確認。
- 最終desktop-lint（lint/typecheck）とStorybook build成功。Windowsの実行中xtask.exeをcargoが置換できないため、変更していないビルド済み`target/debug/xtask.exe`から同じサブコマンドを実行した。
- 全browser: 314/316成功。profile-densityの一時的なpointer遮蔽とfeedbackの`ERR_NO_BUFFER_SPACE`に対し、対象ファイルをworkers=1で再実行し17/17成功。全visual操作smokeは38/38成功（Windowsのsnapshot比較skipとは区別する）。
- Linux visual baseline: 追加要件反映後のrun `34795122453`も成功。生成差分を採用し、最終CIで比較する。
- AccountMenuを開いたままセンターへ切り替える遷移もbrowserで成功。
- CIのoversized-filesで新規指定によるscoped CSSの1000行超過を検出。上限を緩めず、今回追加したExplore指定を既存のcommunity-index-tabs定義へまとめ、不要な2行grid指定を置換した。配置変更後の6/6 layout testとoversized-filesは成功。
- Browser testが既存`docs/progress/assets/992/`の2画像を再生成したため、それらの無関係な生成差分は復元して本PRに含めない。

この記録は各確認時点の証拠であり、PR headの最終CI・merge commit・IssueのComplete判定はPR/Issueの現在判定を参照する。手順や必須検証の免除は行わない。
