# moderation event / safety advisory の trust semantics

最終更新日: 2026-06-19

## この文書の位置づけ

この文書は、community node が発行する moderation event / safety advisory / risk signal を、kukuri network 上で**どのような信頼関係として扱うか**を固定するための設計ドキュメントである。

#353 で実装する CSAM / critical safety moderation 基盤は、signed moderation event と trustness / relation risk signal を生成する。本文書は、それらが **network-wide command ではなく、issuer node の authority scope 内の判断、または各 node / client / user が任意に購読する optional trust input** であることを明確化する。

共通前提は `docs/architecture/p2p-first-community-node-responsibility-boundary.md`（P2P-first 責任境界, #354）であり、manifest の authority scope は #355、通報ルーティングとの連携は #310 を参照する。

この文書は法的助言ではなく、central governance model / network-wide moderation policy を導入するものでもない。

## 1. 基本原則: advisory であって command ではない

kukuri には network 全体の moderation authority は存在しない。したがって、ある node が発行した moderation event / safety advisory は、次のように扱う。

- moderation event は **issuer node によって署名される**。署名は「どの node がこの判断を下したか」を示すものであり、「network 全体がこれに従う」ことを意味しない。
- event の効果は **issuer node の authority scope に限定される**。issuer node は自分が index / moderate / cache / relay / recommend した対象についてのみ、この判断を自分の出力（index / discovery / recommendation / relation）へ反映できる。
- 他の node / client / user は、その signal を **任意に trust input として採用できる**。採用するかどうか、どの重みで扱うかは受け手の判断である。
- advisory は **network-wide command ではない**。他 node に対して強制的な削除・凍結・quarantine を要求するものではない。
- kukuri project / default node は **global moderation authority ではない**。default node が発行した moderation event も、他のあらゆる node の event と同じく optional trust input にすぎない。

```text
moderation event = signed opinion of one node, scoped to that node's authority
                 ≠ network-wide takedown command
```

### 例外: critical safety の扱い

CSAM / CSE / malware / phishing 等の critical safety（#353）であっても、「network 全体に強制する中央権者」を作るわけではない。各 node は自分の index / discovery / recommendation から critical content を**自分の判断で**排除し、その判断を signed moderation event として説明・監査可能にする。受け手側が critical safety signal を強く重み付けるのは自然だが、それは受け手の採用判断であり、network-wide command ではない。

## 2. issuer node / authority scope / visibility の意味

### issuer node

moderation event / risk signal を発行・署名した node。`issuerNodeId` で識別する。受け手は「誰の判断か」を常に辿れる必要がある。

### authority scope

issuer node が manifest（#355）で宣言した `authority_scope` の範囲。moderation event の効果はこの範囲に限定される。具体的には:

- `applies_to`: issuer node が責任を主張する対象（例: `this_node`, `communities_indexed_by_this_node`, `moderation_events_issued_by_this_node`, `media_cached_by_this_node`）。
- `does_not_apply_to`: issuer node が権威を持たない対象（`kukuri_network_as_a_whole`, `third_party_nodes`, `user_identity`, `user_profile_canonical_source`, `user_social_graph_canonical_source`）。

つまり、ある node の moderation event は **user identity / profile / social graph には適用されない**。これらは node-independent であり、moderation event によって所有・凍結・削除されない。

### visibility

#353 の model と整合する 3 段階。signal がどこまで配布されるかを示す。

| visibility | 意味 | 主な用途 |
|---|---|---|
| `local` | issuer node 内のローカル判断にのみ使う | suspected unknown CSAM / CSE、low-confidence classifier 結果 |
| `subscribed_nodes` | この node を trust input として購読している node にのみ配布 | known hash match / provider confirmed の中程度共有 |
| `public` | 公開 advisory として誰でも参照可能 | 明確に公開すべき confirmed result（運用ポリシー次第） |

**suspected unknown CSAM / CSE では、まず `local` を基本とする。** 既知 CSAM hash match / provider confirmed result の場合にのみ `subscribed_nodes` 以上を検討する。誤検知を public advisory として拡散しないための安全側の既定である。

## 3. trust input model

受け手（node / client / user）は、他 node の moderation event / risk signal を次のように扱う。

```text
incoming signed moderation event / risk signal
  ↓
issuer node を確認（署名・manifest authority scope）
  ↓
受け手が issuer を trust input として購読しているか？
  ├─ No  → 参考情報として保持（自動適用しない）
  └─ Yes → 受け手のポリシーに従って重み付け採用
            （downrank / hide / exclude / risk label 表示 等）
  ↓
適用は受け手の出力（自分の index / discovery / UI）に閉じる
```

重要な不変条件:

- 受け手は、purchase していない（購読していない）issuer の signal を**自動的に強制適用しない**。
- 受け手が signal を採用しても、その効果は受け手自身の出力に閉じる。他 node や user の canonical state は変更しない。
- 断定ラベルではなく、根拠つき risk signal（`basis` / `confidence` / `severity`）として扱う。受け手はこれを使って重み付けを決める。

## 4. client が説明できるべきこと

client が moderation label / risk signal を表示する場合、少なくとも次を説明できる状態にする（断定的なラベルを根拠なしに表示しない）。

- **issuer node**: どの node の判断か
- **target**: 対象（post / blob / user / peer）
- **category**: csam / cse / grooming / nsfw / spam / malware / phishing 等
- **severity**: critical / high / medium / low
- **basis**: known_hash_match / provider_verdict / classifier_score / local_policy
- **confidence**（あれば）: classifier スコア等
- **expiresAt**（あれば）: 失効時刻
- **visibility**: local / subscribed_nodes / public
- **subscription state**: この client / 現在の node がこの signal を購読しているか

これにより、user は「これは誰の判断で、どの根拠に基づき、自分はそれを採用しているのか」を理解できる。kukuri は「network 全体がこの user を危険と認定した」という誤った印象を与えない。

## 5. report routing との連携（#310）

risk signal が通報対象に関与している場合、#310 の report routing は issuer node を **report target の候補**として含める。

- moderation label / trust signal を発行した issuer node は、その signal の発行に関して responsible な node である。
- したがって、その label / signal に異議がある場合（誤検知の申し立て等）の通報先候補として、issuer node の report endpoint / abuse contact を提示できる。
- これは provenance（#358）の `observedVia`（capability = `moderation` / `trust_signal`）として表現され、`reportRouting`（#310）が manifest authority scope と突き合わせて候補化する。

report routing は中央集約しない原則を守り、issuer node の authority scope 内へ通報を route する。

## 6. default node を global moderation authority と誤解させない

- default-onboarding-node が発行する moderation event も、他 node の event と**同じ optional trust input** である。
- 「default node がある」ことは、「default node が network 全体の moderation 権者である」ことを意味しない。
- client は default node の moderation label を、他 node の label と区別なく「issuer node の判断」として説明する。default node の判断を特権的な network verdict として表示しない。
- provenance 不明時に moderation 判断を default node へ fallback して帰属させない（#358 / #310 の不変条件と整合）。

## 受け入れ条件との対応

- moderation event / safety advisory が optional trust input であることを文書化している → 1, 3
- network-wide command ではないことを明記している → 1
- issuer node / authority scope / visibility の意味が定義されている → 2
- local / subscribed nodes / public の使い分けが #353 と整合している → 2（visibility 表）
- client が issuer / basis / severity / visibility を説明できる前提になっている → 4
- #310 の report routing と連携できる → 5
- default node を global moderation authority と誤解させない → 6

## 関連

- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`（#354）
- community node manifest の authority scope / P2P boundary（#355）
- public manifest endpoint（#356）
- content provenance / responsible capability metadata（#358）
- 分散通報ルーティング（#310）
- Public community node 向け CSAM / critical safety moderation 基盤（#353, この signal の生成側）
