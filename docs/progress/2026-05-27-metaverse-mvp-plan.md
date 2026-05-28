# Goal: kukuri に P2P Room Metaverse MVP を実装する

あなたは `KingYoSun/kukuri` リポジトリで作業します。

このGoalの目的は、kukuri の既存の topic-first P2P アーキテクチャの上に、最小限動作する P2P Room Metaverse MVP を追加することです。

これは VRChat クローンでも、MMOでも、完成された分散型メタバースでもありません。
目的は、kukuri が「小規模な3Dソーシャルルーム」を扱えることを検証する縦薄切りのMVPです。

今回の重要方針として、複数room discovery UIはゼロから作らず、既存の game room / state共有テスト実装を応用してください。

既存の game room 実装には、以下の再利用候補があります。

- topic単位の room list / projection
- `list_game_rooms` / `list_game_rooms_scoped`
- `create_game_room`
- `update_game_room`
- docs上の state pointer
- blobs上の manifest payload
- projection store の room cache
- `SessionChanged` hint
- desktop shell の `gameRoomsByTopic`
- game section の refresh / loadTopics 導線
- game room panel / draft / card UI の考え方

Metaverse room は、既存 game room を置き換えるのではなく、game room の設計を拡張または横展開して実装してください。

---

## 背景

kukuri は topic-first P2P social app / protocol です。

既存設計では、以下の分離を尊重してください。

- `hints` は通知・同期トリガーであり、source of truth ではない
- `docs` は構造化された状態同期に使う
- `blobs` はメディアや大きなpayloadの同期に使う
- connectivity は static peer、seeded DHT discovery、community node assist によって補助される
- community node は bootstrap / auth / control-plane / connectivity assist の役割を持つが、ユーザーコンテンツの canonical store ではない
- topic / channel / live session / game room の既存概念と整合するように設計する

P2P Room Metaverse は、kukuri における「3D化された topic/channel/game-room 的な surface」として扱ってください。

---

## Existing Game Room Findings

作業前に、既存の game room 実装を確認してください。

特に以下を調査してください。

- `docs/adr/0006-game-room-data-classification.md`
- `crates/app-api/src/game.rs`
- `crates/app-api/src/service/live_game_support.rs`
- `crates/desktop-runtime/src/runtime/private_channels_game_api.rs`
- `apps/desktop/src/shell/store.ts`
- `apps/desktop/src/shell/useDesktopShellData.ts`
- `apps/desktop/src/shell/useDesktopShellDataEffects.ts`
- `apps/desktop/src/shell/useDesktopShellViewModels.ts`
- `apps/desktop/src/lib/api/types.ts`
- game room UI / extended components / mock API / harness scenarios

既存 game room の重要な設計は以下です。

- room state は Durable
- current state pointer は docs に置く
- manifest payload は blobs に置く
- topic replica に room state pointer を置く
- SQLite projection は room list cache として使う
- late joiner / restart restore は docs + blobs で成立させる
- room更新時に新しい manifest blob hash を払い出す
- `SessionChanged` hint によって更新を通知する
- desktop UI は topic単位の `gameRoomsByTopic` を持つ

Metaverse room discovery はこの設計を応用してください。

---

## Product Goal

ユーザー体験として、最低限以下を実現してください。

1. ユーザーが Tauri desktop client を起動する
2. ユーザーが metaverse section または game/metaverse section を開く
3. topic内の複数 metaverse room が一覧表示される
4. ユーザーが新しい metaverse room を作成できる
5. ユーザーが既存 metaverse room に参加できる
6. 参加したroomで3D viewportが表示される
7. 自分のVRMアバター、またはフォールバックアバターが表示される
8. キーボード操作で自分のアバターを移動できる
9. 同じroomにいる他peerが remote avatar として表示される
10. 自分の avatar transform が他peerへ同期される
11. room内でテキストチャットを送受信できる
12. room内に1つの共有オブジェクトを配置・移動できる
13. 共有オブジェクトの状態が他peerへ同期される
14. VRM / GLB などのassetは raw bytes をイベントに埋め込まず、blob-backed asset reference として扱う
15. room metadata と persistent room state は docs または既存 game room manifest pattern で同期・復元できる
16. facilitator/community node は optional だが、存在すると bootstrap / backfill / pinning / connectivity assist により安定する

---

## Non-goals

このGoalでは以下を実装しないでください。

- VRChat相当の完成度
- MMO
- 大規模同時接続
- フル機能のワールドエディタ
- 任意UGC scripting
- 物理演算の厳密同期
- PvP
- voice chat
- WebRTC / SFU
- アイテム経済
- NFT / デジタル所有権
- 課金
- moderation UI
- asset safety scanning
- mobile support
- browser-only P2P support
- 複雑な権限管理
- 高度なCRDT競合解決

ただし、複数room discovery UIは non-goal ではありません。
既存 game room の一覧・作成・参加UIを応用して、MVPに含めてください。

MVPでは、複数room一覧 / 1 room参加 / 少人数peer / 1 shared object / basic avatar sync に集中してください。

---

## Target Architecture

概念的には以下の対応関係で実装してください。

- Metaverse Room
  - kukuri topic/channel/game-room style space
  - 既存 game room の拡張または兄弟機能として扱う

- Room Discovery
  - 既存 game room list/projection を応用する
  - topic単位で metaverse room を一覧する
  - docs pointer + blob manifest + SQLite projection cache の形を優先する

- Room Metadata
  - docs state pointer + blob manifest
  - または既存 game room manifest を拡張した metaverse manifest

- Room presence
  - ephemeral peer/session state
  - avatar transformとは分離する

- Avatar asset
  - blob-backed VRM asset reference

- Room object asset
  - blob-backed asset reference or built-in primitive fallback

- Avatar transform
  - ephemeral signed room event / hint / lightweight P2P message
  - 高頻度なので docs/blobs に直接書き込まない

- Text chat
  - room-scoped signed message
  - 既存 topic/channel message primitive を再利用できるなら優先

- Persistent room object state
  - docs state pointer + manifest blob
  - または docs/op-log backed persistent state

- Facilitator/community node
  - bootstrap / connectivity / pinning / backfill helper
  - must not become canonical authority

---

## Data Model Direction

既存 game room をそのまま破壊しないでください。

推奨方針はどちらかです。

### Option A: Existing Game Room Manifest を拡張する

既存 `GameRoomManifestBlobV1` に `room_kind` または `surface_kind` を追加し、通常game roomとmetaverse roomを区別する。

例:

- room_kind: "score_game" | "metaverse_room"
- metaverse metadata は optional field にする
- 既存 score/status game room UI を壊さない
- `list_game_rooms_scoped` の互換性を維持する
- UI側で metaverse room だけをfilterして discoveryに出す

この方針は既存API再利用が大きいが、既存型の互換性に注意してください。

### Option B: Metaverse Room を Game Room の兄弟モデルとして追加する

新しい `MetaverseRoomManifestBlobV1` / `MetaverseRoomStateDocV1` / projection row を追加する。

例:

- `list_metaverse_rooms_scoped`
- `create_metaverse_room_in_channel`
- `update_metaverse_room`
- `persist_metaverse_room_manifest`
- `fetch_metaverse_room_state_and_manifest`
- `metaverseRoomsByTopic`

この方針は型が明確だが、実装量が増えます。

### Recommendation

MVPでは Option A を優先検討してください。

理由:

- 既存の game room discovery / projection / UI refresh 導線を使いやすい
- 既に docs pointer + blob manifest + projection cache のパターンがある
- late joiner / restart restore の設計が存在する
- desktop shell に `gameRoomsByTopic` があり、複数room一覧の基盤がある

ただし、Option A が既存game roomの互換性を壊しそうな場合は Option B に切り替えてください。

どちらを選んだか、理由を最終報告に明記してください。

---

## MVP State Model

既存の型・命名規則に合わせてよいですが、最低限以下のような概念を導入してください。

### MetaverseRoomId

TypeScriptイメージ:

type MetaverseRoomId = string;

### MetaverseRoomMetadata

TypeScriptイメージ:

type MetaverseRoomMetadata = {
  roomId: MetaverseRoomId;
  title: string;
  description?: string;
  topicId: string;
  channelId?: string | null;
  ownerPubkey: string;
  status: "waiting" | "active" | "ended";
  roomKind: "metaverse_room";
  createdAt: number;
  updatedAt: number;
  worldVersion: number;
  maxPeers?: number;
};

### AssetRef

TypeScriptイメージ:

type AssetRef = {
  kind: "vrm" | "glb" | "texture" | "other";
  blobHash: string;
  mimeType?: string;
  sizeBytes?: number;
  name?: string;
};

### RoomPresence

TypeScriptイメージ:

type RoomPresence = {
  roomId: MetaverseRoomId;
  peerId: string;
  displayName?: string;
  avatarAssetRef?: AssetRef;
  joinedAt: number;
  lastSeenAt: number;
};

### AvatarTransform

TypeScriptイメージ:

type AvatarTransform = {
  roomId: MetaverseRoomId;
  peerId: string;
  seq: number;
  position: [number, number, number];
  rotation: [number, number, number];
  animation?: "idle" | "walk" | "run";
  sentAt: number;
};

### RoomChatMessage

TypeScriptイメージ:

type RoomChatMessage = {
  roomId: MetaverseRoomId;
  messageId: string;
  authorPeerId: string;
  displayName?: string;
  body: string;
  createdAt: number;
};

### SharedRoomObject

MVPでは1つだけ共有オブジェクトをサポートしてください。

TypeScriptイメージ:

type SharedRoomObject = {
  objectId: "mvp-object-1";
  assetRef?: AssetRef;
  primitiveFallback?: "cube" | "sphere";
  position: [number, number, number];
  rotation: [number, number, number];
  scale: [number, number, number];
  updatedBy: string;
  updatedAt: number;
};

### MetaverseRoomManifest

既存 GameRoomManifest を拡張する場合も、新規型にする場合も、manifest payload に以下の情報を含めてください。

TypeScriptイメージ:

type MetaverseRoomManifest = {
  roomId: MetaverseRoomId;
  topicId: string;
  channelId?: string | null;
  ownerPubkey: string;
  title: string;
  description?: string;
  status: "waiting" | "active" | "ended";
  roomKind: "metaverse_room";
  worldVersion: number;
  scene: {
    ground: "default";
    sharedObject: SharedRoomObject;
  };
  defaultSpawn: {
    position: [number, number, number];
    rotation: [number, number, number];
  };
  assetRefs: AssetRef[];
  updatedAt: number;
};

### MetaverseRoomEvent

必要であれば、以下のようなroom event unionを導入してください。

TypeScriptイメージ:

type MetaverseRoomEvent =
  | { type: "presence.join"; presence: RoomPresence }
  | { type: "presence.leave"; roomId: MetaverseRoomId; peerId: string; leftAt: number }
  | { type: "avatar.transform"; transform: AvatarTransform }
  | { type: "chat.message"; message: RoomChatMessage }
  | { type: "object.update"; object: SharedRoomObject }
  | { type: "room.metadata.update"; metadata: MetaverseRoomMetadata };

---

## Multiple Room Discovery UI Requirements

複数room discovery UIをMVPに含めてください。

### Discovery UI の目的

ユーザーが topic 内の metaverse room を一覧し、作成し、参加できるようにします。

最低限のUI:

- metaverse room list
- create room button
- room card
- room title
- room description
- room status
- owner / host label
- updated_at
- audience/channel label
- join/open button
- local/remote availability status
- debug表示として manifest blob hash または persistence status を確認できる導線

既存 game room panel/card がある場合は、それを流用・拡張してください。

### Room List Source

room list は以下を優先してください。

- 既存 `gameRoomsByTopic`
- 既存 `list_game_rooms_scoped`
- 既存 projection store の game room cache
- room_kind / surface_kind による metaverse room filtering

Option B を選んだ場合は、新しく以下を追加しても構いません。

- `metaverseRoomsByTopic`
- `list_metaverse_rooms_scoped`
- metaverse room projection cache

ただしMVPでは、既存 game room list/projection を応用できるなら、その方針を優先してください。

### Create Room

create room は既存 `create_game_room` の流れを参考にしてください。

最低限:

- title required
- description optional
- room id auto generated
- owner is current author
- status starts as waiting or active
- manifest blob is stored
- docs pointer is updated
- projection cache is updated
- SessionChanged または metaverse-specific changed hint をpublishする

既存 `create_game_room_in_channel` を拡張する場合は、通常game room作成を壊さないようにしてください。

### Join/Open Room

room card の join/open button から、metaverse room view を開けるようにしてください。

join/open 時に最低限以下を行ってください。

- selected metaverse room id を shell state に保存
- room manifest を docs + blobs からfetch
- 3D scene を開く
- local presence を発行
- avatar fallback を表示
- room event transport を開始
- known peers / sync status をdebug panelに表示

### Refresh / Backfill

既存 game section は active section が `game` のときに loadTopics を呼ぶ導線があります。
metaverse section を新設する場合は、同様に active section が `metaverse` のときに topic/room list がrefreshされるようにしてください。

game section に統合する場合は、game room list の中で `roomKind === "metaverse_room"` を metaverse room card として表示してください。

late joiner / restart restore は既存 game room ADRの方針に従い、docs state + manifest blob で成立させてください。

---

## Implementation Requirements

### 1. Tauri Desktop Client

既存の Tauri desktop client に、metaverse room discovery UI と metaverse room view を追加してください。

最低限必要なUI:

- metaverse room list / discovery panel
- create metaverse room form
- room card
- join/open room button
- 3D viewport
- local avatar
- remote avatars
- simple movement controls
- text chat panel
- shared object control
- debug/status panel

debug/status panel には最低限以下を表示してください。

- current topic id
- current room id
- local peer id
- known/connected peers
- local avatar asset state
- last sent transform seq
- last received transform timestamp
- room state sync status
- blob asset resolve status
- docs/op-log persistence status
- current manifest blob hash if available
- facilitator/community node assist status if available

UIの完成度よりも、状態が見えてデバッグ可能であることを優先してください。

---

### 2. three.js / three-vrm Rendering

3D描画には以下を使ってください。

- `three.js`
- `three-vrm`

最低限のsceneを作ってください。

- camera
- light
- ground plane
- local avatar
- remote avatars
- shared room object
- fallback avatar
- fallback object

VRMロードに失敗した場合、room全体が壊れないようにしてください。
その場合は capsule / box / simple mesh などのfallback avatarを表示してください。

---

### 3. Local Avatar

MVPでは以下のどちらかを実装してください。

優先:

- ユーザーがローカル `.vrm` ファイルを選択できる
- 選択したVRMを blob-backed asset として取り込む
- room presence で avatarAssetRef を通知する
- remote peer が asset ref から取得できる場合はVRMを表示する

難しい場合の許容fallback:

- bundled fallback avatar または primitive avatar を使う
- ただし `AssetRef` の型・導線は残し、将来VRM assetへ差し替えられるようにする

VRM asset UXの完成度でMVPを止めないでください。

---

### 4. Avatar Movement

local avatar の移動を実装してください。

最低限:

- WASD または arrow keys で移動
- 回転は移動方向に向く、またはyaw操作
- `idle` / `walk` 程度の簡易 animation state
- transform state を内部storeに反映
- transform state をroom transportへ送信

送信頻度は抑えてください。

目安:

avatar transform broadcast <= 10 Hz

docs/blobsに高頻度transformを書き込まないでください。
avatar transform は基本的に ephemeral state として扱います。

---

### 5. Remote Avatar Sync

他peerから受け取った `AvatarTransform` をremote avatarに反映してください。

最低限:

- peerごとにremote avatar entityを作る
- transform更新で位置・回転を更新する
- しばらく更新がないpeerはdebug上でstale表示する
- presence leave / timeout で非表示またはstale化する

補間はできれば実装してください。
ただしMVPでは、単純な位置更新でも可です。

---

### 6. Text Chat

room-scoped text chat を実装してください。

最低限:

- message input
- send
- receive
- message list
- author/display name
- timestamp

既存の kukuri topic/channel message primitive を再利用できる場合は、それを優先してください。
room chat のために完全に別系統のチャット基盤を作らないでください。

ただし既存基盤との接続が大きすぎる場合は、MVP用の `RoomChatMessage` abstraction を作り、後で既存message primitiveへ統合しやすい境界にしてください。

---

### 7. Shared Room Object

room内に1つだけ共有オブジェクトを実装してください。

最低限:

- cube または sphere が表示される
- local user が配置/移動できる
- object state が他peerへ同期される
- object state が restart 後に復元されることを目指す
- 永続化が難しい場合は、その制約を明確にdocsに記載する

競合解決はMVPでは単純で構いません。

推奨:

last-write-wins by updatedAt
tie-breaker by peerId or eventId

この制約はコードコメントとドキュメントに明記してください。

---

### 8. Blob-backed Assets

VRM / GLB / texture などの大きなassetは、room eventやchat messageにraw bytesとして埋め込まないでください。

MVPでは以下の流れにしてください。

asset bytes
-> blobs / iroh-blobs-compatible storage
-> AssetRef(blobHash, mimeType, sizeBytes, kind)
-> room event / presence に AssetRef だけを載せる
-> remote peer が blobHash からasset解決
-> 失敗時はfallback表示

最低限、型と導線が blob-backed になっていることを重視してください。
完全なasset pinning/backfillは後続で構いません。

---

### 9. Room Metadata / Persistent State

room metadata と shared object state を永続化・同期してください。

優先:

- 既存 game room の docs pointer + manifest blob pattern を使う
- topic replica に metaverse room state pointer を置く
- manifest本体を blobs に保存する
- projection cache で room list を高速表示する

難しい場合:

- simple signed op-log abstraction を実装する
- 後でdocs backendへ差し替えられるinterfaceにする

最低限永続化したいもの:

- room metadata
- shared object state
- avatar asset ref per peer
- assetRefs
- 必要なら簡易chat history

avatar transform は高頻度ephemeral stateなので、永続化対象にしないでください。

---

### 10. Facilitator / Community Node

facilitator/community node は optional です。

MVPは facilitator node なしでも、static peer / local multi-client などで可能な範囲動くようにしてください。

facilitator node が存在する場合は、以下を補助してよいです。

- room peer bootstrap
- room metadata advertisement
- docs backfill
- blob pinning
- blob backfill
- connectivity assist

ただし、community node を user content の canonical store や authoritative game server にしないでください。

このMVPにおけるcommunity nodeは、あくまで facilitator です。

---

## Suggested Internal Boundaries

既存構造に合わせて調整して構いませんが、以下のような境界を意識してください。

- MetaverseRoomDiscoveryPanel
  - topic内のmetaverse room一覧
  - create room
  - join/open room
  - existing game room panel/card の再利用候補

- MetaverseRoomCard
  - title
  - description
  - status
  - owner
  - updated_at
  - audience label
  - open/join action

- MetaverseRoomView
  - selected room のUI root
  - 3D scene
  - chat
  - debug panel

- MetaverseScene
  - three.js scene lifecycle
  - camera / light / ground
  - avatar renderers
  - shared object renderer

- AvatarRenderer
  - local / remote avatar mesh
  - VRM load or fallback avatar

- RoomPresenceStore
  - joined peers
  - local presence
  - stale detection

- RoomTransformStore
  - local transform
  - remote transforms
  - seq / timestamp handling

- RoomChatStore
  - room messages
  - send / receive

- RoomObjectStore
  - shared object state
  - LWW update

- RoomAssetStore
  - AssetRef
  - blob import
  - blob resolve
  - fallback handling

- RoomStateStore
  - docs pointer + manifest blob
  - or op-log backed metadata/object persistence

- RoomEventTransport
  - send/receive metaverse room events
  - maps to kukuri P2P/hints/connectivity layer

避けること:

- topic timeline code に3D固有ロジックを直接混ぜる
- blobs にephemeral transformを書き込む
- docs に10Hz transformを書き込む
- community node をcanonical authorityとして扱う
- MVPのために既存プロトコル境界を壊す
- 既存 game room のscore/status roomを壊す

---

## Implementation Milestones

小さな差分で段階的に実装してください。

### Milestone 0: Existing Game Room Investigation

- 既存 game room の ADR / service / runtime / frontend store / UI / tests を読む
- docs pointer + manifest blob + projection cache の流れを把握する
- `list_game_rooms_scoped` と `gameRoomsByTopic` のUI導線を把握する
- Option A / Option B のどちらで実装するか決める
- 決定理由を実装コメントまたはdocsに残す

### Milestone 1: Metaverse Room Discovery Skeleton

- metaverse room discovery panel を追加
- 既存 game room list/projection を使って room card を表示
- roomKind / surfaceKind によるfilterを導入
- create room button を追加
- open/join room action を追加
- selected metaverse room id を shell state に保存

### Milestone 2: Metaverse Room Manifest

- MetaverseRoomManifest を定義
- 既存 GameRoomManifest 拡張または新規manifestとして保存
- docs pointer + blob manifest の保存・取得を実装
- projection cache に discovery用の最低限情報を反映
- restart後に room list が復元されることを確認

### Milestone 3: UI Skeleton

- selected room の metaverse room view を追加
- debug panel を追加
- three.js sceneを起動
- ground plane / camera / light / fallback local avatar を表示

### Milestone 4: Local Movement

- local avatar transform store を追加
- WASD/arrow key movement を追加
- avatar position/rotation をsceneに反映
- debug panelにtransformを表示

### Milestone 5: Room Event Model

- room event types を追加
- local event bus/store を追加
- transform / chat / object update の抽象化を追加
- まだP2Pに繋がっていなくてもlocal loopbackで動くようにする

### Milestone 6: P2P Room Transport

- 既存kukuri connectivity/hints/event系に接続
- 同じroom内peerへイベントを送受信
- remote avatar表示
- remote transform反映
- room chat送受信

### Milestone 7: Blob Asset Flow

- AssetRef を追加
- VRMまたはfallback assetの blob-backed 導線を作る
- presence で avatarAssetRef を共有
- remote peer が asset resolve に失敗してもfallback表示

### Milestone 8: Persistent Room Object State

- shared object state を manifest または docs/op-log に保存
- object update で manifest blob を更新するか、将来拡張しやすいop-logを使う
- restart後にroom stateを復元
- LWW制約を明記

### Milestone 9: Documentation and Manual Test

- `docs/metaverse/p2p-room-mvp.md` を追加
- run instructions を書く
- room discovery の確認手順を書く
- create/open/join room の確認手順を書く
- two-client manual test を書く
- known limitations を書く
- next steps を書く

---

## Documentation Requirements

以下のドキュメントを追加してください。

`docs/metaverse/p2p-room-mvp.md`

内容には最低限以下を含めてください。

- このMVPの目的
- 既存 game room 実装をどう応用したか
- Option A / Option B のどちらを選んだか
- 実装された機能
- 実装していない機能
- multiple room discovery UI の仕組み
- create room / open room / join room の流れ
- room state と kukuri primitives の対応
- hints / docs / blobs / connectivity の使い分け
- facilitator/community node が optional であること
- 起動方法
- 1クライアントでの確認方法
- 2クライアントでの手動確認方法
- VRM / asset flow の確認方法
- shared object sync の確認方法
- restart restore の確認方法
- known limitations
- next implementation steps

---

## Validation Requirements

最低限、以下を確認してください。

- TypeScript check が通る
- Rust check が通る
- 影響範囲で既存テストが壊れていない
- `cargo xtask check` または既存の標準checkコマンドが利用可能
- desktop app が起動できる
- metaverse room discovery UI を開ける
- topic内の複数roomが一覧表示される
- metaverse room を作成できる
- room card から open/join できる
- metaverse room view を開ける
- local avatar/fallback avatar が表示される
- local avatar を移動できる
- chat message を送信できる
- shared object を配置/移動できる
- local loopback または2クライアントでroom eventの送受信を確認できる
- remote avatar表示が可能、または未接続時のfallback/debug表示が明確
- docs/op-log persistence の状態がdebug panelまたはログで確認できる
- restart後に room list と shared object state が復元される
- 既存 game room のscore/status UIが壊れていない

multi-peer自動テストが重い場合は、手動テスト手順を必ずdocsに残してください。

---

## Acceptance Criteria

このGoalは以下を満たしたら完了です。

1. Tauri desktop app に metaverse room discovery UI が追加されている
2. topic内の複数 metaverse room を一覧できる
3. metaverse room を作成できる
4. room card から selected room を開ける
5. selected room の 3D scene が表示される
6. local avatar がVRMまたはfallbackで表示される
7. local avatar を移動できる
8. avatar transform が room event として送信される
9. 他peerのtransformを受信した場合、remote avatarとして表示される
10. room-scoped text chat が動く
11. shared object を1つ配置/移動できる
12. shared object state が同期対象になっている
13. assetはraw bytesではなく AssetRef / blob-backed flowで扱われる
14. room metadata と shared object state が docs pointer + manifest blob またはop-logで永続化対象になっている
15. facilitator/community node は optional であり、canonical authorityになっていない
16. 既存のkukuri check/test/dev flowを破壊していない
17. 既存 game room のscore/status roomが壊れていない
18. `docs/metaverse/p2p-room-mvp.md` に実行・検証手順がある
19. できていること、fallback/mock、未実装、既知の制約が明確に記録されている

---

## Engineering Policy

実装では以下を優先してください。

- MVPとして動く縦薄切り
- 既存 game room 実装の再利用
- 既存kukuri primitiveの再利用
- 将来差し替え可能な小さな抽象化
- debugしやすいUI
- fallbackに強い実装
- docs/blobs/hints/connectivityの役割分離
- community nodeをcanonical storeにしない設計
- 高頻度ephemeral stateと永続stateの分離
- 既存 game room UI/API/test を壊さない

判断に迷った場合は、以下を優先してください。

- room discovery = existing game room list/projection を応用
- room metadata = docs pointer + manifest blob
- hints = notification / sync trigger
- docs = structured persistent state pointer
- blobs = manifest payload and large assets
- connectivity = peer discovery and transport assist
- community nodes = facilitator, not canonical authority
- avatar transform = ephemeral
- room metadata/object state = persistent
- asset bytes = blob-backed

---

## Final Report

作業完了時に、以下を報告してください。

- 変更したファイル一覧
- 実装した機能
- 既存 game room 実装をどう応用したか
- Option A / Option B のどちらを選んだか
- アーキテクチャ概要
- 既存kukuri primitiveとの対応
- room discovery UI の使い方
- 実行方法
- 検証方法
- 動作確認結果
- fallback/mockとして残っている箇所
- known limitations
- 次に実装すべきこと
