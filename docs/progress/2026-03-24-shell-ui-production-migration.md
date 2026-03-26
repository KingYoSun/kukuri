# 2026-03-24 shell UI production migration

## Summary
- この文書は、現行 `apps/desktop` shell UI を本番向け UI へ移行するための実行計画を固定する。
- 正本はこの `docs/progress/` 文書 1 本とし、UI/UX workflow は `docs/adr/0014-uiux-dev-flow.md`、design-system / review rule は `docs/DESIGN.md` を前提にする。
- scope は whole shell rewrite だが、execution order は `infra-first -> staged slices` に固定し、big-bang rewrite は前提にしない。
- この計画では backend、Tauri command、frontend API contract を変更しない。将来 contract 変更が必要な場合は、この文書に黙って含めず別の implementation change または ADR で明示する。

## 2026-03-25 Status Update
- Phase 5 の cutover and cleanup は 2026-03-25 時点で implementation 完了扱いに更新した。
- shell routing は `HashRouter` に固定され、primary section は `#/timeline`, `#/channels`, `#/live`, `#/game`, `#/profile` を使う。route search param は `topic`, `timelineScope`, `composeTarget`, `context`, `threadId`, `authorPubkey`, `settings` に固定した。
- `App({ api })` の外部 surface は維持しつつ、内部では app-instance-local な zustand store provider と router bootstrap を持つ形へ縮退した。`DesktopApi` / Tauri invoke / domain contract は変更していない。
- invalid route param は crash ではなく最寄りの妥当 state へ fallback し、hash を `replace` で正規化する。malformed `authorPubkey` と空 thread route は context close に倒す。
- `legacy-shell.css` は削除し、旧 shell class 群は `.shell-phase1` namespaced stylesheet (`apps/desktop/src/styles/shell-phase1-legacy.css`) へ移した。Storybook preview も `.shell-phase1` + router decorator 前提へ更新した。
- required Storybook surface として `ShellTopBar`, `ShellNavRail`, `ContextPane`, `SettingsDrawer`, `TopicNavList`, `TimelineFeed`, `ThreadPanel`, `AuthorDetailCard`, `PostMedia` の direct story を追加した。
- frontend gate として `cargo xtask desktop-ui-check` を追加し、`lint`, `typecheck`, `test`, `storybook:build`, `test:e2e:browser` を束ねた。GitHub Actions の `Kukuri Fast` / `Kukuri Nightly` Linux lane には Playwright Chromium install と `desktop-ui-check` を追加した。
- local validation は `cargo xtask desktop-ui-check`, `cargo xtask check`, `cargo xtask e2e-smoke` が pass。`cargo xtask test` は `kukuri_docs_sync::tests::public_replica_syncs_over_custom_relay_seed_peers` が 1 回 timeout で flaky failure を返したが、単体 rerun (`cargo test -p kukuri-docs-sync public_replica_syncs_over_custom_relay_seed_peers -- --nocapture`) は pass した。manual smoke (`tauri:dev` on Linux, packaged Windows app) は未実施。
- accepted UI review record は [`../ui-reviews/2026-03-25-desktop-phase5-shell-cutover-cleanup.md`](../ui-reviews/2026-03-25-desktop-phase5-shell-cutover-cleanup.md) を正本とする。

## 2026-03-26 Validation Update
- `cargo xtask desktop-ui-check`, `cargo xtask check`, `cargo xtask test`, `cargo xtask e2e-smoke` は 2026-03-26 の local rerun で pass。`cargo xtask check` で再現した `tests::friend_plus_channel_restore_accepts_fresh_share_after_restart` の mutual author timeout は、desktop-runtime regression test の set-up を `A <-> B mutual` 確立後に `C` を導入する順序へ揃えて修正した。
- `cargo test -p kukuri-desktop-runtime friend_plus_channel_restore_accepts_fresh_share_after_restart -- --nocapture` は fix 後に 2 回連続 pass。`kukuri_docs_sync::tests::public_replica_syncs_over_custom_relay_seed_peers` の timeout は今回の rerun では再現しなかった。
- Linux 側は `cd apps/desktop && npx pnpm@10.16.1 tauri:dev` で `Vite` ready と Tauri `DevCommand` 起動までは確認したが、headless WSL session のため interactive manual smoke は未実施。
- Windows 側は `cmd.exe /c "cd /d C:\\Users\\kgm11\\kukuri && cargo xtask desktop-package"` が pass し、`apps/desktop/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/kukuri_0.1.0_x64-setup.exe` を生成した。packaged app の install / interactive manual smoke は未実施。
- 残件は `Windows packaged app の install/manual smoke` と `human review / PR attach / PR identifier` の review 事務のみ。

## 2026-03-26 Desktop Shell IA v2 Update
- shell IA は `left rail + main workspace + right detail pane stack` へ更新した。中央 lane は `workspace tabs / workspace input surface / workspace synced item list` の 3 区画のみを描画し、`Timeline / Channels / Live / Game / Profile` の同時表示は廃止した。
- top bar は active topic の 1 行表示のみに縮小し、headline、desktop nav trigger、status badge、settings trigger は除去した。mobile nav trigger は footer-only に移し、desktop では footer 自体を描画しない。
- left rail 上部へ `接続状況 badge 群 + Settings button` を移し、workspace switch は中央 tab header に集約した。settings button は rail header の primary action として icon size を拡大した。
- route contract は `#/timeline`, `#/channels`, `#/live`, `#/game`, `#/profile` を維持しつつ、search param は `topic`, `timelineScope`, `composeTarget`, `context`, `threadId`, `authorPubkey`, `profileMode`, `settings` に更新した。`profileMode=edit` は `#/profile` でのみ有効とし、profile default は overview に固定した。
- context route は `none`, `thread`, `author`, `thread -> author` の 4 組み合わせに正規化した。invalid `threadId` は detail stack 全体を閉じ、invalid nested `authorPubkey` は thread を維持したまま author pane だけを閉じる。
- right sidebar は廃止し、desktop 幅では thread pane と author pane を必要時だけ右へ積む stack に置き換えた。detail pane 群は translucent panel をやめ、solid surface token を使う。
- profile overview は topic-first 原則に合わせて `active topic の public timeline` を独立に読み出し、そのうち local author 投稿だけを self timeline として表示する。main timeline が private channel / all-joined scope にあっても profile list は public active topic 基準を維持する。
- accepted UI review record は [`../ui-reviews/2026-03-26-desktop-shell-ia-v2.md`](../ui-reviews/2026-03-26-desktop-shell-ia-v2.md) を正本とする。

## Purpose / Non-goals

### Purpose
- 現行 shell UI の構造的な制約を前提に、本番向け UI への移行順序、責務分離、review artifact、validation gate を実装前に固定する。
- 実装者が UI 置換と frontend infra 導入を同時進行で迷わないよう、phase ごとの entry criteria / deliverables / validation gates / not in phase を定義する。
- product UI と diagnostics UI を分離し、`ADR 0014` と `docs/DESIGN.md` に整合する reviewable な移行パスを用意する。

### Non-goals
- この文書自体で新しい visual spec、token 値、layout pixel、component API を確定しない。これらは各 phase の Figma design artifact と implementation review で決める。
- backend、runtime、Tauri invoke surface、data contract、domain model の変更計画を持ち込まない。
- `legacy/` からの wholesale 移植や、全面 rewrite を 1 PR で完了する前提を置かない。

## Current Snapshot
- この section は planning 開始時点 (`2026-03-24`) の snapshot であり、最新状態は上の `2026-03-25 Status Update` を優先する。
- 現行 frontend は `apps/desktop/src/App.tsx` と `apps/desktop/src/styles.css` 中心の monolithic shell であり、state、layout、product flow、diagnostics flow が 1 surface に集約されている。
- `App.tsx` は約 3,100 行、`App.test.tsx` は約 2,100 行で、timeline、thread、composer、profile、discovery、community-node、private channel、live、game が同居している。
- style は shared token layer ではなく hard-coded CSS を中心に構成されている。
- `apps/desktop/package.json` には Tailwind、shadcn/ui、Storybook が未導入で、frontend toolchain は `React + Vite + Vitest` の最小構成に留まっている。
- `apps/desktop/tests/e2e-smoke/` は空であり、Playwright の実運用 surface は未整備である。
- 現在の主要 regression layer は `cd apps/desktop && npx pnpm@10.16.1 test` による Vitest であり、workspace 全体の integration gate は `cargo xtask check`, `cargo xtask test`, `cargo xtask e2e-smoke` が担っている。
- current scope には Windows desktop support、seeded DHT discovery、community-node connectivity/auth、social graph v1、private channel audience v1 が含まれており、UI 移行計画もこの全 shell を対象にする。

## Applicable Rules
- workflow の正本は `docs/adr/0014-uiux-dev-flow.md` とし、Codex-assisted UI proposal では Figma を primary review artifact にする。
- Codex-assisted UI proposal の既定 path は `local HTML / React draft -> generate_figma_design による HTML capture -> human review in Figma` とする。
- design-system / review / exception policy の正本は `docs/DESIGN.md` とする。
- merge 済みで user-facing behavior または design rule を変えた UI 変更は `docs/ui-reviews/` に record を残す。
- 新規 UI と大きく触る既存 UI の標準 stack は Tailwind + shadcn/ui + Storybook とし、Playwright は top-level flow 変更または component 境界をまたぐ高リスク変更で required にする。
- product UI と diagnostics UI は設計と review の両方で分離する。observability surface を primary product flow として扱わない。

## Target Information Architecture

### Primary Product Surfaces
- app frame / navigation: topic selection、active context、global entry point、responsive shell frame
- content workspace: timeline、thread、composer、attachment preview、media display
- relationship surface: author detail、follow state、mutual / friend-of-friend affordance
- extended product modules: private channel、profile、live session、game room

### Secondary Diagnostics / Settings Surfaces
- sync status
- discovery config / diagnostics
- community-node config / auth / consent / diagnostics
- raw peer / error / observability detail

### Boundary Rules
- primary product surfaces は、投稿、返信、閲覧、topic 移動、channel 操作、profile / live / game など user goal に直結する flow を優先して構成する。
- secondary diagnostics / settings surfaces は、main workspace を圧迫しない panel、drawer、settings page、subsection へ再配置する。
- diagnostics 情報は削除しないが、primary CTA や primary reading order を占有しない配置へ移す。
- shell layout boundary、token taxonomy、primitive/component layers、test surface は frontend 側の公開ルールとしてこの計画で固定する。

## Phase Plan

### Phase 0: frontend infra foundation
- Entry criteria
  - 現行 shell は現役のまま維持する。
  - `ADR 0014` と `docs/DESIGN.md` を前提に、導入する標準 stack の責務を明文化する。
- Deliverables
  - Tailwind 導入
  - shared token layer の初期導入
  - shadcn/ui base primitive 導入方針と最小 base component
  - Storybook 導入
  - Playwright skeleton と shared test helper 導入
  - frontend directory / layer 境界の初期整理
- Validation gates
  - frontend build と Vitest が既存 shell を壊さず green を維持する
  - Storybook が起動し、最低限の primitive review surface を持つ
  - 導入手順と例外が PR に明記される
- Required review artifacts
  - Figma link
  - PR-visible preview image または short video
  - user flow summary
  - Shneiderman checklist
- Not in phase
  - 既存 shell layout の本格 rewrite
  - domain flow の再設計

### Phase 1: shell frame and information architecture
- Entry criteria
  - Phase 0 の frontend infra が安定し、token / primitive / Storybook の review 面が使える
  - 現行 shell の primary / secondary surface の分類が完了している
- Deliverables
  - shell frame、navigation、page regions の定義
  - responsive rule と resize behavior の定義
  - keyboard path と focus order の定義
  - product workspace と settings / diagnostics surface の分離
  - reusable shell primitives と layout story
- Validation gates
  - narrow desktop width でも unreadable overflow を起こさない
  - keyboard だけで primary navigation と main action に到達できる
  - shell frame proposal が Figma review を通過する
- Required review artifacts
  - Figma link
  - PR-visible preview image または short video
  - user flow summary
  - Shneiderman checklist
  - reusable component story
- Not in phase
  - timeline / thread / composer の full cutover
  - diagnostics detail の最終再配置

### Phase 2: core product flow migration
- Entry criteria
  - shell frame と layout boundary が確定している
  - token と primitive が timeline 系 surface を支えられる
- Deliverables
  - topic switching UI
  - timeline UI
  - thread UI
  - composer UI
  - attachment preview / media display UI
  - author / social affordance UI
  - reusable product component と story
- Validation gates
  - must-preserve flow の `publish -> render`, `reply`, `topic switch`, `media attach preview` が維持される
  - Vitest で core flow regression を維持する
  - top-level flow 変更が大きい場合は Playwright scenario を追加する
- Required review artifacts
  - Figma link
  - PR-visible preview image または short video
  - user flow summary
  - Shneiderman checklist
  - accepted UI review record が必要な変更では `docs/ui-reviews/` を追加
- Not in phase
  - private channel / profile / live / game の全面移行
  - discovery / community-node diagnostics の最終整理

### Phase 3: extended product flow migration
- Entry criteria
  - core product flow が新しい token / component system 上で成立している
  - private channel、profile、live、game の state handling 要件が洗い出されている
- Deliverables
  - private channel flow
  - profile editor flow
  - live session flow
  - game room flow
  - extended module 用 reusable component / story
- Validation gates
  - must-preserve flow の `private channel create/join/grant/share/freeze/rotate`, `profile edit`, `live/game flow` が維持される
  - state ごとの loading / empty / error / success が設計される
  - 既存 contract を変更しない
- Required review artifacts
  - Figma link
  - PR-visible preview image または short video
  - user flow summary
  - Shneiderman checklist
  - accepted UI review record が必要な変更では `docs/ui-reviews/` を追加
- Not in phase
  - sync / discovery / community-node の diagnostics 主導 redesign
  - backend / API contract change

### Phase 4: diagnostics and settings migration
- Entry criteria
  - primary product workspace が新 shell 上で安定している
  - secondary surface に移す observability 情報の最小セットが合意されている
- Deliverables
  - sync status surface の再配置
  - discovery settings / diagnostics の再配置
  - community-node config / auth / consent / diagnostics の再配置
  - peer / error / observability detail の整理
  - settings / drawer / panel / subsection の UI pattern 固定
- Validation gates
  - must-preserve flow の `discovery/community-node config`, `diagnostics/error feedback` が維持される
  - product flow の reading order を diagnostics が阻害しない
  - keyboard / resize / error visibility rule を満たす
- Required review artifacts
  - Figma link
  - PR-visible preview image または short video
  - user flow summary
  - Shneiderman checklist
  - accepted UI review record が必要な変更では `docs/ui-reviews/` を追加
- Not in phase
  - 旧 shell の全面削除
  - domain capability の拡張

### Phase 5: cutover and cleanup
- Entry criteria
  - primary / extended / diagnostics surface の staged migration が完了している
  - regression layers が新 shell に追従している
- Deliverables
  - 旧 shell 依存の削除
  - Storybook / Vitest / 必要な Playwright の整備完了
  - accepted UI review record の追加
  - 本番向け UI を既定 path に切り替える cleanup
- Validation gates
  - `cd apps/desktop && npx pnpm@10.16.1 test`
  - `cargo xtask check`
  - `cargo xtask test`
  - `cargo xtask e2e-smoke`
  - required Storybook surface と Playwright surface が揃っている
- Required review artifacts
  - Figma link
  - PR-visible preview image または short video
  - user flow summary
  - Shneiderman checklist
  - `docs/ui-reviews/` accepted record
- Not in phase
  - 追加 product feature の持ち込み
  - backend / protocol change を伴う拡張

## Validation Matrix
- Baseline frontend regression layer は Vitest を維持する。
- reusable component と大きく改修した reusable component には Storybook story を付ける。
- top-level flow を変える変更、または複数 component をまたぐ高リスク変更では Playwright を required にする。
- workspace integration gate は `cargo xtask check`, `cargo xtask test`, `cargo xtask e2e-smoke` を維持し、frontend-only tooling で置き換えない。
- must-preserve flow は最低限次を含む。
  - publish / reply
  - topic switch
  - media attach preview
  - private channel create / join / grant / share / freeze / rotate
  - profile edit
  - live / game flow
  - discovery / community-node config
  - diagnostics / error feedback

## Risks / Dependencies
- Tailwind、shadcn/ui、Storybook、Playwright の導入は現行 Vite / Vitest / Tauri 開発体験を壊さない形で段階投入する必要がある。
- 現行 `App.tsx` に集中している state と UI の責務分離を誤ると、見た目だけ先に変わって regression が増える。
- diagnostics を後景へ移す際に observability を失うと、current scope の connectivity / auth / audience troubleshooting が困難になる。
- Windows / Linux 両方で resize、focus、packaged app behavior を崩さない前提で進める必要がある。
- UI proposal workflow では Figma link と PR-visible preview が required になるため、各 phase の PR は review artifact を先に揃える必要がある。
- Figma link の既定値は HTML capture で生成した Figma design URL とし、FigJam は補助 artifact として扱う。

## Exit Criteria
- 現行 shell の primary / secondary surface が新しい shell boundary に載り替わっている。
- hard-coded one-off style 依存が shared token / primitive / component layer へ置き換わっている。
- Storybook、Vitest、必要な Playwright、`cargo xtask` gate が新 UI を守る状態になっている。
- `ADR 0014` と `docs/DESIGN.md` に沿った review artifact と validation note が各主要 phase の PR に揃っている。
- user-facing behavior または reusable design rule を変えた採用済み UI 変更について、必要な `docs/ui-reviews/` record が残っている。

## Notes
- この planning-doc PR 自体では Figma design artifact や UI review record を新規作成しない。これらは implementation phase の成果物として扱う。
- 本文書は roadmap / sequencing の正本であり、実装時の具体的な token 値、component prop、layout detail は各 phase の proposal で確定する。
