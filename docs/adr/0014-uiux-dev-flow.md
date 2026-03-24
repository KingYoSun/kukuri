# ADR 0014: 本番向け UI/UX 開発フロー

## Status
Accepted

## Context
- 現行 desktop frontend は、依然として `apps/desktop/src/App.tsx` と `apps/desktop/src/styles.css` を中心に構成されている。
- repo は public で、外部 contributor からの review も想定する。UI proposal は local 実装差分だけでなく、PR 上で読める review artifact を持つ必要がある。
- Codex が UI を提案する場合、code review より前に人間が一次レビューできる面が必要であり、その正本は Figma に置く。
- 現状の official Figma MCP では、自然言語だけから自由度の高い高忠実度 UI を安定生成するより、local HTML / React draft を `generate_figma_design` で capture する方が再現性が高い。
- 現在の検証基盤は Vitest、`cargo xtask`、GitHub Actions が中心であり、新しい frontend tooling はこの品質ゲートを置き換えずに補完する必要がある。
- 今後の UI 実装は Tailwind + shadcn/ui + Storybook へ寄せるが、現行 app はまだ全面移行されていない。
- Figma MCP は review 用 design data の生成に使える。shadcn MCP と Code Connect は将来的な支援手段だが、workflow の前提条件にはしない。

## Decision
- Codex が作る UI proposal は、実装パッチに加えて Figma data を作成または更新しなければならない。人間の一次レビューは Figma 上で行う。
- Codex-assisted UI proposal の標準フローは次に固定する。
  - text instruction から local HTML / React draft を作る
  - `generate_figma_design` の HTML capture で Figma design を生成または更新する
  - 人間が Figma 上で一次レビューし、必要なら Figma 側で修正する
  - Codex は修正版 Figma を再取り込みして code へ反映する
- Figma link は、原則として HTML capture で生成した Figma design URL を使う。FigJam は IA 補助 artifact としては許容するが、primary review artifact の代替にはしない。
- UI proposal PR には次を必須にする。
  - Figma link
  - PR 上で読める preview image または short video
  - 変更する user flow の短い summary
  - Shneiderman の 8 つの黄金律に対する checklist 結果、または例外理由
- 新規 UI と大きく触る既存 UI の標準 stack は Tailwind + shadcn/ui + Storybook にする。
- 未着手の既存 UI は、対象 surface を触るまで現行の `App.tsx` / `styles.css` path に残してよい。全面移行は前提条件にしない。
- Playwright は、top-level user flow を変える変更、または複数 component をまたぐ回帰リスクが高い変更に限って required にする。Vitest と既存 `cargo xtask` lane は baseline gate として維持する。
- 製品 UI と diagnostics UI は、設計と review の両方で分離する。discovery、community-node、sync などの observability surface を primary product flow として扱わない。
- tool の優先順位は次に固定する。
  - Figma MCP: required
  - shadcn MCP: 標準 stack 導入後の optional implementation aid
  - Code Connect: optional future accelerator。前提条件にしない
- UI/UX の guardrail、review checklist、例外ポリシーは `docs/DESIGN.md` に置く。
- 採用済み UI review record は `docs/ui-reviews/` に残す。

## Consequences
- code だけ、または文章だけの UI proposal は不十分になる。Figma review data と PR-visible preview が必須になる。
- text-only の Figma 生成を前提にしない。Codex は review 用の HTML / React draft を先に作る責務を持つ。
- local capture のために一時的に挿入した script や URL hook は、product requirement でない限り capture 後に戻す必要がある。
- Figma edit 権限を持たない contributor でも、PR に summary と preview があれば public repo 上で UI review に参加できる。
- reusable component と major UI refactor には Storybook story が必要になる。one-off の glue UI は、再利用を想定せず既存 test で十分守られている場合に限って Storybook を省略できる。
- shadcn MCP は component 探索や registry 作業の加速には使えるが、registry から入れた component をそのまま採用せず、local token、命名、variant、state、accessibility へ合わせる必要がある。
- Code Connect は plan と permission 条件が満たされた後に追加できるが、これがなくても workflow は成立し続けなければならない。
