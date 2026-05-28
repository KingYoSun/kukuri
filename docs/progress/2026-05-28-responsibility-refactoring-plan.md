# 2026-05-28 責務別リファクタリング計画

## 目的

この文書は `docs/progress/2026-04-05-monolith-modularization.md` の次段にあたるリファクタリング計画です。

前回の campaign では root file と crate root を thin facade 化しました。本計画ではその作業をやり直さず、すでに分割済みの現行実装を責務ごとに再確認し、境界のにじみ・大型テスト・大型 helper を小さな wave で整理します。

## 対象範囲

対象は root workspace の現行実装のみです。

- `crates/core`
- `crates/store`
- `crates/transport`
- `crates/docs-sync`
- `crates/blob-service`
- `crates/app-api`
- `crates/desktop-runtime`
- `crates/cn-*`
- `crates/harness`
- `apps/desktop`
- `apps/desktop/src-tauri`

この計画はリファクタリング専用です。機能追加、protocol 変更、storage schema 変更、依存更新、product behavior 変更、UI redesign は含めません。

## リファクタリングモード

この計画に従う実装作業は、必ず `REFACTORING.md` に従います。

- 1 PR = 1 intent。
- rename、move、extract、behavior change、dependency update、storage migration を同じ PR に混ぜない。
- 明示的な non-refactor task がない限り、public API、protocol object、serialized DTO、Tauri command 名、storage semantics、docs/blobs canonical source、community-node endpoint contract は変更しない。
- 振る舞いの仕様が不足している場合は、構造変更の前に characterization test、contract、scenario を追加する。
- 必須 validation は `REFACTORING.md` の path-based matrix に従う。

## 現状スナップショット

現行実装は top-level ではすでに modularized されています。次の圧力点は crate root ではなく、責務が集まりすぎた module と大型 test file です。

| area | 観測した圧力 | 現在の anchor path |
| --- | --- | --- |
| Desktop runtime tests | integration-style test module が大きく、runtime capability ごとの regression を切り分けにくい。 | `crates/desktop-runtime/src/tests/mod.rs` |
| Desktop shell tests | shell integration test が大きく、複数 workflow が同じ fixture surface を共有している。 | `apps/desktop/src/shell/DesktopShellPage.test.tsx` |
| App API sync tests | sync behavior tests が durable source / recovery concern を複数まとめて扱っている。 | `crates/app-api/src/tests/sync.rs` |
| Desktop mocks | mock API surface が大きく、command-domain boundary が見えにくい。 | `apps/desktop/src/mocks/desktopApiMock.ts` |
| Harness waiters | wait/assert helper が広く、scenario と結合している。 | `crates/harness/src/waiters.rs` |
| Community-node scenarios | scenario implementation が stack lifecycle、auth/consent、behavior assertion を混ぜている。 | `crates/harness/src/scenarios/community_node.rs` |
| Desktop shell runtime hooks | data/action/view-model hook が feature responsibility の境界で膨らみやすい。 | `apps/desktop/src/shell/useDesktopShellData.ts`, `apps/desktop/src/shell/useDesktopShellActions.ts`, `apps/desktop/src/shell/useDesktopShellViewModels.ts` |
| App API private channel support | domain API と support helper が oversized threshold 付近にある。 | `crates/app-api/src/private_channels.rs`, `crates/app-api/src/service/private_channels_support.rs` |
| Metaverse UI surface | 新しい UI 領域が大きく、3D/view/state/event の責務を分けておく必要がある。 | `apps/desktop/src/components/extended/MetaverseRoomPanel.tsx` |

## 目標責務マップ

### Domain contracts

責務:

- protocol-native value object
- envelope
- signing / encryption helper
- domain object construction

対象 path:

- `crates/core/src/*`

ガードレール:

- persistence policy を持たない。
- transport orchestration を持たない。
- stable domain type を超える desktop/UI DTO shaping を持たない。
- ここでの refactor は原則 `refactor:extract` または `refactor:rename`。behavior-affecting change はこの計画の対象外。

### Persistence and projections

責務:

- SQLite / memory storage
- migration
- row mapping
- pagination
- local projection

対象 path:

- `crates/store/src/*`
- `crates/store/migrations/*`

ガードレール:

- protocol shape decision を持たない。
- network recovery policy を持たない。
- storage schema change は migration であり、refactoring ではない。
- projection helper extraction は row semantics と cursor behavior を維持する。

### Durable sync and blob plane

責務:

- docs replica access
- docs/blobs source-of-truth mechanics
- private replica access control
- blob payload handling

対象 path:

- `crates/docs-sync/src/*`
- `crates/blob-service/src/*`

ガードレール:

- hint は notification / sync trigger のままで、canonical state にしない。
- docs/blobs canonical source behavior を変えない。
- private replica capability requirement は test で維持する。

### Transport and discovery

責務:

- transport trait
- fake transport
- iroh integration
- ticket parsing
- diagnostics
- static-peer
- seeded DHT
- relay configuration

対象 path:

- `crates/transport/src/*`

ガードレール:

- app-level sync policy を持たない。
- community-node HTTP API contract を変えない。
- ticket format と discovery config semantics を維持する。

### App service orchestration

責務:

- core/store/docs/blob/transport をまたぐ application use case
- timeline
- media
- social graph
- private channels
- direct messages
- reactions
- notifications
- live/game
- sync
- view DTO

対象 path:

- `crates/app-api/src/*`

ガードレール:

- App API DTO shape は別の contract task がない限り維持する。
- feature module は可能な限り自分の helper を所有する。
- cross-feature helper は、実際の重複削減になり、隠れた feature dependency を作らない場合だけ許容する。

### Desktop runtime

責務:

- desktop process runtime
- identity
- local paths
- shared stack construction
- discovery config
- community-node session / token handling
- attachment normalization
- Rust command-facing API

対象 path:

- `crates/desktop-runtime/src/*`

ガードレール:

- runtime は service を orchestration し、app-service domain behavior を複製しない。
- community-node token/session persistence は `community_node/*` 配下に隔離する。
- public request/status type は別の contract task がない限り維持する。

### Tauri edge

責務:

- Tauri setup
- state wiring
- tracing
- command adapter

対象 path:

- `apps/desktop/src-tauri/src/*`

ガードレール:

- command name と payload shape を維持する。
- command file は `kukuri-desktop-runtime` への薄い adapter に保つ。
- business logic を Tauri edge に移さない。

### Desktop frontend

責務:

- shell routing
- local UI state
- API invocation adapter
- visual component
- mock
- story
- browser-level user flow

対象 path:

- `apps/desktop/src/*`

ガードレール:

- component は、明示的に shell/container module として命名されていない限り presentational に保つ。
- shell hook は orchestration を持ち、component は rendering と local interaction を持つ。
- API command name と DTO contract を維持する。
- UI refactor と product flow redesign を同じ PR に混ぜない。

### Community-node server slice

責務:

- community-node shared auth/bootstrap/config/database logic
- user API
- relay server
- CLI operation

対象 path:

- `crates/cn-core/src/*`
- `crates/cn-user-api/src/*`
- `crates/cn-iroh-relay/src/*`
- `crates/cn-cli/src/*`

ガードレール:

- HTTP endpoint contract を維持する。
- database schema change は migration であり、refactoring ではない。
- relay / auth / control-plane boundary を明示的に保つ。

### Harness and scenarios

責務:

- executable behavior scenario
- runtime fixture
- waiter
- artifact
- scenario dispatch

対象 path:

- `crates/harness/src/*`
- `harness/scenarios/*`

ガードレール:

- scenario assertion は implementation detail ではなく product behavior を表す。
- waiter を抽出する場合は behavior domain ごとにまとめる。
- scenario YAML shape は別の scenario contract task がない限り維持する。

## 提案 wave

### Wave 0: Audit lock

種別: `docs`

目的:

- この文書を responsibility-first refactoring の baseline として固定する。
- 実装 file は変更しない。

作業:

- 後続 wave に入る前に、現行 code と責務マップを照合する。
- 新しい oversized file や boundary pressure を見つけた場合は、refactor 前にこの文書へ追記する。

Validation:

- Markdown review のみ。
- code validation は不要。

完了条件:

- この文書が `docs/progress` 配下に存在する。
- 後続 wave が独立した refactoring PR として切れる状態になっている。

### Wave 1: Test responsibility split

種別: `refactor:extract`

目的:

- 大型 test module を behavior domain ごとに分割する。
- assertion の意味は変えない。

候補 path:

- `crates/desktop-runtime/src/tests/mod.rs`
- `apps/desktop/src/shell/DesktopShellPage.test.tsx`
- `crates/app-api/src/tests/sync.rs`
- `crates/app-api/src/tests/direct_messages.rs`
- `crates/app-api/src/tests/private_channels.rs`
- `crates/app-api/src/tests/timeline.rs`

PR slicing:

- Runtime tests: identity、restart restore、community-node、private-channel、media/blob、live/game。
- Desktop shell tests: routing/bootstrap、compose/timeline、private-channel、settings/connectivity、media、notifications/messages。
- App API sync tests: durable-source recovery、blob fetch、gossip loss、projection hydration。

Validation:

- Rust test split: relevant `cargo test -p <crate>`。
- Desktop test split: `cd apps/desktop && npx pnpm@10.16.1 test`。
- touched test area が広い場合、merge / closeout 前に `cargo xtask test`。

完了条件:

- 分割後の各 test file が 1 つの behavior domain を明確に持つ。
- shared fixture は元の巨大 file 名ではなく behavior 名で命名されている。
- assertion を弱めたり、代替なしに削除したりしていない。

### Wave 2: Harness waiters and scenario helpers

種別: `refactor:extract`

目的:

- generic polling mechanics と domain-specific scenario assertion を分離する。

候補 path:

- `crates/harness/src/waiters.rs`
- `crates/harness/src/scenarios/community_node.rs`
- `crates/harness/src/scenarios/private_channel.rs`
- `crates/harness/src/scenarios/direct_message.rs`

目標 boundary:

- generic wait primitives
- timeline / thread waiters
- connectivity / community-node waiters
- private-channel waiters
- direct-message waiters
- scenario stack lifecycle helpers

Validation:

- `cargo test -p kukuri-harness`
- 変更した scenario command。例: `cargo xtask scenario community_node_public_connectivity`
- private-channel behavior に触れた場合は、対応する private-channel harness test も実行する。

完了条件:

- scenario file が behavior script として読める。
- waiter module が、明示的に scenario domain 名を持つ場合を除き、scenario-specific setup knowledge を持たない。

### Wave 3: Desktop shell orchestration boundaries

種別: `refactor:extract`

目的:

- shell data loading、shell action、view-model assembly を feature responsibility ごとに分ける。

候補 path:

- `apps/desktop/src/shell/useDesktopShellData.ts`
- `apps/desktop/src/shell/useDesktopShellActions.ts`
- `apps/desktop/src/shell/useDesktopShellViewModels.ts`
- `apps/desktop/src/shell/data/*`
- `apps/desktop/src/shell/actions/*`
- `apps/desktop/src/mocks/desktopApiMock.ts`

目標 boundary:

- timeline / topic loading
- profile / social graph
- private channels
- direct messages
- reactions
- live / game / metaverse
- connectivity / settings
- command domain ごとの mock API fixture

Validation:

- `cargo xtask desktop-ui-check`
- 狭い slice では最低限 `cd apps/desktop && npx pnpm@10.16.1 test` と typecheck/lint。full UI check を実行しなかった場合は理由を報告する。

完了条件:

- hook file が orchestration facade であり、mixed feature implementation になっていない。
- 1 domain の mock setup を読むために unrelated command defaults を追う必要がない。
- component props と shell view model は別の UI contract task がない限り維持されている。

### Wave 4: App API support helper boundaries

種別: `refactor:extract`

目的:

- public app API behavior を維持したまま、app-service support module に集中した helper を整理する。

候補 path:

- `crates/app-api/src/service/private_channels_support.rs`
- `crates/app-api/src/private_channels.rs`
- `crates/app-api/src/service/timeline_runtime_support.rs`
- `crates/app-api/src/service/object_persistence_support.rs`
- `crates/app-api/src/service/direct_messages_*_support.rs`

目標 boundary:

- private-channel capability / grant / share helpers
- epoch / archive / rotation helpers
- timeline hydration / merge helpers
- object persistence / blob manifest helpers
- direct-message subscription / delivery helpers

Validation:

- `cargo test -p kukuri-app-api`
- private channels、direct messages、media/blob、social graph に触れる場合は `docs/runbooks/dev.md` にある関連 targeted tests。
- DTO payload shape に触れる場合は frontend tests も実行する。

完了条件:

- feature module が同じ public service method を公開し続ける。
- support helper が incidental call order ではなく domain operation ごとにまとまっている。
- contract または serialized shape の変更がない。

### Wave 5: Desktop runtime community-node and stack seams

種別: `refactor:extract`

目的:

- runtime stack construction、community-node session management、requests、reconnect policy を独立して読める状態にする。

候補 path:

- `crates/desktop-runtime/src/runtime/*`
- `crates/desktop-runtime/src/community_node/*`
- `crates/desktop-runtime/src/stack.rs`
- `crates/desktop-runtime/src/discovery.rs`

目標 boundary:

- runtime public API facade
- shared iroh stack construction
- discovery config / env parsing
- community-node HTTP client
- session state and token storage
- reconnect / metadata refresh

Validation:

- `cargo test -p kukuri-desktop-runtime`
- `cargo xtask e2e-smoke`
- community-node slice の場合は `cargo xtask cn-test`、`cargo xtask scenario community_node_public_connectivity`、`cargo xtask scenario community_node_multi_device_connectivity`。

完了条件:

- runtime module が app-api behavior を複製していない。
- community-node code の auth、consent、metadata、token storage、reconnect の責務が見える。
- request/status public type が維持されている。

### Wave 6: Community-node server boundary review

種別: `refactor:boundary`

目的:

- active connectivity/auth work 後の server-side boundary を再確認し、core、API、relay、CLI の accidental coupling を避ける。

候補 path:

- `crates/cn-core/src/*`
- `crates/cn-user-api/src/*`
- `crates/cn-iroh-relay/src/*`
- `crates/cn-cli/src/*`

目標 boundary:

- `cn-core`: models、config、auth、bootstrap、consents、rollout、database。
- `cn-user-api`: HTTP routing と request/response adapter。
- `cn-iroh-relay`: relay server configuration と runtime。
- `cn-cli`: operational command adapter。

Validation:

- `cargo xtask cn-check`
- `cargo xtask cn-test`
- relay code に触れる場合は `cargo test -p kukuri-cn-iroh-relay`。

完了条件:

- HTTP contract tests が endpoint behavior を表し続ける。
- database init / prepare / deploy の責務が明示されている。
- relay code が user API policy を吸収していない。

### Wave 7: Metaverse UI/component responsibility check

種別: `refactor:extract`

目的:

- 新しい metaverse UI surface が scene、room state、debug、shell orchestration を抱え込む module にならないようにする。

候補 path:

- `apps/desktop/src/components/extended/MetaverseRoomPanel.tsx`
- `apps/desktop/src/components/extended/types.ts`
- `apps/desktop/src/shell/actions/liveGame.ts`
- `apps/desktop/src/shell/page/DesktopShellPrimaryWorkspace.tsx`

目標 boundary:

- room discovery / list card
- room view container
- scene rendering component
- room debug / status panel
- room input / control panel
- shell action adapter

Validation:

- `cargo xtask desktop-ui-check`
- file extraction を超えて layout / interaction surface に触れた場合のみ browser/manual visual check。

完了条件:

- rendering、state、debug panel を独立して読める。
- game room behavior が壊れていない。
- extraction PR に redesign を混ぜていない。

## Cross-Wave Validation Baseline

各 PR では targeted validation を使います。広い refactoring campaign を merge する前には、可能な限り以下を推奨します。

```bash
cargo xtask check
cargo xtask test
cargo xtask e2e-smoke
```

community-node behavior に触れた場合:

```bash
cargo xtask cn-check
cargo xtask cn-test
cargo xtask scenario community_node_public_connectivity
cargo xtask scenario community_node_multi_device_connectivity
```

desktop frontend behavior に触れた場合:

```bash
cargo xtask desktop-ui-check
```

## 非目標

- 新機能。
- 先に failing test / contract / scenario を置かない bug fix。
- 依存更新。
- storage migration。
- protocol / DTO / Tauri command shape change。
- UI redesign。
- formatting-only rewrite と semantic change の混在。
- repo-wide mechanical move。

## Status Tracker

| wave | status | notes |
| --- | --- | --- |
| 0 | planned | 初期計画 document のみ。 |
| 1 | not started | Test responsibility split。 |
| 2 | not started | Harness waiters and scenario helpers。 |
| 3 | not started | Desktop shell orchestration boundaries。 |
| 4 | not started | App API support helper boundaries。 |
| 5 | not started | Desktop runtime community-node and stack boundaries。 |
| 6 | not started | Community-node server boundary review。 |
| 7 | not started | Metaverse UI/component responsibility check。 |

## Completion Report Template

この計画に従う PR または local refactoring slice は、完了時に以下を報告します。

```md
- Change type:
- Goal:
- Changed paths:
- Behavior changes:
- Public API / protocol / storage changes:
- Tests/contracts/scenarios added or updated:
- Validation run:
- Validation not run:
- Risks:
- Suggested follow-ups:
```
