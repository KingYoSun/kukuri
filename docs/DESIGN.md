# kukuri UI/UX Design Rules

## Purpose
- この文書は、現行 kukuri workspace における UI proposal、review、implementation の既定ルールを定義する。
- workflow 自体は `docs/adr/0014-uiux-dev-flow.md` が定義し、この文書はその中で使う design-system rule と review rule を定義する。

## Applies To
- `apps/desktop` の user-facing UI
- Storybook story と UI review artifact
- Codex が implementation patch と Figma review data を同時に生成する UI proposal

## Required Workflow
1. Codex が UI を新規提案または大きく改修する場合、先に Figma で proposal を作る。
2. 一次レビューは Figma で行う。
3. code review に耐える方向性が固まってから実装する。
4. 変更の大きさとリスクに応じて Storybook、Vitest、Playwright、`cargo xtask` で検証する。
5. merge された変更が user-facing behavior または design rule を変える場合は、`docs/ui-reviews/` に記録を残す。

## Tooling Policy
- Figma MCP は Codex-assisted UI proposal で required とする。Figma を primary review artifact にする。
- 新規 UI と大きく触る既存 UI の標準 stack は Tailwind + shadcn/ui + Storybook にする。
- Playwright は top-level user flow を変える変更、または複数 component をまたぐ回帰リスクが高い変更で required とする。
- shadcn MCP は optional とする。対象 surface に標準 stack が入ってから使う。
- shadcn MCP は implementation aid であり、review system でも Figma MCP の代替でもない。
- Code Connect は optional とし、workflow の blocker にしない。

## Review Artifacts
- すべての UI proposal PR は Figma link を含む。
- すべての UI proposal PR は、PR 上で直接 review できる preview image または short video を含む。
- すべての UI proposal PR は、変更する user flow の短い summary を含む。
- すべての UI proposal PR は、Shneiderman checklist の結果または例外理由を含む。
- public PR reader が Figma edit 権限なしでも proposal を理解できる状態を必須にする。

## Design System Rules
- token から始める。color、typography、spacing、radius、border、shadow、motion は one-off hard-code ではなく shared token layer から取る。
- primitive を再利用する。同じ pattern が 2 回以上出たら、markup と style を複製せず reusable component に昇格させる。
- naming を安定させる。component 名、variant 名、prop 名は一時的な実装都合ではなく product 上の意味を表す。
- product UI と diagnostics UI を分離する。discovery、sync、community-node などの observability panel は、primary user flow よりも視覚的・構造的に後景へ置く。
- state を設計し切る。意味のある surface には loading、empty、error、success または interactive state を定義する。
- desktop window resize に耐える。最大化 window だけを前提にせず、狭い desktop 幅でも unreadable overflow を起こさない。
- keyboard access を守る。primary action、navigation、dialog、menu、form は mouse なしでも到達・理解できるようにする。
- visible feedback を残す。focus、hover、selected、disabled、pending、error は識別できるようにする。
- motion は意図的に使う。state 変化や hierarchy を説明するために使い、過剰にせず reduced motion も考慮する。

## shadcn Usage Rules
- shadcn/ui は base layer として扱い、完成済み design system と見なさない。
- registry component を追加するときは、merge 前に token、命名、variant、state handling、accessibility を kukuri に合わせる。
- kukuri の terminology、layout、state model と衝突する registry default はそのまま採用しない。
- 新しい reusable component と、大きく改修した reusable component には Storybook story を付ける。
- one-off の glue UI は、再利用を想定せず既存 test で十分守られている場合に限って Storybook を省略できる。

## Shneiderman Checklist
- Consistency: topic、thread、channel、settings をまたいでも、似た action、label、layout は同じように振る舞うこと。
- Shortcuts for frequent users: 繰り返し操作は click 数削減、context 維持、keyboard-friendly flow により高速化できること。
- Informative feedback: save、publish、load、sync wait、consent state、failure を UI が明確に伝えること。
- Dialog closure: multi-step flow は、完了状態または次に取るべき action を明示して終わること。
- Error prevention: 危険または無効な操作は commit 前に防止または明確化すること。
- Easy reversal: user が transient action を取り消すか、mistake から recovery できること。
- Internal locus of control: unexplained reset、forced context jump、hidden mode switch で user を驚かせないこと。
- Reduce short-term memory load: panel をまたいでも identifier、前の選択、隠れた前提条件を user に覚えさせ過ぎないこと。

## Validation Expectations
- Storybook は、実装開始後の reusable component review surface の既定値とする。
- Vitest は component と UI logic の既定 frontend regression layer として維持する。
- Playwright は、Storybook と Vitest だけでは守りにくい component 境界越えの変更に追加する。
- `cargo xtask` は workspace 全体の integration gate として維持し、frontend-only tooling で置き換えない。

## Exceptions
- この文書からの例外は PR description に明記する。
- 例外が受け入れ済み product direction を変える場合は、`docs/ui-reviews/` に短い record を追加する。
