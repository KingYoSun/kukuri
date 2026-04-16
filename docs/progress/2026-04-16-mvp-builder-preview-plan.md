# 2026-04-16 MVP Builder Preview Plan

## Summary

- このマイルストーンは general launch ではなく `builder preview` の切り出しです。
- capability baseline は [2026-03-10-foundation.md](./2026-03-10-foundation.md) を維持し、その上に `初回体験 / 配布 / 説明 / feedback loop` を載せます。
- Community Node は最後まで単一の概念として扱います。preview 向けの自動導線は node ごとの `auto_approve` で制御します。
- current preview surface は `launch -> auto-approved community node ready -> starter topic -> post/reply -> private channel -> feedback` です。

## Current Snapshot

- runtime は `community-node.json` を後方互換で読みつつ、fresh install では `https://api.kukuri.app` を `auto_approve=true` の Community Node として preload します。
- `auto_approve=true` の node は起動時に `authenticate -> consent accept -> metadata refresh` を自動で進めます。
- token は期限 5 分前から proactive refresh し、`401` は `re-authenticate -> retry`、`403 CONSENT_REQUIRED` は `auto_approve=true` node だけ `accept -> retry` します。
- desktop settings は textarea editor をやめ、Community Node の単一 list 上で `base URL`, `auto_approve`, diagnostics, troubleshooting actions を扱います。
- starter topic は `kukuri:topic:demo`, `kukuri:topic:iroh`, `kukuri:topic:nostr`, `kukuri:topic:operators` を default とします。

## Preview Surface

- packaged distribution: Windows NSIS installer via GitHub Releases
- source-run fallback: Linux
- docs: root README, `docs/runbooks/mvp-user-quickstart.md`, `docs/runbooks/mvp-troubleshooting.md`
- feedback home: GitHub を canonical とし、preview announcement 前に Discussions か同等の feedback surface を有効化する

## Workstreams

| Workstream | Status | Type | Notes |
| --- | --- | --- | --- |
| Community Node config `auto_approve` | landed | repo change | runtime persistence, Tauri payload, frontend type を更新 |
| Startup auto onboarding | landed | repo change | auto-approved node の auth / consent / metadata refresh を自動化 |
| Token expiry auto re-auth | landed | repo change | proactive refresh, `401` retry, `403` conditional accept を追加 |
| Community Node unified settings surface | landed | repo change | official/custom split を作らず row editor に置換 |
| Starter topics default | landed | repo change | desktop shell default tracked topics を 4 件に変更 |
| Preview docs refresh | landed | repo change | README, docs index, user quickstart, troubleshooting を追加 |
| Windows release workflow | landed | repo change | tag / manual dispatch で NSIS asset を Release に載せる |
| Seed content on hosted preview node | planned | launch op | project-owned author で preview topics を事前投入する |
| GitHub feedback surface | planned | launch op | Discussions category か同等の GitHub feedback home を整備する |

## Validation Matrix

| Path | Gate |
| --- | --- |
| runtime compile | `cargo check -p kukuri-desktop-runtime` |
| frontend settings regression | `npx pnpm@10.16.1 --dir apps/desktop test` |
| workspace fast path | `cargo xtask check` |
| UI path | `cargo xtask desktop-ui-check` |
| community-node path | `cargo xtask cn-test` |
| smoke path | `cargo xtask e2e-smoke` |
| preview scenario | `cargo xtask scenario community_node_public_connectivity` |

## Launch Checklist

- [ ] GitHub Release で Windows installer を公開
- [ ] README 冒頭を builder preview 導線へ更新
- [ ] `mvp-user-quickstart` と `mvp-troubleshooting` を公開
- [ ] hosted preview node 上で starter topic seed content を確認
- [ ] GitHub feedback surface を preview copy から辿れるようにする
- [ ] packaged Windows app で `launch -> ready -> post -> reply -> private channel` を手動確認

## Assumptions

- `auto_approve` は node ごとの UX policy であり、Community Node の新しい種別ではありません。
- community-node server endpoint contract 自体は変更しません。
- Linux binary packaging、hosted Storybook、general-public launch、moderation tooling はこの milestone の exit criteria に含めません。
