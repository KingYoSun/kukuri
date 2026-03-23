# 2026-03-24 desktop phase0 frontend foundation

- PR: [#213](https://github.com/KingYoSun/kukuri/pull/213)
- Figma: なし。この PR は infra-first の foundation 追加として扱い、merge 前の standalone な Figma proposal は作成していない。例外として受け入れた。
- Summary: Phase 0 として `apps/desktop` の frontend foundation を追加した。Tailwind と shared token layer、shadcn 互換の base primitive、Storybook の review surface、shared desktop mock fixture、browser-only の Playwright smoke path を導入した。既存 shell の構造、Rust/Tauri invoke surface、`DesktopApi` contract は維持した。
- User flow summary: 既存 shell の boot、publish、render の挙動は同じ surface に残した。この PR で追加したのは review / regression 用の基盤が中心で、現行 shell 内の control / panel レベルに限定して primitive を適用した。
- Review result: shell UI production migration の Phase 0 foundation slice として採用、merge 済み。後続の UI phase では通常どおり Figma-first の review flow を前提にする。
- Shneiderman: checklist 省略の例外を受理。この PR は infra-first の slice であり、top-level user flow や information architecture の変更が最小だったため、完全な checklist は PR に添付していない。
- Exceptions: PR 213 には Figma link、PR-visible preview image または short video、PR 内の Shneiderman checklist を付けていない。この欠落は一回限りの Phase 0 foundation slice に対する process debt としてここに記録し、後続 phase へ持ち越さない。
- Validation: `cd apps/desktop && npx pnpm@10.16.1 lint`; `cd apps/desktop && npx pnpm@10.16.1 typecheck`; `cd apps/desktop && npx pnpm@10.16.1 build`; `cd apps/desktop && npx pnpm@10.16.1 test`; `cd apps/desktop && npx pnpm@10.16.1 storybook --smoke-test`; `cd apps/desktop && npx pnpm@10.16.1 storybook:build`; `cd apps/desktop && npx pnpm@10.16.1 test:e2e:browser`; `cargo xtask check`; `cargo xtask e2e-smoke`。local 検証では touched frontend surface の外側にある `kukuri-desktop-runtime` の flaky failure により `cargo xtask test` が不安定だったが、その後 PR 213 は当該 lane が green の状態で merge された。
