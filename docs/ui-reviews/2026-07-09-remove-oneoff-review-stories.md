# 2026-07-09 remove-oneoff-review-stories

- PR: WP-Q1 PR6（dead code 削除バッチ）
- Preview: なし（削除のみ。UI 挙動・見た目の変更なし）
- Summary: `apps/desktop/src/components/review/` の一回性 review Storybook 2 本
  （`DesktopShellReactionComposerFixes.stories.tsx` / `DesktopShellWorkspaceGallery.stories.tsx`、
  計 1,094 行）を削除した。これらは既に完了した review の成果物で、他モジュールからの
  import は無く、共有 `@/components/storyFixtures` 等は残す。対応する durable な
  decision record は
  [2026-03-30-desktop-shell-reaction-composer-fixes.md](2026-03-30-desktop-shell-reaction-composer-fixes.md)
  および shell workspace 系の phase review 記録に既に存在するため、Storybook 本体は
  git 履歴に残しつつ削除して保守対象から外す。
- Review result: 採用（review 完了済みの一回性成果物のため削除）
- Exceptions: なし
- Validation: `pnpm typecheck` / `cargo xtask desktop-ui-check`（lint / unit /
  storybook:build / e2e browser / 視覚回帰 14 面）緑。削除対象 stories を import する
  箇所は grep 0。
