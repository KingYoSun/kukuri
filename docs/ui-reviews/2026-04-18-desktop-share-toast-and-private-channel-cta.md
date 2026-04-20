# 2026-04-18 desktop share toast and private-channel cta

- PR: local workspace turn 2026-04-18 desktop clipboard feedback, pagination stabilization, and relay-assisted refresh
- Figma: not updated in this turn. This record covers the shipped desktop behavior changes only.
- Summary: clipboard copy actions routed through the desktop shell now surface a short success toast with `aria-live="polite"` and auto-dismiss behavior. Private channel detail replaces the fixed `Share Token` CTA copy with `channel label / audience label` and renders the `DoorOpen` icon on the trailing edge so the button reads like a channel-scoped share target instead of a generic export action.
- Review result: shell-level copy feedback appears once per successful copy action, the private-channel CTA reflects the selected channel name and audience scope, and the new copy does not regress the existing topic/profile/private-channel shell flows.
- Validation: `cd apps/desktop && npx pnpm@10.16.1 vitest run src/shell/DesktopShellPage.test.tsx src/components/core/PostCard.test.tsx src/components/settings/SettingsPanels.test.tsx`; `cargo test -p kukuri-desktop-runtime tests::community_node_ -- --nocapture`; `cargo xtask scenario community_node_public_connectivity` passed.
