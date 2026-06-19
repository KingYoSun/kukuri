# P2P-first community node の責任境界

最終更新日: 2026-06-19

## この文書の位置づけ

この文書は、kukuri の community node が負う責任と負わない責任の境界を固定するための基準である。

`#352`（operator docs / 機能トグル生成 CLI）、`#353`（CSAM / critical safety moderation 基盤）、`#310`（分散通報ルーティング）など、community node operator の法務・運用・safety・通報に関する作業は、すべてこの責任境界を共通前提とする。

ここで境界を固定しないと、operator docs / safety architecture / report routing / default node policy が、暗黙のうちに「kukuri network 全体の運営者」を仮定してしまう。kukuri はそのような中央運営者を持たない。

このページは法的助言ではなく、central governance model / network-wide moderation policy を導入するものでもない。あくまで実装・文書・UX が共有すべき責任境界の定義である。

## 1. kukuri は Mastodon clone ではない

- community node は Mastodon 的な home server **ではない**。
- user identity / profile / social graph は **node-independent** である。これらは特定の community node に属さず、user の鍵に紐づき、P2P network 上で流通・同期される。
- community node は、P2P network に補助能力を提供する **service provider** である。user の所属先（home）でも、user の存在の前提でもない。

kukuri の基本通信優先度は `Direct P2P -> Relay Supported P2P -> Relay Fallback` であり、community node はこの経路を補助するための層に位置する。詳細は `AGENTS.md` の「通信経路」および `docs/adr/0010-kukuri-protocol-v1-boundary-definition.md` を参照する。

### node-independent であることの含意

- user を識別・認証する canonical source は user 自身の鍵であり、いずれかの community node ではない。
- ある community node が停止・消失しても、user identity / profile / social graph は失われない。別の node や直接 P2P 経路から引き続き参照・再構築できる。
- したがって、community node を「アカウントの所在地」「退会先」「凍結権者」として設計してはならない。

## 2. Community node の責務（capability scope 内）

community node は、operator の設定に応じて以下の capability を**提供し得る**。ただし、いずれも node が明示した capability と authority scope の範囲に限定される。

- auth / consent
- bootstrap assist
- relay assist
- topic rendezvous
- community index
- moderation
- trust signal publication
- media cache
- report endpoint
- optional gateway / bridge

これらは「有効化したものだけ」「自分が関与した対象についてだけ」責任を持つ、という形で限定される。node manifest が宣言した capability / authority scope が、その node の責任範囲の上限である。

- 例: `community index` を有効化した node は、自分が index した content についてのみ index 責任を負う。他 node が index した content には責任を負わない。
- 例: `moderation` を有効化した node の moderation event / trust signal は、その node の authority scope 内でのみ意味を持つ（`#353` の signed moderation event / risk signal、`#310` の report routing を参照）。

## 3. Community node の非責務

community node は、少なくとも以下を**負わない**。これらは operator docs / safety / report flow のいずれにおいても node に帰属させてはならない。

- kukuri network 全体の運営
- user identity の所有
- profile の canonical store
- social graph の canonical store
- 全 content の truth source
- third-party node の活動に対する責任
- global moderation authority
- central abuse intake（中央通報窓口）

つまり、ある node は「自分が index / moderate / cache / relay / recommend した対象」についてのみ責任を負い、network 全体や他 node の振る舞いについては責任を負わない。通報・moderation・trust signal はいずれも node 単位の authority scope に閉じる。

## 4. Default community node の位置づけ

kukuri は onboarding を成立させるために default community node を用意し得るが、その位置づけは限定的である。

- default community node は **onboarding compromise / onboarding infrastructure** である。新規 user が最初の接続・bootstrap・初期体験を得るための補助に過ぎない。
- default node は **network-wide authority ではない**。「default である」ことは「kukuri network 全体の運営者・通報先・moderation 権者である」ことを意味しない。
- default node policy は、**default node が提供する capability にのみ**適用される。
- default node は、third-party node の content / index / moderation / cache / trust signal に対して責任を持たない。
- 通報先は「default node があるから default に集約する」のではなく、対象を実際に表示・索引・moderation・cache・recommend した node の authority scope に基づいて決まる（`#310` を参照）。provenance 不明時に default node / kukuri project へ fallback してはならない。

## 5. Operator safety work の位置づけ

`#352` / `#353` / `#310` をはじめとする operator docs / safety / report flow の整備は、community node を中央 SNS 運営者にするためのものではない。

これらの目的は、**P2P network の補助層を、個人・小規模グループでも現実的かつ説明可能に運用できるようにする**ことである。

- operator docs（`#352`）: operator が有効化した capability に対応した説明責任（terms / privacy / 外部送信表示 / 届出補助など）を決定論的に生成し、分散運用の説明責任負荷を下げる。これは node を network 全体の運営者にするものではない。
- safety moderation 基盤（`#353`）: operator が自分の index / discovery / recommendation / relation から critical safety risk を排除できるようにする。verdict / moderation event / risk signal は node の authority scope 内で機能し、global moderation authority を作らない。
- 分散通報ルーティング（`#310`）: 通報を中央集約せず、対象に実際に関与した責任ある node へ route する。default node や kukuri project を network-wide の中央通報窓口にしない。

## 受け入れ観点（この文書が満たすべきこと）

- P2P-first responsibility boundary を明文化している。
- community node が home server ではないことを明記している。
- user identity / profile / social graph が node-independent であることを明記している。
- community node の責務と非責務を分離している。
- default community node が onboarding infrastructure であり network-wide authority ではないことを明記している。
- `#352` / `#353` / `#310` から共通前提として参照できる。
- operator docs / safety / report flow が、P2P 補助層を分散運用可能にするためのものであることを明記している。

## 関連文書

- `AGENTS.md`（通信経路の優先度と community node の役割）
- `docs/adr/0009-community-node-relay-auth-data-classification.md`（community-node connectivity/auth のデータ分類）
- `docs/adr/0010-kukuri-protocol-v1-boundary-definition.md`（kukuri Protocol v1 の境界）
- `docs/runbooks/community-node-self-host-vps.md`（community node self-host 運用）
