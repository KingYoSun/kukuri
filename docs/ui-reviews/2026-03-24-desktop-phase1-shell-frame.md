# 2026-03-24 desktop phase1 shell frame

- PR: local workspace change for shell UI production migration Phase 1. PR identifier is pending.
- Figma: [Phase 1 shell HTML capture](https://www.figma.com/design/4AbJxI5rOI91h9WLPtD3NQ)
- Summary: `apps/desktop` の shell を 3-pane 基準の frame へ差し替え、left nav rail、workspace、thread/author context、settings drawer を分離した。main workspace から diagnostics / connectivity / discovery / profile editor を退避し、topic navigation、composer、timeline、live、game は既存 contract のまま中央 pane に残した。
- User flow summary: 起動後の初期 focus path は `Skip to workspace` から始まり、top bar actions、topic nav、workspace controls、context trigger / tabs、settings drawer contents の順で辿れる。wide では thread / author context を右 pane に常設し、medium / narrow では drawer に切り替える。settings hub から `Save Seeds`、`Save Nodes`、`Authenticate`、`Consents`、`Accept`、`Refresh`、`Clear Token`、`Import Peer` へ既存順序のまま到達できる。
- Review result: shell UI production migration の Phase 1 slice として採用。router や backend contract を増やさずに shell boundary と information architecture を先行固定する方針を承認した。
- Shneiderman: 一貫性は section / drawer / context tab の命名と操作順維持で担保した。頻出操作のショートカットは focus jump と persistent context tab で補強した。情報量の圧縮は diagnostics を hub へ退避し、workspace には summary badge のみ残す方針で評価した。`Esc` で overlay を閉じて trigger へ focus を戻すこと、active nav に `aria-current`、tabs に tab semantics、drawer trigger に `aria-expanded` / `aria-controls` を持たせることを review 条件として確認した。
- Exceptions: 一次 review では HTML capture の Figma design と local Storybook stories を基準に進めた。初期 IA の検討には FigJam proposal も併用した。PR-visible preview image / short video はこの local 実装時点では未作成のため、PR 作成時に補完する前提で記録する。
- Validation: `cd apps/desktop && npx pnpm@10.16.1 build`; `cd apps/desktop && npx pnpm@10.16.1 test`; `cd apps/desktop && npx pnpm@10.16.1 storybook:build`; `cd apps/desktop && npx pnpm@10.16.1 test:e2e:browser`; `cargo xtask check`; `cargo xtask test`; `cargo xtask e2e-smoke`。
