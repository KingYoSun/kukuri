# 2026-08-24 issue-768-validation-completion

- PR: https://github.com/KingYoSun/kukuri/pull/770 （Issue #768 / #748 監査コメント https://github.com/KingYoSun/kukuri/issues/748#issuecomment-5387831754 ）
- Preview: [Community Node unavailable（trigger 状態 dot + Explore inline Notice）](assets/2026-08-24-issue-768/community-node-unavailable.png) / [Notifications / Messages Column](assets/2026-08-24-issue-768/notifications-and-messages-columns.png) / [Stream 1 span](assets/2026-08-24-issue-768/stream-one-span.png) / [Metaverse 1 span](assets/2026-08-24-issue-768/metaverse-one-span.png) / [narrow Desktop 760px](assets/2026-08-24-issue-768/narrow-desktop-columns.png)
- Summary:
  - Issue #748 Validation マトリクスの欠落を補完した。Storybook: `Review/ProductionColumnWorkspace` に NotificationsAndMessagesColumns / StreamOneSpan / MetaverseOneSpan / CommunityNodeHealthy / CommunityNodeUnavailable / NarrowDesktopColumns / ReducedMotionProduction を追加し、MobilePagingAndImmersiveLifecycle は Storybook コアの viewport（390×844）で preview iframe 自体を縮めて production の `@media (max-width: 759px)` 規則（snap / indicator / gesture owner）を発火させる形にした。
  - Playwright: `column-scope.spec.ts`（新規 5 件: 複数 scope Timeline 同時表示と投稿の scope 分離 / private Thread の header・footer 返信 scope / Conversation footer の DM 送信 / Control Center Focus の desktop・mobile）を追加。既存 smoke の reorder を実 key 入力（Enter → ArrowDown / ArrowUp → Enter）駆動へ差し替え、mobile paging test を 375×812 / 390×844 / 430×932 のパラメタ実行にした。
  - reduced motion の JS 抑制（Column 切替 scroll / edge auto-scroll）は `prefersReducedMotion()` helper（OS の `prefers-reduced-motion` 最優先 + review 用 `data-reduced-motion='reduce'` の OR。production は属性を設定しないため挙動不変）経由となり、Storybook の Reduced motion toggle で CSS token と JS の両方を review できる。
  - 記録: Wave 3〜6 record へ PR link（#755 / #756 / #757 / #758）と reduced motion / 375・430px の追記、ADR 0031 Consequences へ Fullscreen・Stream player・Metaverse footer の「対象外 / #766 送り」を明記した（いずれも日付付き追記で当時の判断本文は不変）。
- Review result:
  - built Storybook（storybook-static）を browser で確認。新 story 全件 render、unavailable story で trigger accessible name「Open Community Node needs attention」系の状態 dot と Explore Column の `community-node-unavailable-notice` が同時に見え、healthy では Notice が無い。Stream 1 span は `.shell-stream-layout` 1 track、Metaverse 1 span は HUD overlay、narrow 760px は Canvas 内 scroll のみで document 横 overflow なし。
  - mobile story は manager UI 経由で preview iframe が実測 390px になり、iframe 内で `(max-width: 759px)` が match、`scroll-snap-type: x mandatory` / 44px page indicator / gesture owner `touch-action: none` を確認した（iframe.html 直開きでは viewport が効かない点に注意）。
  - 発見事項: Explore Column（1 span）内の unavailable Notice はタイトル列が狭く 1 語ずつ折り返す。機能上の問題は無いが、Notice の narrow layout 調整は presentation 課題として記録する（Issue #765 / #766 の対象外。必要なら別途起票）。
- Exceptions: Fullscreen 退出後の layout 復帰 / Stream seek 非競合の Playwright は機能未実装のため対象外（ADR 0031 Consequences の 2026-08-24 追記と Issue #766 を参照）。
- Validation:
  - `cd apps/desktop && npx vitest run`（helper / story fixture / ColumnCanvas 追加テスト含む全件）
  - `npx playwright test --project=chromium`（31 件 = 既存 24 + column-scope 5 + mobile paging パラメタ +2）/ `--project=visual`（14 件、baseline 変更なし）
  - `npx pnpm@10.16.1 storybook:build`
  - `cargo xtask desktop-ui-check` / `cargo xtask check` / `git diff --check`
