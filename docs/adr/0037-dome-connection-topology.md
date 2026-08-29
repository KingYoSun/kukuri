# ADR 0037: Dome Connection record・proposal queue・component-local topology

## Status

Accepted

## Context

固定DomeはNorth / East / South / Westの最大4方向で、同一Spatial Contextにいる双方ownerの合意によって接続する。ConnectionをDome manifestやglobal mapへ埋め込むと、owner authority、引っ越し時のrelationship破棄、P2P replica上の同時操作を分離できない。そこでConnectionの事実だけを署名済みrecordとして保存し、world topologyはrecord集合から導出する。

## Decision

### Feature Data Classification

- Feature 名: Dome Connection agreement / proposal queue / component-local topology
- Durable / Transient:
  - Durable: proposal、receiver slot selection、双方署名済みagreement、draining / revoke / invalidation event
  - Transient / Derived: `waiting_for_slot`、active候補の裁定結果、component membership、component-local座標、topology digest、UI進行状態
- Canonical Source: 対象Spatial Context replica上の署名済みappend-only record
- Replicated?: Yes
- Rebuildable From: Context replicaのConnection recordとcurrent Dome Instance slot
- Public Replica / Private Replica / Local Only: public topicまたはprivate channel replica。SQLiteはlocal projectionのみ
- Gossip Hint 必要有無: Yes。Contextとtopology digestを持つ同期開始通知だけに使い、hintを正本にしない
- Blob 必要有無: No。Connection recordは小さなJSONで、asset bytesを含まない
- SQLite projection 必要有無: Yes。proposal queue、active Connection、component / coordinatesをread modelとして保持できるが、docsから削除・再構築可能とする
- 必須 contract: 双方署名、owner / Instance generation / Context binding、4方向opposite、proposal lifecycle、slot競合収束、component join制約、cycle / coordinate collision拒否、revoke split、move / tombstone / block invalidation、queue / frequency上限
- 必須 scenario: `desktop_smoke_metaverse_dome_connections`

### Recordと署名境界

proposalはproposer Instance、generation、owner、方向、receiver Instance、generation、owner、Spatial Context、sequenceをproposerが署名する。receiverはproposal idと自分の方向slot、slot generation、観測済みactive Connectionをselectionとして署名する。Connection agreementは両endpoint、opposite方向、proposal id、activation generationをcanonical JSONにし、双方ownerが同一bytesへ署名する。selection generationと観測済みactive Connection idはreceiver署名済みselectionおよびlifecycle recordへ保持する。片署名、異なるcontentへの署名、owner以外の署名は無効である。

ConnectionはDome Preset / Instance manifestへ埋め込まない。引っ越し先へ旧recordをコピーせず、ADR 0036のdetach markerまたはtombstoneを観測した時点で旧Instance generationに束縛されたrecordを無効化する。

### Lifecycleとproposal queue

- `proposed`: proposer署名が有効で、破棄条件がない。
- `reserved`: receiverがslot generation付きselectionを署名した状態。hard reservationではない。
- `accepted`: agreementへの双方署名が揃った状態。
- `active`: accepted候補がslot・component・座標制約を満たし、競合裁定でwinnerになった導出状態。
- `waiting_for_slot`: 未選択、またはreceiver slotだけが別のactive Connectionで占有されている導出状態。
- `draining` / `revoked`: いずれかのendpoint ownerが署名できるterminal lifecycle。途中失敗は同じoperation id / generationで再開する。

proposerの指定slotに別Connectionが成立、proposerが同じslotで別proposalからConnectionを成立、どちらかのInstanceがdetach / tombstone / delete、owner間block入力、proposer withdrawではproposalをterminal discardとする。receiver slotが別Connectionで占有されたことだけではdiscardせず、待機リストに残す。owner間blockによる`owners_blocked` terminal eventはblockを入力したどちらのendpoint ownerも署名でき、block解除後もproposalを復元しない。receiverはaccept recordを保存する直前にも双方向blockを再検査し、block済みならproposalだけをterminal discardしてConnection recordを作らない。ADR-0043がsigned blockを検出してactive Connectionを`owners_blocked`で即時revokeする。

### Concurrencyと上限

receiver selectionはowner署名付き単調増加slot generationで更新する。同generationの並行selectionはrecord digestでtotal orderを作る。activationは観測済みactive Connection idを因果参照し、因果的に先行するactive edgeを後着proposalが置き換えない。並行候補だけをcanonical digestで裁定し、入力列挙順やwall-clockに依存しない。

初期値は次のとおりとする。

- 1 Domeあたりnon-terminal outbound proposal: 最大32件
- 同一proposerから同一receiver slot: 最大4件
- receiver slotのeligible queue: 最大32件
- local create: owner / Spatial Contextごとに10分間で最大8件

受信側は署名、sequence、決定論的queue capを検証する。local frequency limitはUX / abuse guardであり、network-wide consensusやtrusted clockを主張しない。

### Topology導出

active agreementを決定論的な因果順とdigest順で処理し、次だけを許可する。

- 両方が未接続のDomeを最初のcomponentとして接続する。
- 未接続Domeを既存componentへ1つ追加する。
- endpointの方向slotが双方とも空いている。
- `fixed_dome_v1`のopposite方向と5,700cm offsetで新座標が空いている。

既存component同士の結合、同一component内cycle、slot重複、既存座標との衝突は副作用なく拒否する。componentは保存しない。active edge集合ごとにcanonical instance id最小のDomeをroot `[0, 0, 0]`としてBFSし、`fixed_dome_v1().endpoints[].adjacent_dome_offset_cm`から各座標を得る。Connection revoke後は残ったedge集合だけで再計算し、分離したsubcomponentはそれぞれ新しいrootを持って存続する。global座標は定義しない。

## Consequences

Issue #797の通常解除、draining deadline、安全上の即時terminal化は[ADR-0045](0045-dome-offline-draining-return-home.md)で追加定義する。

- proposalの待機とConnectionの事実をDome assetから独立して保持できる。
- 全peerは同じ署名済みrecord集合から同じforestと相対座標を得られる。
- seamless transitionはこのtopology viewを利用できるが、scene描画・prefetch・physics handoffはIssue #790以降の責務である。
- component mergeとcycleを後から許可する場合は、新しいworld versionと競合規則が必要になる。
