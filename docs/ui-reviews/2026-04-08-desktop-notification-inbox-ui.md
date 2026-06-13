# 2026-04-08 desktop-notification-inbox-ui

- PR: local workspace change for the desktop notifications inbox slice. PR identifier is pending.
- Figma: pending HTML capture URL for the notifications inbox review. This implementation turn did not generate a new dedicated Figma capture before validation.
- Summary: left sidebar の「トピック」見出しより上に通知ボタンを追加し、未読件数バッジと `#/notifications?topic=...` の inbox route を導入した。通知ページでは loading / empty / load error / auto-read error を描画し、一覧表示時に未読通知を自動で既読化する。reply / mention / repost / quote repost は timeline thread、direct message は messages pane、followed は timeline author pane へ click-through する。
- Review result: notification backend / Tauri API を再利用した shell 導線と inbox surface を採用した。未読件数は 2 秒 poll で全画面更新し、`NotificationView.thread_root_object_id` は read-time enrichment で返す方針としたため durable schema migration は不要。
- Exceptions: Figma review capture は pending。`docs/DESIGN.md` の required workflow に対して、この turn は automated validation と local code review を先行し、HTML capture artifact の生成は後続補完扱いにした。
- Validation: `cd apps/desktop && npx pnpm@10.16.1 exec tsc --noEmit` passed. `cd apps/desktop && npx pnpm@10.16.1 exec vitest run src/shell/DesktopShellPage.test.tsx src/shell/routes.test.tsx` passed. `cd apps/desktop && npx pnpm@10.16.1 exec playwright test tests/playwright/hash-routing.spec.ts` passed. `cargo test -p kukuri-app-api notifications -- --nocapture` passed.
- Note (2026-06-13 #308): 上記 `docs/DESIGN.md` の required Figma workflow は廃止された。フロー / ガードレール / 例外ポリシーは `docs/adr/0014-uiux-dev-flow.md`、ビジュアル仕様は root `DESIGN.md` へ移設。本記録の Figma 言及は当時の履歴。
