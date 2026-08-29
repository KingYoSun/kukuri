# Issue #824 UI/UX quality contracts

## Summary

`DESIGN.md`を製品固有の設計契約、ADR 0014を成果物ベースの開発フローとして再構成した。実装配置は`docs/architecture/desktop-ui-implementation.md`へ分離し、PR template、UI review record、Storybook Foundations、CSS／token contract testを同じ用語へ揃えた。

外部skillは参考情報に留め、repository内のdocs、tests、Storybook、Playwright、Tauri／WebView実機結果を合否の正本とする。

## Contract surfaces

- 設計契約: root `DESIGN.md`
- UI実装配置: `docs/architecture/desktop-ui-implementation.md`
- 開発・検証flow: `docs/adr/0014-uiux-dev-flow.md`
- 実行token: `apps/desktop/src/styles/tokens.css`
- 描画確認面: `apps/desktop/src/stories/foundations/`
- PR証跡: `.github/PULL_REQUEST_TEMPLATE.md`
- 採用記録: `docs/ui-reviews/README.md`

## Current implementation audit

| Severity | Evidence | Observation | Tracking |
|---|---|---|---|
| P1 | `apps/desktop/src/styles/tokens.css:128,153` | light themeのprimary面`#d77d45`と文字`#fff7ef`は約2.87:1で、通常文字4.5:1を満たさない | #828 |
| P1 | `apps/desktop/src/styles/tokens.css:142,156` | light themeのdestructive淡面`#f6dfd4`と文字`#b35f46`は約3.54:1で、通常文字4.5:1を満たさない | #828 |
| P2 | `apps/desktop/src/components/shell/ColumnSurface.tsx:155-169`、`ColumnCanvas.tsx:264-328` | HTML dragとpointer captureが混在し、実pointerでの確実な並べ替えが未完了 | #812 |
| P2 | `apps/desktop/src/shell/viewModels/useTimelineViewModels.ts:255-260`、`components/core/PostMedia.tsx:84-85` | media取得不能時も`loading`／skeletonへ残り得る経路があり、状態に終端がない | #814 |
| P2 | `apps/desktop/src/components/core/AuthorDetailCard.tsx:122`、`CommunityIndexWorkspace.tsx:393` | 通常面に完全なpubkey／object idを常時表示する箇所が残る | #815、#820 |
| P2 | `apps/desktop/src/components/shell/ColumnSurface.tsx:154,190,207` | icon操作にはaccessible nameがあるが、hoverとkeyboard focusで共通説明を出す契約が未実装 | #817 |
| P3 | `apps/desktop/src/components/ui/button.tsx:18-20` | defaultは44px、`sm`は36px、`icon`は40px。Desktop profileでは24px最小とspacingを満たす場合に許容し、touch利用箇所は44px目標として個別確認する | #817でheader操作を確認。component値自体は本Issueで変更しない |

## Existing tracked product work

- state継続と派生Column位置: #813
- Timeline／Exploreの切替: #816
- 日本語文言と法務表示: #818
- keyboard投稿shortcut: #819

これらは本Issueで規範と検証条件を追加するが、production UIの個別修正は各Issueに残す。

## Audit outcome

- 長文、空値、日本語／英語／中国語、狭幅は既存`apps/desktop/tests/playwright/localization-layout.spec.ts`を変更種別別validationへ接続した。新しい個別不具合は本監査では確定していない。
- performanceは一律の推測修正を行わず、長い一覧、Stream／Metaverse、store購読、global listener、localStorage versionを対象変更時に計測する契約とした。本監査だけを根拠にproduction codeを変更しない。
- contrast以外の既知差分は#812〜#820に追跡先があり、contrastは#828へ分離した。

## Validation

- `npx pnpm@10.16.1 exec vitest run src/styles/css-vars.test.ts src/styles/design-contract.test.ts`: 3 tests passed
- `cargo xtask check`: success
- `cargo xtask test`: Rust 694、harness 22、frontend 917 tests passed。doctest success
- `cargo xtask desktop-ui-check`: frontend 917、Playwright browser 44、visual 14 tests passed。Storybook build success
- Storybook `Foundations/Tokens`: dark／lightをPlaywrightで実描画し、primary面がそれぞれ`rgb(245, 157, 98)`／`rgb(215, 125, 69)`へ切り替わることを確認
- `git diff --check`: success
