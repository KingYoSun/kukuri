# ADR 0014: 本番向け UI/UX 開発フロー

## Status
Accepted

**2026-06-13 改訂**: Figma を一次レビュー面とする HTML capture フローを廃止した。旧 `docs/DESIGN.md` の design-system ルール / review チェックリスト / 例外ポリシーを本 ADR に統合し、色・タイポ・余白・コンポーネント等の**具体的なビジュアル仕様は root [`DESIGN.md`](../../DESIGN.md) へ分離**した。

## Context
- 現行 desktop frontend は Tailwind + shadcn/ui + Storybook へ寄せつつ移行中で、未着手の既存 UI も残る。
- repo は public で、外部 contributor からの review も想定する。UI proposal は local 実装差分だけでなく、PR 上で読める review artifact を持つ必要がある。
- 現在の検証基盤は Vitest、`cargo xtask`、GitHub Actions が中心であり、新しい frontend tooling はこの品質ゲートを置き換えずに補完する必要がある。
- Figma を一次レビュー面とする `generate_figma_design` の HTML capture フローは、運用上ほぼ未使用だった（`docs/ui-reviews/` の記録では毎回「Figma capture 未生成」の例外として扱われていた）。実効性が無いため廃止し、一次レビューは PR 上の preview と Storybook に一本化する。

## Decision

### レビューフロー
- UI proposal の一次レビューは **PR 上**で行う。UI proposal PR には次を必須にする。
  - PR 上で読める preview image または short video
  - 変更する user flow の短い summary
  - Shneiderman の 8 つの黄金律に対する checklist 結果、または例外理由
- reusable component と major UI refactor には Storybook story を付ける。一次レビュー面は Storybook と PR preview とする。
- public PR reader が、特別な権限なしに proposal を理解できる状態を必須にする。

### 標準スタック
- 新規 UI と大きく触る既存 UI の標準 stack は Tailwind + shadcn/ui + Storybook にする。
- 未着手の既存 UI は、対象 surface を触るまで現行 path に残してよい。全面移行は前提条件にしない。
- Playwright は、top-level user flow を変える変更、または複数 component をまたぐ回帰リスクが高い変更に限って required にする。Vitest と既存 `cargo xtask` lane は baseline gate として維持する。
- shadcn MCP は標準 stack 導入後の optional な implementation aid。Code Connect は optional future accelerator であり、前提条件にしない。

### ビジュアル仕様
- 色・タイポグラフィ・余白・radius・影・コンポーネント・レイアウト・レスポンシブの**具体的な数値仕様**は root [`DESIGN.md`](../../DESIGN.md) に置く（正本）。
- ランタイムの真実は `apps/desktop/src/styles/tokens.css`。`DESIGN.md` はこれをミラーし、食い違う場合は `tokens.css` を正とする。

### Design System ルール
- **token から始める**。color / typography / spacing / radius / border / shadow / motion は one-off hard-code ではなく shared token layer から取る。
- **primitive を再利用する**。同じ pattern が 2 回以上出たら、markup と style を複製せず reusable component に昇格させる。
- **naming を安定させる**。component 名・variant 名・prop 名は一時的な実装都合ではなく product 上の意味を表す。
- **product UI と diagnostics UI を分離する**。discovery / sync / community-node などの observability panel は、primary user flow よりも視覚的・構造的に後景へ置く。
- **state を設計し切る**。意味のある surface には loading / empty / error / success または interactive state を定義する。
- **desktop window resize に耐える**。最大化 window だけを前提にせず、狭い desktop 幅でも unreadable overflow を起こさない。
- **keyboard access を守る**。primary action / navigation / dialog / menu / form は mouse なしでも到達・理解できるようにする。
- **visible feedback を残す**。focus / hover / selected / disabled / pending / error は識別できるようにする。
- **motion は意図的に使う**。state 変化や hierarchy を説明するために使い、過剰にせず reduced motion も考慮する。

### shadcn 利用ルール
- shadcn/ui は base layer として扱い、完成済み design system と見なさない。
- registry component を追加するときは、merge 前に token / 命名 / variant / state handling / accessibility を kukuri に合わせる。
- kukuri の terminology / layout / state model と衝突する registry default はそのまま採用しない。
- 新しい reusable component と、大きく改修した reusable component には Storybook story を付ける。
- one-off の glue UI は、再利用を想定せず既存 test で十分守られている場合に限って Storybook を省略できる。

### Shneiderman の 8 黄金律チェックリスト
- **Consistency**: topic / thread / channel / settings をまたいでも、似た action / label / layout は同じように振る舞うこと。
- **Shortcuts for frequent users**: 繰り返し操作は click 数削減・context 維持・keyboard-friendly flow により高速化できること。
- **Informative feedback**: save / publish / load / sync wait / consent state / failure を UI が明確に伝えること。
- **Dialog closure**: multi-step flow は、完了状態または次に取るべき action を明示して終わること。
- **Error prevention**: 危険または無効な操作は commit 前に防止または明確化すること。
- **Easy reversal**: user が transient action を取り消すか、mistake から recovery できること。
- **Internal locus of control**: unexplained reset / forced context jump / hidden mode switch で user を驚かせないこと。
- **Reduce short-term memory load**: panel をまたいでも identifier / 前の選択 / 隠れた前提条件を user に覚えさせ過ぎないこと。

### Validation Expectations
- Storybook は、実装開始後の reusable component review surface の既定値とする。
- Vitest は component と UI logic の既定 frontend regression layer として維持する。
- Playwright は、Storybook と Vitest だけでは守りにくい component 境界越えの変更に追加する。
- `cargo xtask check` / `cargo xtask test` は日常 gate、`cargo xtask desktop-ui-check` は browser-aware UI gate として維持し、frontend-only tooling で workspace integration gate を置き換えない。

### 例外ポリシー
- 本 ADR および `DESIGN.md` からの例外は PR description に明記する。
- 例外が受け入れ済み product direction を変える場合は、`docs/ui-reviews/` に短い record を追加する。
- 採用済み UI review record は `docs/ui-reviews/` に残す。

## Consequences
- code だけ、または文章だけの UI proposal は不十分になる。PR-visible preview と user flow summary、Shneiderman checklist が必須になる。
- Figma / `generate_figma_design` への依存は無くなる。一次レビューは PR preview と Storybook で行う。
- Figma edit 権限の有無に関わらず、PR に summary と preview があれば public repo 上で UI review に参加できる。
- reusable component と major UI refactor には Storybook story が必要になる。one-off の glue UI は、再利用を想定せず既存 test で十分守られている場合に限って Storybook を省略できる。
- shadcn MCP は component 探索や registry 作業の加速には使えるが、registry から入れた component をそのまま採用せず、local token / 命名 / variant / state / accessibility へ合わせる必要がある。
- Code Connect は plan と permission 条件が満たされた後に追加できるが、これがなくても workflow は成立し続けなければならない。
- 旧 `docs/DESIGN.md` は、root `DESIGN.md`（ビジュアル仕様）と本 ADR（フロー / ガードレール / 例外ポリシー）に分割・移設された。
