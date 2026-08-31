# Issue #828 light theme semantic color contrast

## Summary

light themeの実使用semantic pairをWCAG 2.2 AA（通常文字4.5:1、large text／non-text UI 3:1）へ揃えた。warm-orange × cool-tealのidentityとdark themeは変更していない。`:root[data-theme='light']`の10 tokenだけを明度調整し、`DESIGN.md`のtoken contract表を同期した。

primary buttonはdark themeと同じ解法（オレンジ面 + 濃色ink）へ統一し、`--primary-foreground`をdark themeの`#0e1b26`と同値にした。背景`#d77d45`は変更していない。

## Token changes（light theme のみ）

| Token | 旧値 | 新値 | 主要ペアの達成比 |
|---|---|---|---|
| `--primary-foreground` | `#fff7ef` | `#0e1b26` | 5.74 on `--surface-button-primary`、4.81 on hover（旧2.87） |
| `--destructive` / `--danger` | `#b35f46` | `#9d4d36` | 4.63 on `--surface-destructive-soft`、5.92 on `--surface-panel`（旧3.54／4.52） |
| `--warning` | `#9a6e2a` | `#845e21` | 4.81 on `--surface-warning-soft`、5.09 on `--background`（旧3.74／3.95） |
| `--accent` | `#0f8c82` | `#0c756c` | 5.56 on `--surface-panel`、4.86 on `--background`（旧4.12／3.60） |
| `--muted-foreground-soft` | `#74818a` | `#626d77` | 4.82 on `--surface-input`、4.55 on `--surface-panel-accent`（旧3.65／3.45） |
| `--border-destructive` | `#d89b86` | `#b56a50` | 3.19 vs soft、4.08 vs panel（旧1.83／2.35） |
| `--border-warning` | `#d1a06d` | `#a67839` | 3.23 vs soft、3.91 vs panel（旧1.94／2.34） |
| `--border-accent` | `#78a8a2` | `#4f8b84` | 3.23 vs `--surface-accent-soft`、3.92 vs panel（旧2.19／2.65） |
| `--ring` | `rgba(15, 140, 130, 0.32)` | `rgba(12, 117, 108, 0.8)` | 合成後3.74 over panel、3.44 over `--background`（旧1.51／1.47） |

合格済みで変更しなかった主なペア: `--surface-accent-soft`／`--accent-foreground` 10.79、`--surface-warning-soft`／`--foreground` 11.19、`--surface-badge-neutral`／`--muted-foreground` 4.79、`--surface-input`／`--foreground` 12.36、`--surface-button-secondary`／`--foreground` 10.75、`--surface-selection`／`--foreground` 7.23。

disabled button（`opacity: 0.56`）はWCAG 1.4.3のdisabled除外として対象外。dark themeのtokenとvisual baselineは変更していない。

## Regression gate

`apps/desktop/src/styles/contrast.test.ts`を新設した。light themeの実使用ペア（text 36件 + ring合成3件）を明示テーブルで持ち、tokens.cssの実値から相対輝度でWCAG比を検証する。token変更前は23件failし、変更後に全件passすることを確認した（failing-test-first）。dark themeは#828の対象外としてテストに含めない。

## Validation

- `npx pnpm@10.16.1 exec vitest run src/styles/`: 46 tests passed（contrast 39、design-contract、css-vars、unused-selectors、native-select）
- `cargo xtask desktop-ui-check`: success（lint、typecheck、frontend 1031、Storybook build、Playwright browser 52、visual 14）
- visual baseline: light 3枚（`timeline-wide-light`、`timeline-narrow-light`、`settings-appearance-wide-light`）を`kukuri-visual-baseline` workflowで再生成して差し替え。dark baselineは無変更
- Storybook `Foundations/Tokens`ほかをlight themeで実描画確認（`docs/ui-reviews/2026-08-31-issue-828-light-theme-contrast.md`参照）

関連: #824（監査元）、`docs/progress/2026-08-29-issue-824-ui-ux-quality-contracts.md`
