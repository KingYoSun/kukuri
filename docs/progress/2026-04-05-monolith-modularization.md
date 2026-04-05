# 2026-04-05 monolith modularization

## Summary
- この文書は、current kukuri workspace にある長大な entry file を段階的に facade 化する multi-PR modularization campaign の正本である。
- 対象は root workspace の現行実装のみとし、`legacy/` は参照専用で campaign scope から除外する。
- execution order は `edge -> core` に固定し、PR は subsystem wave 単位で切る。repo 全体を横断する mechanical split-only PR は前提にしない。
- この文書は TODO メモではなく、`対象 inventory + 目標モジュール境界 + wave 順序 + validation gate + status update rule` を先に固定する設計書兼 progress tracker とする。
- broad cleanup は許容するが、new feature 追加は scope 外とする。
- public contract は原則維持し、serialized shape、Tauri command 名、route/search param、cross-crate public API に変更が入る場合だけこの文書へ明示追記する。

## Current Snapshot

| path | current LOC | primary responsibilities | risk | target modules | wave | status |
| --- | ---: | --- | --- | --- | --- | --- |
| `apps/desktop/src/App.tsx` | 7166 | shell bootstrap、route normalization、zustand store、UI orchestration、media helper | 高 | `shell/store.ts`, `shell/routes.ts`, `shell/selectors.ts`, `shell/media.ts`, `shell/DesktopShellPage.tsx` | 1 | planned |
| `apps/desktop/src/App.test.tsx` | 3252 | shell integration regression、media helper、theme/routing regression | 中 | thin `App` smoke + route/store/media/orchestration test split | 1 | planned |
| `apps/desktop/src-tauri/src/lib.rs` | 1017 | Tauri entrypoint、command 実装、state、tracing | 高 | `commands/{posts,reactions,profile,direct_messages,live_game,community_node}.rs`, `tracing.rs`, `state.rs` | 1 | planned |
| `crates/app-api/src/lib.rs` | 17181 | DTO/view、`AppService`、social/timeline/direct message/reaction/notification/live/game/private channel/sync/media、tests | 非常に高 | `views.rs`, `service.rs`, `social.rs`, `timeline.rs`, `direct_messages.rs`, `reactions.rs`, `notifications.rs`, `live.rs`, `game.rs`, `private_channels.rs`, `sync.rs`, `media.rs`, `tests/*` | 2 | planned |
| `crates/desktop-runtime/src/lib.rs` | 7859 | request DTO、runtime façade、community-node/discovery、iroh stack reload、attachment/path helper、tests | 非常に高 | `requests.rs`, `runtime.rs`, `community_node.rs`, `discovery.rs`, `stack.rs`, `attachments.rs`, `paths.rs` | 2 | planned |
| `crates/store/src/lib.rs` | 4528 | store model、traits、SQLite 実装、memory 実装、row mapping、pagination、tests | 高 | `models.rs`, `traits.rs`, `sqlite.rs`, `memory.rs`, `row_mapping.rs`, `pagination.rs`, `tests/*` | 3 | planned |
| `crates/harness/src/lib.rs` | 5163 | scenario spec、runtime bring-up、wait helper、artifact 出力、desktop/community-node/private-channel/direct-message scenario | 高 | `scenario.rs`, `runtime.rs`, `waiters.rs`, `artifacts.rs`, `scenarios/{desktop_smoke,community_node,private_channel,direct_message}.rs` | 3 | planned |
| `crates/core/src/lib.rs` | 3761 | id/value type、crypto、envelope builder/parser、posts/profile/reactions/private channel/direct message/media | 中 | `ids.rs`, `crypto.rs`, `envelope.rs`, `posts.rs`, `profile.rs`, `reactions.rs`, `private_channels.rs`, `direct_messages.rs`, `media.rs` | 4 | planned |
| `crates/transport/src/lib.rs` | 2980 | transport traits、config、fake transport、iroh transport、discovery、ticket、diagnostics | 中 | `traits.rs`, `config.rs`, `fake.rs`, `iroh.rs`, `discovery.rs`, `tickets.rs`, `diagnostics.rs` | 4 | planned |

### Watchlist

| path | current LOC | primary responsibilities | reason to watch | expected wave | status |
| --- | ---: | --- | --- | --- | --- |
| `crates/docs-sync/src/lib.rs` | 1342 | `DocsSync` trait、iroh/memory 実装、replica id helper、private replica access | wave 2-4 で周辺 crate の split 後に implementation root が残る可能性がある | 5 | planned |
| `crates/cn-core/src/lib.rs` | 1195 | community-node auth/bootstrap/policy shared type、DB init、URL/auth helper | community-node path の再編後に shared utility の再集中が残る可能性がある | 5 | planned |

## Refactor Rules
- PR は subsystem wave 単位で切る。repo-wide mechanical split は行わない。
- broad cleanup は許容するが、対象 wave 内に閉じる。許容範囲は命名整理、dead code 除去、visibility 整理、helper 統合までとする。
- user-facing behavior、serialized field、Tauri command 名、route/search param、cross-crate public API を黙って変えない。
- `apps/desktop/src/App.tsx` は引き続き `App` を export し、`apps/desktop/src-tauri/src/lib.rs` は Tauri entrypoint の役割を維持する。
- Rust crate root は facade として残し、既存 public symbol は `pub use` で維持する前提とする。
- 不具合修正が必要になった場合は、先に failing test / contract / scenario で再現してから同じ PR で直す。
- 本体分割と companion tests の移動・追加は同じ PR に含める。tests-only 後追い PR は作らない。
- すべての PR でこの progress 文書を更新し、wave status と log を進める。

## Target Module Map

### `apps/desktop/src/App.tsx`
- `App.tsx` は thin bootstrap に縮め、theme persistence と provider/router wiring だけを残す。
- shell state 定義と store 初期化は `shell/store.ts` へ移す。
- route parsing / normalization / section mapping は `shell/routes.ts` へ移す。
- view assembly に必要な selector / label helper は `shell/selectors.ts` へ移す。
- media helper と poster/object URL utility は `shell/media.ts` へ移す。
- 現在の巨大 component body は `shell/DesktopShellPage.tsx` へ移し、`App.tsx` からは呼び出しだけにする。

### `apps/desktop/src/App.test.tsx`
- thin `App` integration smoke は残す。
- routing/theme/bootstrap regression は `App` 近傍 test として残し、store/media/orchestration regression は対応 module 近傍へ分離する。
- `App.test.tsx` 自体を bootstrap smoke 中心に縮め、helper-heavy な test data builder は共通 fixture 化を許容する。

### `apps/desktop/src-tauri/src/lib.rs`
- `lib.rs` は Tauri entrypoint、state wire-up、command registration のみを持つ facade にする。
- command 実装は feature 群ごとに `commands/posts.rs`, `commands/reactions.rs`, `commands/profile.rs`, `commands/direct_messages.rs`, `commands/live_game.rs`, `commands/community_node.rs` へ分ける。
- tracing 初期化と directive helper は `tracing.rs`、`DesktopState` は `state.rs` に分ける。

### `crates/desktop-runtime/src/lib.rs`
- request DTO は `requests.rs` へ抽出する。
- `DesktopRuntime` façade と public method surface は `runtime.rs` に集約する。
- community-node auth/consent/config/metadata は `community_node.rs` に分ける。
- discovery config/env/seed parsing は `discovery.rs` に分ける。
- shared iroh stack、reloadable transport/docs/blob の構築は `stack.rs` に分ける。
- attachment normalize/crop helper は `attachments.rs`、db/config path helper は `paths.rs` に分ける。
- 既存 `identity.rs` はそのまま維持する。

### `crates/app-api/src/lib.rs`
- DTO/view 群は `views.rs` に集約する。
- `AppService` façade と constructor は `service.rs` に置く。
- feature 本体は `social.rs`, `timeline.rs`, `direct_messages.rs`, `reactions.rs`, `notifications.rs`, `live.rs`, `game.rs`, `private_channels.rs`, `sync.rs`, `media.rs` に分ける。
- 大型 helper は feature module に寄せ、shared helper が必要なら feature 横断 util を最小限だけ導入する。
- tests は `tests/*` に分け、feature と同じ責務境界に追従させる。

### `crates/store/src/lib.rs`
- projection row / cursor / page などの data model は `models.rs` に分ける。
- `Store` / `ProjectionStore` trait は `traits.rs` に分ける。
- SQLite 実装は `sqlite.rs`、memory 実装は `memory.rs` に分ける。
- SQL row conversion helper は `row_mapping.rs`、cursor/page helper は `pagination.rs` に分ける。
- tests は backend ごとに `tests/*` へ分ける。

### `crates/harness/src/lib.rs`
- scenario DSL と spec 型は `scenario.rs` に分ける。
- runtime bring-up / shutdown / fixture helper は `runtime.rs` に分ける。
- polling / assert helper は `waiters.rs` に分ける。
- result serialization / artifact 出力は `artifacts.rs` に分ける。
- scenario runner 本体は `scenarios/desktop_smoke.rs`, `scenarios/community_node.rs`, `scenarios/private_channel.rs`, `scenarios/direct_message.rs` に分ける。

### `crates/core/src/lib.rs`
- value object と id 型は `ids.rs` に分ける。
- key/sign/encrypt/decrypt helper は `crypto.rs` に分ける。
- common envelope type と sign/parse 基盤は `envelope.rs` に分ける。
- post/profile/reaction/private channel/direct message/media はそれぞれ domain module へ分ける。
- crate root は `pub use` に徹し、domain contract の入口だけを保つ。

### `crates/transport/src/lib.rs`
- trait と共通 type は `traits.rs` に分ける。
- network/discovery/relay config は `config.rs` に分ける。
- fake transport は `fake.rs`、iroh transport 実装は `iroh.rs` に分ける。
- DHT/static-peer discovery helper は `discovery.rs` に分ける。
- ticket encode/parse は `tickets.rs`、status/detail 生成は `diagnostics.rs` に分ける。

## Wave Plan

### Wave 1
- status: planned
- goal: UI edge と Tauri edge を先に薄くし、`App.tsx` と `src-tauri/lib.rs` を facade 化する。
- included files:
  - `apps/desktop/src/App.tsx`
  - `apps/desktop/src/App.test.tsx`
  - `apps/desktop/src-tauri/src/lib.rs`
- PR slices:
  - shell store/routes/selectors/page 抽出と `App.tsx` bootstrap 化
  - media helper 抽出と `App.test.tsx` の bootstrap smoke / media / routing test 分離
  - Tauri `commands/*`, `tracing.rs`, `state.rs` 抽出と entrypoint 縮退
- exit criteria:
  - `App.tsx` が theme/provider/router bootstrap にほぼ限定される
  - `App.test.tsx` が app bootstrap smoke 中心になり、module-level test が分離される
  - `apps/desktop/src-tauri/src/lib.rs` が Tauri entrypoint と command registration 中心になる
- blocked by:
  - なし。campaign の先頭 wave とする

### Wave 2
- status: planned
- goal: runtime と app service の feature 本体を分離し、UI edge から下の orchestration root を薄くする。
- included files:
  - `crates/desktop-runtime/src/lib.rs`
  - `crates/app-api/src/lib.rs`
- PR slices:
  - `desktop-runtime` request DTO / path helper / attachment helper 抽出
  - `desktop-runtime` discovery/community-node/stack split と `runtime.rs` façade 化
  - `app-api` views/service split
  - `app-api` feature module split と `tests/*` 分離
- exit criteria:
  - `DesktopRuntime` public method surface は維持したまま implementation が module へ退避している
  - `AppService` public method surface は維持したまま feature ごとに分割されている
  - tests が feature module 単位に追従している
- blocked by:
  - Wave 1 で UI/Tauri edge の module 境界が先に固定されていること

### Wave 3
- status: planned
- goal: persistence と scenario harness の巨大 root を分割し、app-api/runtime split 後の依存先を整える。
- included files:
  - `crates/store/src/lib.rs`
  - `crates/harness/src/lib.rs`
- PR slices:
  - `store` models/traits/pagination/row mapping split
  - `store` SQLite / memory backend split と tests 再配置
  - `harness` scenario/runtime/waiters/artifacts split
  - `harness` scenario runner を scenario module へ分割
- exit criteria:
  - `store` root が model/trait 再 export に近づく
  - `harness` root が scenario dispatch と public entrypoint に近づく
  - scenario/wait helper の責務境界がテストと同じ単位で見える
- blocked by:
  - Wave 2 で runtime/app-api の feature 境界が安定していること

### Wave 4
- status: planned
- goal: domain core と transport core を最後に薄くし、repo-wide module map を完成させる。
- included files:
  - `crates/core/src/lib.rs`
  - `crates/transport/src/lib.rs`
- PR slices:
  - `core` ids/crypto/envelope split
  - `core` posts/profile/reactions/private_channels/direct_messages/media split
  - `transport` traits/config/tickets/discovery split
  - `transport` fake/iroh/diagnostics split
- exit criteria:
  - `core` root が domain entrypoint と `pub use` 中心になる
  - `transport` root が public trait/config export 中心になる
  - downstream crate の import 面が安定する
- blocked by:
  - Wave 2-3 で downstream caller 側の整理が済んでいること

### Wave 5
- status: planned
- goal: watchlist と residual implementation root を処理し、campaign を閉じる。
- included files:
  - `crates/docs-sync/src/lib.rs`
  - `crates/cn-core/src/lib.rs`
  - 前 wave 後も implementation root が残る file
- PR slices:
  - watchlist file の要否判定と必要時の module split
  - facade root に残った一時 helper / `pub use` / transitional shim の cleanup
  - final validation と progress closeout
- exit criteria:
  - watchlist を含め implementation root が campaign 許容範囲まで解消している
  - final validation matrix が一巡している
  - 文書上の status が landed/blocked へ収束している
- blocked by:
  - Wave 1-4 の完了

## Validation Matrix

| wave | required validation |
| --- | --- |
| 1 | `cargo xtask desktop-ui-check`, `cargo xtask check`, `cargo xtask e2e-smoke` |
| 2 | `cargo test -p kukuri-app-api`, `cargo test -p kukuri-desktop-runtime`, `cargo xtask check`, `cargo xtask test` |
| 3 | `cargo test -p kukuri-store`, `cargo test -p kukuri-harness`, `cargo xtask e2e-smoke` |
| 4 | `cargo test -p kukuri-core`, `cargo test -p kukuri-transport`, `cargo xtask check`, `cargo xtask test` |
| 5 | watchlist 対象 crate test + `cargo xtask check` + `cargo xtask test` + 必要な scenario rerun |

- community-node / discovery / private-channel に触る wave は、`docs/progress/2026-03-10-foundation.md` で gate 化済みの contract / scenario も rerun 対象に含める。
- frontend regression では shell bootstrap、routing、theme persistence、media helper、`App` integration smoke を最低限維持する。
- runtime / app-api regression では social graph、private channel、direct message、live、game、discovery、community-node、blob/media を最低限維持する。
- store regression では pagination、projection mapping、notification / direct message row mapping を最低限維持する。
- harness regression では scenario runner と wait helper の安定性を最低限維持する。

## Public API / Interface Guardrails
- `apps/desktop/src/App.tsx` は campaign 全体を通じて `App` export を維持する。
- `apps/desktop/src-tauri/src/lib.rs` は campaign 全体を通じて Tauri entrypoint と command registration の責務を維持する。
- Rust crate root は implementation を外へ出しても import 面を壊さないよう `pub use` を維持する前提で進める。
- DTO/view の serialized shape、Tauri command 名、route/search param contract、cross-crate public API は原則不変とする。
- 上記 contract を変える必要が出た場合は、その PR で黙って混ぜず、この文書の summary / target map / validation へ明示追記する。

## Status Updates

### Wave Status

| wave | status | note |
| --- | --- | --- |
| 1 | planned | edge shell / Tauri edge を facade 化する |
| 2 | planned | runtime / app-api の orchestration root を分割する |
| 3 | planned | store / harness の backend root を分割する |
| 4 | planned | core / transport の domain root を分割する |
| 5 | planned | watchlist と residual root を閉じる |

### 2026-04-05 Initial Plan Lock
- PR: N/A
- files moved: none
- root LOC before/after: planning only, no file split yet
- validation run: none
- follow-ups:
  - Wave 1 から着手し、`App.tsx`, `App.test.tsx`, `apps/desktop/src-tauri/src/lib.rs` の facade 化を進める
  - 各 PR で wave status、moved files、root LOC before/after、validation run を追記する
  - campaign の追加 scope は新しい monolith を見つけた場合でも、まず watchlist へ入り、既存 wave を崩さない

## Exit Criteria
- `App.tsx` と各 crate root `lib.rs` が implementation body ではなく facade になっている。
- companion tests が本体と同じ責務境界に追従して分割されている。
- watchlist を含む entry file の責務が campaign で定義した module map に収束している。
- 同じ巨大 file への再集中を防ぐ import/export boundary が定着している。
