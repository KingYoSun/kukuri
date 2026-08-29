# ADR-0044: Spatial Context entry and authoritative safe spawn

- Status: Accepted
- Date: 2026-08-29
- Issue: #796

## Context

Spatial Contextを開いた時点では、複数のDomeが存在し、直前まで有効だったhost、channel参加資格、block、capacityが変化している可能性がある。Client側の選択だけでavatarを表示すると、拒否後にghost participantが残るほか、propやavatarと重なる位置へspawnし得る。

## Decision

### Entry resolution

- 入場候補は、active hostを持つDomeだけから「local author所有」「同じContextで最後にauthoritative入場したInstance」「private channel ownerが設定したentry Instance」「Instance IDの安定順」の順に重複排除して評価する。
- 最終訪問履歴はauthorとSpatial Contextをkeyにしたlocal-only stateであり、authoritative `Join`成功またはDome transition commit後だけ更新する。選択、preview、失敗では更新しない。
- private channelはcurrent policyに0または1件の`entry_dome_instance_id`を持つ。既存channel ownerだけが、同じSpatial Contextのactive Instanceへ設定または解除できる。epoch rotationではcurrent policyへ引き継ぐ。
- entry設定はConnection topologyへ入力しない。参照先のhost停止、move、削除、access失効時は設定を暗黙変更せず、resolverが次候補へ進む。

### Authoritative admission

- `selectedRoom`と`admittedRoom`を分離する。Clientはhostへ署名済み`Join`を送り、host署名済みphysics snapshotにlocal avatar bodyが含まれるまでscene、presence、音声、操作入力を開始しない。
- Admission確認snapshotはcontrol-plane receiptとして通常のstream snapshot頻度制御から分離し、直前の古いsnapshotを再利用しない。これによりhost側参加だけが成立するghost admissionを防ぐ。
- Owner-device hostではapp-apiがcurrent Spatial Context accessとdestination ownerからのblockを再評価する。Community Node hostではparticipant、Spatial Context、target owner、有効期限へ束縛した`DomeSpatialAccessProofV1`を必須とする。
- Active lease/session、Instance generation、manifest/assets、capacity、resource budgetは既存hosting session境界で再検証する。失敗した候補はparticipantやavatar bodyを作らず、次候補または選択一覧へfallbackする。
- Metaverseは実験機能のため、変更したsession wireに旧schemaとの後方互換を設けない。

### Safe spawn

- Hostは`DomeInstanceManifestV1.default_spawn`を最初に試し、そこから150 cm間隔の固定25候補を決定的な順序で評価する。
- Clientはthree-vrmでavatarをpreloadし、VRM/GLTFの明示colliderを優先する。明示情報が無ければ描画objectのbounding boxを包含するcapsuleを生成してrequestへ渡す。Assetを読めない場合だけhostがbounds `[-25, 0, -25]..[25, 180, 25]`からfallback capsuleを生成する。Colliderのlocal centerはphysics shapeへ反映する。
- 各候補は25 cmの水平安全余白を加えたAABBが固定半球Dome内に収まり、既存avatar、persistent prop、guest propのcollider AABBと重ならない場合だけ採用する。
- 候補が全て塞がっている場合は`DOME_ENTRY_NO_SAFE_SPAWN`を返し、participant setとphysics body setを変更しない。このprimitiveを将来のReturn Homeからも再利用する。

## Feature Data Classification

- Durable: private channel current policyのentry Instance参照。
- Local-only: author + Spatial Context別の最終訪問Instance。
- Transient: candidate evaluation、access proof、admission中状態、spawn候補、初期snapshot。
- Canonical source: channel設定はowner-signed channel policy、accessはtopic subscription/current channel participant、host/sessionとspawnはactive host runtime。
- Retention: access proofは10秒、admission/spawn計算はsession処理中だけ保持する。最終訪問履歴はlocal storageから利用者が消去できる。

## Consequences

- 優先Domeが利用不能でも、access不能な詳細やparticipant identityを公開せずに次候補へ進める。
- Avatarはhostが参加と初期transformを同時に確定した後だけ見えるため、拒否時の一瞬表示とghost participantを避けられる。
- Dome固有のmembership、role、moderatorは追加しない。
