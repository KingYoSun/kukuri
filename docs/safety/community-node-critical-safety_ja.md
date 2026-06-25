# Community Node Critical Safety Architecture（日本語版）

正本（canonical English）: `docs/safety/community-node-critical-safety.md`

最終更新日: 2026-06-25

## この文書の位置づけ

この文書は、public community node が user content の public indexing / discovery / recommendation / relation 出力を有効化する前に必要となる critical safety architecture を定義する。現時点のリポジトリ内の実装と docs、特に community node capability model、operator docs、P2P-first 責任境界、moderation event trust semantics を踏まえる。

現時点のリポジトリ実装は、auth / consent、bootstrap assist、topic rendezvous、iroh relay、report endpoint などの community-node connectivity と operator-support capability を提供している。一方で、community indexing、moderation、community-local trust は現時点では計画中の capability であり、提供中の runtime capability ではない。この文書は、それらが public content-surfacing path になる前に設計へ組み込むべき safety constraints を固定する。

- これは法的助言ではない。最終判断は各 community node operator 自身および専門家・関係機関への確認が必要である。
- これは production provider integration spec ではない。provider 契約、credential、本番 integration は、この文書が扱う現時点の実装範囲外である。
- これは provider outreach（事前問い合わせ）と、public indexing を有効化する前の architecture alignment のための設計文書である。

関連する実装と docs:

- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`
- `docs/architecture/moderation-event-trust-semantics.md`
- `docs/runbooks/community-node-operator-docs.md`
- `crates/cn-operator/src/capability.rs`
- `crates/cn-operator/src/manifest.rs`

## 1. 現在の実装状況（正直な現状）

kukuri の community node は、現時点では **early implementation phase** であり、auth / consent / bootstrap assist / topic rendezvous / iroh relay / report endpoint を中心とする接続補助層である。

- `community_index` / `moderation` / `community_local_trust` は **Phase B（計画中・この配布物では未提供）** である（`crates/cn-operator/src/capability.rs` の `Availability::Planned`、`docs/runbooks/community-node-operator-docs.md` の Phase A / Phase B 区分）。
- `report_endpoint` は Phase A（提供中）であり、`POST /v1/report` で通報を受け付け、`cn-cli reports` で運営者が確認できる。
- index / discovery / recommendation / relation のような content surfacing 機能は、まだ実装されていない。

**したがって、public indexing は fail-closed な critical safety architecture が実装されるまで有効化しない。** 本文書は、その safety architecture を後付けではなく **設計制約（architecture constraint）** として先に固定するためのものである。

## 2. P2P-first 責任境界（safety 判断のスコープ）

kukuri network 全体を統治する中央権者は存在しない。これは safety moderation があえて中央権者を「作らない」という選択の結果ではなく、kukuri が P2P を基盤としているために **そもそも構造的に不可能** だからである。network を所有する中央のチョークポイントが存在しないため、kukuri project であれ個々の community node であれ、たとえ望んだとしても network 全体の統治者の位置に立つことはできない。したがって safety architecture は、P2P 基盤の上に中央権者を追加することはできないという制約の **内側で** 機能するように設計する。

- community node は home server ではなく、P2P network に補助能力を提供する **service provider** である。
- user identity / profile / social graph は **node-independent** であり、いずれの node も canonical state として保持しないため、特定の community node に所有・凍結・削除されない。
- safety 判断（verdict / moderation event / risk signal）は、**issuer node の authority scope 内**でのみ効力を持つ。各 node は自分が index / moderate / cache / recommend した対象についてのみ、自分の出力からそれを排除できる。
- 呼び出せる中央権者がそもそも存在しないため、global moderation authority も network-wide takedown command も誰にとっても利用できない。critical safety であっても「network 全体に強制する中央権者」は存在せず、各 node は自分の authority scope 内でしか行動できない。

```text
safety verdict / moderation event = issuer node の authority scope 内の判断
                                  ≠ network-wide command（P2P 基盤上ではそのような command 自体が存在し得ない）
```

## 3. safety goals

public community node では、少なくとも以下を満たせる architecture を用意する。

- CSAM / CSE をはじめとする critical safety risk を、その node の index / discovery / recommendation / relation 出力から **積極的に排除**できる。
- 「積極的に排除している」ことを **説明・監査可能**にする（signed moderation event / risk signal による）。
- 個人・小規模 operator が CSAM / CSE メディアを **人力レビューすることに依存しない**設計にする。
- community node が blob 本体を **恒久保存しない（no permanent blob storage）**前提を維持する。

## 4. non-goals

- CSAM hash database を自前で保持・配布しない。
- CSAM 検知モデルを独自に学習しない。
- operator に CSAM / CSE メディアの人力レビューを要求しない。
- kukuri project / default node を network-wide moderation authority として記述しない。その役割は P2P 基盤上では成立し得ない。
- 一般 NSFW moderation と CSAM / CSE critical safety を同一 route で扱わない。

## 5. component boundary（責務分離・計画）

```text
community node
  - gossip / docs / nostr / local ingestion から投稿・参照を受ける
  - blob 本体は恒久保持しない
  - media / external blob reference を index 前に moderation server へ scan request する
  - `allow` verdict のみ index へ反映する
  - signed moderation event を保存・配布できる
  - trustness / relation へ risk signal を反映する

moderation server
  - provider credential を保持する
  - 必要に応じて blob を一時 fetch する（恒久保存しない）
  - scan provider / classifier / policy router を実行する
  - verdict を返す
  - incident / moderation event の元データを生成する
  - reporting workflow へ接続する hook を持つ
```

provider は単一 boolean ではなく capability として扱う（例: known CSAM hash match / perceptual hash match / unknown CSAM・CSE classifier / general media moderation / malware・phishing detection / reporting workflow）。

production provider integration を設定する前に、リポジトリ内では **mock provider** によって provider abstraction と readiness の振る舞いを検証できる状態にする。

## 6. data flow（計画）

```text
incoming post / blob reference / metadata
  ↓
community node
  ↓
moderation server
  ↓
policy router
  ├─ known CSAM hash matching
  ├─ unknown CSAM / CSE classifier
  ├─ general media moderation
  ├─ text moderation / grooming suspicion
  ├─ spam / abuse / malware / phishing
  └─ reporting workflow hook
  ↓
safety verdict
  ↓
index / search / discovery / recommendation / relation へは `allow` のときだけ反映
```

## 7. verdict model と routing（計画）

検知ラベルと最終 action を分離する。

- action: `allow` / `hold` / `quarantine` / `exclude`
- `csam_confirmed`（known hash match / provider confirmed）と `csam_suspected`（classifier high score / CSE 疑い）を明確に区別する。

| 区分 | 入力例 | routing |
|---|---|---|
| 既知 CSAM | known hash match / provider confirmed | `exclude` + critical moderation event + risk signal + reporting workflow hook（該当時） |
| 未知 CSAM / CSE 疑い | classifier high score / CSE suspicion | `hold` または `quarantine` + local-first risk signal + provider / report workflow への further route |
| 一般モデレーション | nsfw / violence / hate / harassment / spam / malware / phishing | critical とは別の non-critical policy route（allow / downrank / hide / exclude） |

未知 CSAM / CSE 疑いは `confirmed` として扱わず、`suspected` として critical route に分岐する。

## 8. fail-closed invariants

content-surfacing 実装では、以下を DB 制約・テストとして保証する。

- scan 前（unscanned）の media は **決して index しない**。
- scan failure / provider unavailable は **決して `allow` にしない**（fail-closed）。
- `hold` / `quarantine` / `exclude` verdict は search 可能にしない。
- critical verdict は discovery / recommendation に入れない。
- public-node profile は、必須の known-CSAM provider が未設定なら readiness check で失敗する。
- community node は blob 本体を恒久保存しない（**no permanent blob storage**）。

## 9. signed moderation event と risk signal

- signed moderation event は issuer node と対象（post / blob / user / peer）を識別し、issuer node が署名する。
- event は issuer node の authority scope 内の advisory であり、**network-wide command ではない**。
- risk signal は断定ラベルではなく、`basis` / `confidence` / `severity` / `visibility` を伴う根拠つき signal として扱う。
- visibility は `local` / `subscribed_nodes` / `public` の 3 段階。**suspected unknown CSAM / CSE では既定で `local`** とし、known CSAM hash match / provider confirmed の場合にのみ `subscribed_nodes` 以上を検討する（誤検知を public advisory として拡散しないため）。

## 10. reporting / appeal / operator audit

- 通報は中央集約せず、対象に実際に関与した node の authority scope 内へ route する。この責務境界は `crates/desktop-runtime/src/community_node/report_routing_support.rs` と community-node manifest model で表現される。
- moderation event / incident の記録・通報では、有害コンテンツそのものを evidence として再配布せず、event ID / reference ID と根拠区分で説明する。
- operator が「積極的に排除している」ことを、生の有害コンテンツを再配布せずに監査・説明できる状態にする。

## 11. public indexing 前の readiness（最低条件）

public-node profile では、public indexing を有効化する前に少なくとも以下を満たす。

- known-CSAM provider が設定されている。
- provider credential が readiness check で検証されている。
- `index_before_scan = false`。
- scan error 時は hold（fail-closed）。
- signed moderation events が有効。
- permanent blob storage が無効。
- scan coverage metrics が取得できる。

## 12. 実装前提

現時点のリポジトリは、public indexing / moderation / community-local trust を提供中の runtime capability としてまだ持たない。これらが public content-surfacing path になる前に、少なくとも以下の実装と検証を追加する必要がある。

- provider abstraction と provider capability model
- deterministic test のための mock provider coverage
- known CSAM、suspected unknown CSAM / CSE、general moderation を分離する policy routing
- public-node profile の readiness check
- fail-closed indexing constraints
- signed moderation event generation
- risk signal persistence と distribution semantics

production provider への申請・本番 integration は、production credential に依存せず provider abstraction と readiness check を検証できるようになってから扱う。

## 13. Provider outreach summary（事前問い合わせ用要約）

以下は Project Arachnid Shield / Microsoft PhotoDNA / Thorn Safer / Hive 等の safety provider への事前問い合わせ用の短い要約である。意図する architecture と現時点の early-stage implementation status を説明するものであり、承認済みの production integration を表すものではない。問い合わせ時にそのまま使えるよう、引用ブロックは英文のまま保持する。

> **kukuri — community node critical safety, provider outreach summary**
>
> kukuri is a P2P-first social application. Community nodes are auxiliary service providers for the P2P network; they are **not** home servers, and user identity, profile, and social graph are node-independent. A network-wide moderation authority is not merely absent by policy — it is structurally impossible on a P2P foundation, because there is no central chokepoint that could govern the whole network. A node's safety decisions therefore apply only within that node's own authority scope.
>
> kukuri is currently in an early implementation phase. Community nodes today focus on auth/consent, bootstrap assist, topic rendezvous, iroh relay, and a report endpoint. Community indexing, moderation, and trust signals are **planned, not yet shipped**.
>
> Before enabling any public indexing, we intend to build CSAM / critical safety as an **architecture constraint, not a later add-on**:
>
> - Media and external blob references are scanned by a moderation server **before** indexing.
> - Indexing is **fail-closed**: unscanned media, scan failures, and provider-unavailable states are never indexed or surfaced.
> - Known CSAM, suspected unknown CSAM/CSE, and general moderation are architecturally separated.
> - Matched or suspected critical content is excluded from index, discovery, and recommendation.
> - Community nodes do **not** permanently store blob bodies.
> - Exclusion decisions are recorded as signed, auditable moderation events so operators can demonstrate active exclusion.
> - We do not ask operators to manually review CSAM/CSE material, and we do not host or train our own CSAM detection database/model.
>
> We would like to confirm: (1) whether individual or small third-party community node operators can obtain access to your services, and (2) whether the kukuri project could serve as an integration coordinator / centrally approved integration provider on their behalf.
>
> Formal application and production integration would follow our mock-provider and readiness-check implementation; this outreach is to confirm eligibility and the right integration model first.

## 関連

- 正本: `docs/safety/community-node-critical-safety.md`
- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`
- `docs/architecture/moderation-event-trust-semantics.md`
- `docs/runbooks/community-node-operator-docs.md`
- `docs/architecture/default-community-node-dependency-reduction.md`
