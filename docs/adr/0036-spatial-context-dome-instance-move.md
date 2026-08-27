# ADR 0036: Spatial Context単位のDome Instanceと引っ越し

## Status

Accepted

## Context

Metaverseはpublic topicだけでなくprivate channelでも動作する。参加・閲覧・書き込み権限は既存のtopic / channel機能へ委譲し、Metaverse固有のmembershipを追加しない。一方、Domeの見た目、environment、gravity強度、persistent prop初期配置、参照assetはownerのユーザー資産であり、配置先のSpatial Contextから独立して再利用できなければならない。

Metaverseは実験機能なので、旧`world_version = 1` / `2` payloadのdecode fallback、migration、二重読み取りは提供しない。旧Domeは再作成する。

## Decision

### Spatial Context identity

`SpatialContextV1`を次の閉じた型とする。

- topic: `topic:{topic_id}`
- channel: `channel:{topic_id}:{stable_channel_id}`

channel identityにはepoch replica idではなく安定したchannel idを使う。実際のread/write replicaとepoch rotationは既存private channel機能が決定する。topicとchannel、および異なるtopic内の同名channelは同一Contextとして扱わない。

### Feature Data Classification

- Feature 名: owner-owned Dome Preset / Context-owned Dome Instance / Dome move
- Durable / Transient:
  - Durable: Preset current pointerとmanifest、Instance slot / manifest、generation、status、relationship detach marker、move record
  - Transient: avatar / prop transform、presence、実行中session event、UIのmove進行表示
- Canonical Source:
  - Preset current pointer: owner author replicaの`metaverse/dome-presets/{preset_id}/state`
  - Preset manifestとasset: content-addressed blob
  - Instance owner slot: Context replicaの`metaverse/dome-instances/{owner_pubkey}/state`
  - Instance manifest: owner署名済みcontent-addressed blob
  - move record: owner author replicaの`metaverse/dome-moves/{move_id}/state`
- Replicated?: Yes
- Rebuildable From: `docs + blobs`
- Public Replica / Private Replica / Local Only: Instanceは対象topic / private-channel replica、Presetとmove recordはauthor replica、SQLiteはlocal projection
- Gossip Hint 必要有無: room projection更新の同期開始に`SessionChanged`を使うが、hintを正本にしない
- Blob 必要有無: Yes。Preset / Instance manifest、texture、VRM、GLBをcontent hashで保存する
- SQLite projection 必要有無: Yes。現行UIの`game_room_cache`は解決済みDome viewを保持する再構築可能なread modelであり、owner slotの一意性やlifecycle authorityには使わない
- 必須 contract: Context canonicalization、owner slot一意性、署名者とowner一致、Preset refのid / owner / hash binding、generation一致、same-context relationship、旧generation event拒否、tombstone非表示
- 必須 scenario: `desktop_smoke_metaverse_dome_move`

### PresetとInstanceの境界

`DomePresetManifestV1`だけが次を所有する。

- `fixed_dome_v1` customization（surface、environment、gravity強度）
- persistent prop初期定義と配置
- texture / model / VRMなどのasset refs

`DomeInstanceManifestV1`だけが次を所有する。

- instance id、Spatial Context、owner、Presetのimmutable manifest ref
- title / description、instance-local max peers、spawn
- lifecycle generationと`staging | active | tombstoned`
- sessionに復元するdurable chat history
- relationship detach markerと置換先instance id

InstanceはPreset manifest hashを参照する。引っ越し先でPresetやasset bytesを複製しない。`GameRoomManifestBlobV1.metaverse`と`game_room_cache.metaverse`はdesktopへ返す解決済みread modelであり、Preset / Instanceのauthorityではない。

### Owner slotとgeneration

Instance idとowner slot keyは`Spatial Context canonical id + owner pubkey`から決定論的に導出する。同じContext replicaの同じowner slotに`active`または`staging`が存在する場合、新規作成・別moveによる上書きを拒否する。tombstoned slotの再利用時はgenerationを増加させ、旧generationのrelationship、session、eventを受理しない。

### Relationship scope

Connection / proposalの両端は完全に同じ`SpatialContextV1`で、両Instanceが`active`、かつ両方にrelationship detach markerがない場合だけ有効である。detach markerはmove idとsource generationに束縛する。引っ越し先には旧Connection / proposalをコピーしない。実際のConnection recordとproposal queueはIssue #792でこのcontractを利用する。

### Move state machine

同じmove idの再実行は保存済みphaseから継続する。異なるsourceまたはtargetへ同じmove idを再利用してはならない。

| Phase | 完了条件 | 途中で失敗した場合 | Retry |
|---|---|---|---|
| `preparing` | owner、source active generation、target authority / 空slot、Preset / 全assetを検証 | sourceはactive / attachedのまま | 検証とstagingを再実行 |
| `target_staged` | 同じPreset refを持つtarget Instanceをstaging保存。UI一覧には出さない | sourceはactive / attachedのまま | 同一generationのstagingだけ再利用 |
| `source_detached` | source generationにdetach markerを保存 | sourceはまだactiveだが旧relationshipは無効 | target公開から継続 |
| `target_active` | targetをactiveとして公開 | targetだけが新規session / eventを受理。source一覧復活は許さない | source tombstoneから継続 |
| `source_tombstoned` | sourceにreplacement idを保存してtombstone化 | targetはactive、sourceは非表示 | 完了record保存を再実行 |
| `completed` | 全段階完了 | なし | 同じrecordを返す |

target staging前のPreset / asset検証失敗ではsourceを変更しない。detach後の失敗ではsourceをtombstone化せず、同じmove idで安全に継続する。target公開後はsourceを再びactiveとして扱わず、再試行・restart時にtombstone処理を完了する。

### Event binding

署名済み`metaverse-room-event`はinstance id、canonical Spatial Context、generation、session idをcontentへ含める。受信・一覧・publish時に現在のactive / attached Instanceと照合し、Context不一致、旧generation、tombstoned Instanceのeventを拒否する。

## Consequences

- topicとchannelのどちらでも、ownerごとにDomeは最大1つへ収束する。
- customizationまたはasset importは新しいPreset manifest versionを作り、InstanceのPreset refだけを更新する。
- move後もPreset / texture / model hashは同一で、blob storageのcontent addressingにより同一bytesを重複保存しない。
- staging / tombstoned rowはdiscoveryから除外され、score game roomの既存挙動は変えない。
- Hosting Leaseとauthoritative physicsはIssue #788、Connection / proposalの保存形式はIssue #792、guest layout commitはIssue #793の責務とする。
