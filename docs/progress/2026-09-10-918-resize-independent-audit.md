# Issue #918 再開修正の独立監査

- 判定: **PASS**
- 対象commit: `3f3327dba4f343d4a739ba849f4399d3a0b93ce9`
- 基準commit: `5db2aeb0853329ac12558a5fbc503553fd6618d0`
- Scope revision: `918-r2`
- リスク区分: B・再Open
- inventory: 合計8／適合8／不適合0／未分類0
- blocker: 0
- 対象: [PR #970](https://github.com/KingYoSun/kukuri/pull/970)。後続の記録追加commitのdelta確認・CI・merge判定はPRを参照する。

実装担当とは独立した担当が、固定AC・DESIGN・ADR 0031と製品差分から先に経路を再構築し、その後に[今回の作業記録](2026-09-10-918-reopen-conditional-clipping.md)、[UI review](../ui-reviews/2026-09-10-918-resize-context.md)、実行ログを照合した。

## 条件との対応

| 条件 | 監査結果 |
| --- | --- |
| AC-1 | 新規testはControl Centerからの通常移動後、resize直後のCanvas内boundsを操作による自動scrollより先に検査する。既存testは3操作・pointer／Enter／Spaceを確認 |
| AC-2 | 長URL・Node別回復・pending等の既存testを保持。案内、送信先、callbackの製品差分なし |
| AC-3 | CSS・token変更なし。既存computed style testの本文14px・投稿15px・補助12px・header16pxと成功ログを確認 |
| AC-4 | 3言語×2themeのdesktop／mobile境界往復、新規13件の独立実行成功。既存reflow・localization・visualとnative画像を照合 |
| INVAR-1 | resize callbackはDOM scrollと既存scroll guardだけを変更。検索・規約取得・同意保存のcall記録が空であることを新規testで検査 |
| INVAR-2 | query・focus・active・保存layout同一性を検査。手動scrollでactiveから離れた状態を引き戻さない負の条件も成功 |
| INVAR-3 | semantic属性・DOM順・色・target不変。bounds・focus検査、native画像、a11y132条件の違反0と対応 |

## 入口・遷移の監査

製品callerは `DesktopShellColumnWorkspace` のみ。Story、`ColumnCanvas.test.tsx`、`ColumnCanvas.swipe.test.tsx`も列挙して一致した。製品callerから全10種類のColumnへ適用され、可視集合の変更は既存runtime経路へ流れる。

幅不変時の早期return、対象不存在、手動scroll、幅変更に先行するsnap、mobile補正、settle取消し、programmatic targetの解除、active変更時の再購読、unmount時のlistener／observer解除を確認した。補正scroll自身はactive・route・保存stateを書かず、既存settleの誤認をguardする。R-TR-1〜5に未分類の遷移なし。

保存復元testは `threeColumns:false` のinit scriptから開始し、初回だけページから3カラムを設定する。reload前後では保存値を再投入せず比較するため、従来fixtureの再seedによる偽陽性を回避している。

## 実行・照合したvalidation

- 独立実行: `npx pnpm@10.16.1 exec playwright test tests/playwright/column-resize-context.spec.ts --project chromium --workers 2`。13件成功、15.3秒。
- 独立実行: `git diff --check`。成功。監査による作業tree変更なし。
- 照合: Linux check、Vitest165ファイル／1314件、browser165件、visual20件、a11y132条件。新規test名・assertionと成功ログを対応付けた。

## non-blockerとした限界

- native操作は監査担当が再実行したものではなく、記録と4枚の画像を確認した。通常resizeの前後でExploreの見切れ解消を目視できる。
- native zoomは検証用capabilityを使った製品buildであり、既定capabilityによるzoom操作の証明ではない。この限界は記録済み。
- nativeは未同意案内・回復・resize／zoom・復元を担当し、検索成功／pending／retryはcomponent／browserの証拠である。
- Windows正常Quit時の永続化、実Node検索成功、screen reader全体は今回の証明範囲外。対象差分による具体的Regressionは確認していない。

固定条件が証拠へ対応し、inventory未分類0・不適合0・blocker0で監査停止条件を満たした。対象surfaceが変わらなければこのPASSを利用し、必須CI成功後のmergeへ進める。
