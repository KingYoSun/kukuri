# Default Community Node 依存低減ロードマップ

最終更新日: 2026-06-19

## この文書の位置づけ

kukuri には onboarding のための **default community node** が存在する。これは初期利用体験のために必要な妥協だが、**default node があることは kukuri project が network-wide authority であることを意味しない**。そもそも P2P 基盤上には network 全体を統治する authority という座が存在しないため、default node であってもそれを担うことは構造的に不可能である。

この文書は、default community node への暗黙依存を段階的に減らす roadmap を固定する。前提は `docs/architecture/p2p-first-community-node-responsibility-boundary.md`（P2P-first 責任境界）であり、user identity / profile / social graph は default node に所有されない。

この文書は法的助言ではなく、central node registry を network-wide authority として導入するものでもない。P2P 基盤上では、そのような中央権者は構造的に成立し得ない。

## 1. Default node の位置づけ

- default node は **onboarding infrastructure** である。初回起動時に peer 探索・relay・rendezvous 等の補助を提供し、user が network へ参加しやすくする。
- default node は **network-wide authority ではない**。「default である」ことは「kukuri network 全体の運営者・通報先・moderation 権者・凍結権者である」ことを意味しない。P2P 基盤上にはそのような network 全体の authority が構造的に存在しないため、default node がそれを担うこともできない。
- default node の policy（terms / moderation / retention）は、**default node が提供する capability にのみ適用される**。
- default node は **third-party node の content / index / moderation / trust signal には責任を持たない**。
- user identity / profile / social graph は default node に所有されない。default node が停止・変更・削除されても、これらは失われない（`docs/runbooks/community-node-shutdown-and-user-continuity.md` 参照）。

## 2. Hidden dependency の棚卸し

現在 client が default node に（暗黙に）依存している点を棚卸しする。これは「減らすべき対象」のベースラインである。

| 依存項目 | 現状の依存度 | canonical か補助か |
|---|---|---|
| initial bootstrap（初期 peer 探索） | 高（初回起動で default node 前提） | 補助。DHT / 既知 peer で代替可能。 |
| relay assist（NAT 越え） | 中（直接 P2P 不可時に default relay へ） | 補助。直接 P2P / 別 relay で代替可能。 |
| topic rendezvous（topic peer 合流） | 中 | 補助。別 rendezvous / 既存 peer で代替可能。 |
| consent / auth | 中（補助機能利用時） | 補助。利用する capability にのみ必要。 |
| future index / moderation / trust signal | 低（未実装 / 計画中, Phase B） | 補助かつ optional trust input（`docs/architecture/moderation-event-trust-semantics.md`）。 |
| generated policy / manifest display | 低（表示のみ） | 補助。manifest 取得失敗時は fallback 表示（client settings の node 依存度表示）。 |

重要: いずれも **補助 capability** であり、user identity / profile / social graph という canonical state は default node に依存しない。依存低減とは「補助経路を default node 以外でも自然に賄えるようにする」ことであり、「user の存在を default node から切り離す」ことではない（それは既に切り離されている）。

## 3. 依存低減フェーズ

```text
Phase 0: default node fixed / preview onboarding
  - 初回起動は default node を前提に動く（現状）。
  - ただし identity / profile / social graph は既に node-independent。

Phase 1: settings で node dependency visibility を表示
  - どの node のどの capability を利用しているかを settings で可視化する。
  - identity / profile / social graph が node-owned ではないことを明示する。
  - default node が network-wide authority ではない（P2P 基盤上ではそもそも成立し得ない）ことを明示する。

Phase 2: user-added community node を first-class に扱う
  - user が追加した community node を default node と対等に扱う。
  - 複数 node の capability を併用できる。
  - default node を「特別な中央」ではなく「既定の 1 候補」として扱う。

Phase 3: first-run で node selection / import を可能にする
  - 初回起動時に node 選択 / manifest import / ticket import を可能にする。
  - default node を使わない初期化経路を用意する。

Phase 4: default node なしでも継続利用できることを検証する
  - default node の停止・変更・削除後も、既存 identity / social graph /
    local data が継続利用できることを検証する。
  - 補助経路（bootstrap / relay / rendezvous）が default node 以外で
    成立することを検証する。
```

### 現在地

- Phase 1 の基盤（settings の node 依存度 / capability scope / authority scope 表示）は実装済み。identity / profile / social graph が node-owned ではないこと、default node が network-wide authority ではないこと（P2P 基盤上ではそもそも成立し得ないこと）を settings で明示している。
- Phase 2 の前提（manifest authority scope / public manifest endpoint / content provenance / 分散通報ルーティング）が揃っており、user-added node を first-class に扱うための土台はある。
- Phase 3 / Phase 4 は今後の実装対象。

## 4. Release criteria（各フェーズで何を壊すか）

各フェーズで、default node の **停止 / 変更 / 削除** が何を壊し、何を壊さないかを明示する。これがリリース判断基準となる。

| フェーズ | default node 停止で壊れるもの | 壊れないもの（不変条件） |
|---|---|---|
| Phase 0 | 初回 onboarding 経路（新規 user の参加が難しくなる） | 既存 user の identity / profile / social graph |
| Phase 1 | 同上。ただし user は依存内容を把握できる | 同上 + 依存の可視性 |
| Phase 2 | 初回 onboarding のみ。既存 user は user-added node で継続 | identity / social graph + 複数 node 併用 |
| Phase 3 | 何も壊れない（default node 不要で初期化できる） | onboarding 含め継続可能 |
| Phase 4 | 何も壊れない（検証済み） | すべて継続可能 |

全フェーズ共通の不変条件:

- user identity / profile / social graph は default node に所有されず、停止しても失われない。
- default node は network-wide authority ではない（P2P 基盤上ではそもそも成立し得ない）。停止しても network は継続する。

## 受け入れ条件との対応

- default node dependency reduction roadmap が文書化されている → 3
- default node が onboarding infrastructure であり、network-wide authority は P2P 基盤上で成立し得ないことを明記している → 1
- 現在の default node 依存が棚卸しされている → 2
- dependency reduction の段階が定義されている → 3
- default node outage / shutdown / replacement 時に何が失われ、何が残るかを説明している → 4
- identity / profile / social graph が default node に所有されないことを明記している → 1, 2, 4
- P2P-first responsibility boundary と整合している → 全体

## 関連

- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`
- community node manifest authority scope / capability scope
- public manifest endpoint
- client settings の node 依存度表示（Phase 1 実装）
- content provenance / responsible capability metadata
- `docs/runbooks/community-node-shutdown-and-user-continuity.md`（community node shutdown と user continuity）
- `docs/architecture/moderation-event-trust-semantics.md`（moderation event の trust semantics）
