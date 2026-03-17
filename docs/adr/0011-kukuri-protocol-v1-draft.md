# kukuri Protocol v1 正式仕様ドラフト

- Status: Draft
- Version: v1-draft
- Date: 2026-03-17
- Related ADR:
  - ADR: kukuri Protocol v1 の境界定義（何を残し、何を切るか）

---

## 1. Overview

`kukuri Protocol v1` は、署名付き envelope を最小共通単位として採用しつつ、実際のデータ流通・状態同期・接続性確保を以下の複数レイヤに分離する P2P / state-sync / blob-sync 指向プロトコルである。

- Identity / Signed Envelope
- Hint / Notification
- State / Structured Sync
- Blob / Large Object Transport
- Discovery / Routing / Connectivity

本仕様は、relay-first なイベント配送を中核とせず、**通知・状態・実体・接続性を明確に分離する**ことを前提とする。

---

## 2. Goals

`kukuri Protocol v1` の目的は以下である。

1. テキスト、画像、動画、音声、live、game を含む多様な流通モデルを統一的に扱う
2. P2P 環境における可用性と拡張性を両立する
3. gossip をヒント配信に限定し、永続状態や実体配送と切り離す
4. docs / blobs を正規データソースとして扱う
5. community / topic / room / session を第一級概念として扱う
6. 必要最小限の署名付きイベント互換性を維持する

---

## 3. Non-Goals

本仕様は以下を目的としない。

- Nostr relay との全面互換
- REQ / EVENT / CLOSE を用いた内部同期
- 単一イベントのみで全データ流通を完結させること
- relay をシステムの唯一または主要な truth source とすること
- すべての state を stateless なイベント列だけで復元すること

---

## 4. Terminology

### 4.1 Identity
鍵ペアに基づく主体識別子。ユーザー、ノード、サービス主体などを識別する。

### 4.2 Envelope
署名付きメタデータ単位。作成者、時刻、型、タグ、内容、署名を含む最小オブジェクト。

### 4.3 Object
kukuri 上で扱われる論理オブジェクト。投稿、コメント、画像、動画、room、community、live session、game session などを含む。

### 4.4 Pointer
Object または Blob を参照するための解決情報。doc path、blob hash、namespace、locator などを含みうる。

### 4.5 Hint
新着通知、購読候補、参加候補、pointer 告知など、同期の起点となる軽量メッセージ。

### 4.6 State
ある object 群や namespace に関する最新状態。投稿一覧、membership、session metadata、room state など。

### 4.7 Blob
大容量またはバイナリ実体。画像、動画、音声、live asset、game asset など。

### 4.8 Topic
Hint や state 同期のための論理チャネル。community / room / thread / live / game 単位で構成されうる。

### 4.9 Community Node
接続補助、bootstrap、discovery assist、policy assist、optional gateway として振る舞うノード。

### 4.10 Authority
ある namespace または state に関して書き込み権限や検証責務を持つ主体。

---

## 5. Protocol Model

### 5.1 Layer Model

#### Layer 1: Identity / Signed Envelope
責務:
- 鍵管理
- 署名
- 作成者証明
- 最小メタ情報の運搬

#### Layer 2: Hint / Notification
責務:
- 新着通知
- topic discovery
- pointer 告知
- 接続補助情報の伝搬

#### Layer 3: State / Structured Sync
責務:
- 正規 state の保持と同期
- 投稿・スレッド・membership・session の構造化表現
- object index 管理

#### Layer 4: Blob / Large Object Transport
責務:
- 実体の保存と取得
- hash / locator ベースの取得
- 大容量データ流通

#### Layer 5: Discovery / Routing / Connectivity
責務:
- peer discovery
- DHT
- bootstrap
- NAT traversal assist
- relay assist

---

## 6. Data Source Policy

`kukuri Protocol v1` では、データ種別ごとに正規 source of truth を定義する。

| Data Type | Primary Source | Secondary Role |
|---|---|---|
| Hint / Notification | gossip | キャッシュ可、永続真実ではない |
| Structured State | docs | ローカルキャッシュ可 |
| Binary / Large Object | blobs | ローカルキャッシュ可 |
| Discovery / Routing | DHT / static-peer / relay assist | ヒントとして再配布可 |
| Identity Metadata | docs または署名付き envelope | hint で告知可 |

### 6.1 Rule
- hint は truth source ではない
- state は docs を正とする
- blob 実体は blobs を正とする
- connectivity 情報は DHT / static-peer / relay assist を正とする
- envelope 単体は truth ではなく、state / blob を参照するための署名付きメタデータである

---

## 7. Identity and Envelope

### 7.1 Identity

Identity は以下を持つ。

- public key
- private key
- optional local alias
- optional profile metadata
- optional delegated roles

Identity は主体識別の最小単位であり、すべての書き込み操作は署名可能でなければならない。

---

### 7.2 Envelope Schema

Envelope の論理 schema は以下とする。

```ts
type KukuriEnvelope = {
  id: string
  pubkey: string
  created_at: number
  kind: number | string
  tags: KukuriTag[]
  content: string
  sig: string
}
```

### Field Semantics

- `id`: envelope 全体の canonical serialization から導出される識別子
- `pubkey`: 作成者公開鍵
- `created_at`: 作成時刻（unix timestamp）
- `kind`: envelope の論理種別
- `tags`: 構造化メタデータ
- `content`: 小さな本文または補助 JSON
- `sig`: 署名

#### 7.2.1 Canonicalization
Envelope の `id` および `sig` は canonical serialization に基づいて生成される。

v1 では canonicalization の詳細実装は既存実装互換を優先するが、将来バージョンでは別 ADR / spec に切り出して固定する。

---

### 7.3 Tag Model

Tag は可変長配列または key-value 的表現を取れるが、Protocol v1 では意味論を以下に分類する。

#### 7.3.1 Reference Tags
他 object / envelope / blob / community / session を参照する。

例:
- parent
- reply_to
- root
- target
- object_ref

#### 7.3.2 Pointer Tags
実体や state locator を示す。

例:
- doc_namespace
- doc_key
- blob_hash
- blob_mime
- locator
- manifest_ref

#### 7.3.3 Context Tags
community, room, topic, thread, live, game などの文脈を示す。

例:
- community
- room
- thread
- live_session
- game_session
- topic

#### 7.3.4 Capability / Policy Tags
公開範囲や権限、期待する同期方式などを示す。

例:
- visibility
- permission
- policy
- retention
- authority

#### 7.3.5 Media Tags
メディア実体の説明情報を持つ。

例:
- mime
- width
- height
- duration
- codec
- size

---

## 8. Object Model

`kukuri Protocol v1` は envelope を直接 UX 単位として扱わず、envelope を用いて object を定義する。

### 8.1 Core Object Types

最低限、以下の object type を持つ。

- identity-profile
- post
- comment
- reaction
- repost / boost
- media-manifest
- room
- community
- membership
- thread
- live-session
- game-session
- system-announcement

---

### 8.2 Object Envelope Relation

1つの object は 1 つ以上の envelope を持ちうる。

例:
- post 作成 envelope
- post 編集 envelope
- visibility 更新 envelope
- media 追加 envelope

ただし truth source は envelope 列そのものではなく、docs 上で正規化された state とする。

---

### 8.3 Post Object

```ts
type KukuriPost = {
  object_id: string
  author_pubkey: string
  created_at: number
  updated_at: number
  body: string
  media_refs: string[]
  thread_id?: string
  community_id?: string
  room_id?: string
  visibility: "public" | "community" | "room" | "private"
  reply_to?: string
  root_post_id?: string
  status: "active" | "edited" | "deleted" | "tombstoned"
}
```

---

### 8.4 Media Manifest Object

```ts
type KukuriMediaManifest = {
  object_id: string
  owner_pubkey: string
  created_at: number
  items: KukuriMediaItem[]
}

type KukuriMediaItem = {
  blob_hash: string
  mime: string
  size: number
  width?: number
  height?: number
  duration_ms?: number
  codec?: string
  thumbnail_blob_hash?: string
}
```

Media 実体は blobs にあり、manifest は docs 上に置く。

---

### 8.5 Community Object

```ts
type KukuriCommunity = {
  community_id: string
  owner_pubkey: string
  created_at: number
  updated_at: number
  title: string
  description?: string
  policy_ref?: string
  topic_namespace: string
  membership_mode: "open" | "approval" | "invite-only"
  status: "active" | "archived"
}
```

---

### 8.6 Membership Object

```ts
type KukuriMembership = {
  membership_id: string
  community_id: string
  subject_pubkey: string
  role: "owner" | "admin" | "moderator" | "member" | "guest"
  status: "active" | "pending" | "revoked" | "left"
  updated_at: number
  granted_by?: string
}
```

---

### 8.7 Live Session Object

```ts
type KukuriLiveSession = {
  session_id: string
  owner_pubkey: string
  created_at: number
  updated_at: number
  title?: string
  state: "scheduled" | "live" | "paused" | "ended"
  community_id?: string
  room_id?: string
  manifest_ref?: string
  participant_topic?: string
}
```

---

### 8.8 Game Session Object

```ts
type KukuriGameSession = {
  session_id: string
  owner_pubkey: string
  created_at: number
  updated_at: number
  title?: string
  state: "waiting" | "running" | "paused" | "ended"
  rules_ref?: string
  asset_manifest_ref?: string
  participant_topic?: string
}
```

---

## 9. Node Roles

### 9.1 User Node
ユーザー端末またはクライアントノード。

責務:
- identity 保持
- envelope 作成・署名
- docs / blobs の読書き
- hint の送受信
- peer discovery 参加
- 必要 object のローカルキャッシュ

---

### 9.2 Community Node
接続補助・discoverability・policy assist を担う補助ノード。

責務:
- bootstrap endpoint 提供
- peer introduction
- topic participation 補助
- relay assist
- optional policy distribution
- optional bridge / gateway

非責務:
- 全データの恒久 truth source
- すべての state の単独 authoritative host

---

### 9.3 Bridge Node
外部プロトコルとの変換を担うノード。

責務:
- Nostr import / export
- external relay / feed bridging
- schema conversion
- foreign event normalization

Bridge Node は任意であり、core protocol の必須要素ではない。

---

### 9.4 Service Node
検索、サムネイル生成、トランスコード、moderation assist などの補助サービスノード。

責務:
- 派生データ生成
- index 生成
- policy evaluation assist
- AI / recommendation 補助

Service Node が生成するデータも、必要に応じて署名付き envelope または docs state として公開できる。

---

## 10. Namespace Model

docs / blobs / topics は namespace で整理される。

### 10.1 Namespace Types

- user namespace
- community namespace
- room namespace
- thread namespace
- live namespace
- game namespace
- system namespace

### 10.2 Example

```text
users/{pubkey}/profile
users/{pubkey}/posts/{object_id}
communities/{community_id}/meta
communities/{community_id}/posts/{object_id}
rooms/{room_id}/state
threads/{thread_id}/index
live/{session_id}/state
live/{session_id}/chat/{object_id}
games/{session_id}/state
```

namespace 命名規則は実装に依存しうるが、論理責務は本仕様に従う。

---

## 11. Sync Model

### 11.1 General Rule

同期は以下の順で発生する。

1. hint を受信する
2. pointer / locator を解決する
3. 必要 state を docs から同期する
4. 必要 blob を blobs から取得する
5. ローカル index / cache を更新する

---

### 11.2 Hint Flow

Hint は軽量であり、最低限以下を含みうる。

```ts
type KukuriHint = {
  hint_id: string
  topic: string
  emitted_at: number
  emitter_pubkey: string
  object_kind: string
  object_id?: string
  pointer_refs?: string[]
  summary?: string
  sig?: string
}
```

### Rule
- hint は loss-tolerant である
- hint は再取得可能な state / blob への入口である
- hint 不達でも後から同期できる設計でなければならない

---

### 11.3 State Sync Flow

#### Case A: New Post

1. user node が post object を作成
2. signed envelope を生成
3. post state を docs に書き込む
4. media があれば blobs にアップロードし manifest を docs に書く
5. hint を topic に publish
6. 受信側は hint をきっかけに docs / blobs を解決する

#### Case B: Membership Update

1. authority が membership object を更新
2. signed envelope で変更を証明
3. community namespace docs を更新
4. hint を community topic に流す
5. 受信側は membership state を再同期する

#### Case C: Live Session Update

1. owner または authority が live session state を更新
2. session manifest / presence / state を docs に反映
3. 必要 asset を blobs に配置
4. hint を session topic に流す
5. 参加ノードは最新 state と asset pointer を同期する

---

### 11.4 Blob Sync Flow

1. object / manifest が blob hash と mime を提示
2. recipient node は locator を解決
3. blob transport で取得
4. hash 検証
5. local cache へ格納
6. optional thumbnail / transcode を service node が補助可能

---

### 11.5 Recovery Flow

ノードは hint を取り逃しても、以下により recovery できるべきである。

- known namespace の docs resync
- thread / community / room index の再取得
- DHT / community node からの再 discovery
- direct peer retry
- manifest 再解決

---

## 12. Consistency Model

`kukuri Protocol v1` は強整合ではなく、用途別整合モデルを採用する。

### 12.1 Hint Layer
- at-most-once / best-effort
- 順序保証なし
- 重複受信ありうる

### 12.2 State Layer
- eventually consistent
- versioned merge 可能
- authority / policy によって conflict 解決戦略を上書き可

### 12.3 Blob Layer
- content-addressed consistency
- hash による完全性検証
- 実体は immutable を基本とする

---

## 13. Conflict Resolution

### 13.1 General
同一 object に複数更新が競合した場合、object type ごとに戦略を定義する。

#### 13.1.1 Post
- 編集系は `updated_at` と署名検証を前提に latest accepted write
- delete は tombstone を優先
- policy により復元不可設定を許可

#### 13.1.2 Membership
- authority 優先
- owner/admin/moderator の権限階層を評価
- 不正署名は拒否

#### 13.1.3 Community Metadata
- authority 優先
- 複数管理者系は policy document で決定

#### 13.1.4 Live / Game Session
- session owner または designated authority 優先
- ephemeral presence は merge 可
- state transition は許可された遷移のみ受理

---

## 14. Security Model

### 14.1 Signature Verification
すべての write 意図は署名可能でなければならない。

### 14.2 Hash Verification
blob 実体は取得後に hash 検証を必須とする。

### 14.3 Replay Protection
必要に応じて以下を組み合わせる。
- `created_at`
- monotonic version
- session sequence
- doc version
- nonce / ephemeral token

### 14.4 Trust Separation
- 接続補助ノードは identity authority ではない
- community node は truth source とは限らない
- bridge node は import/export 変換主体であり、原典 author ではない

### 14.5 Optional Encryption
private / restricted scope の object は今後の拡張で暗号化 payload を持てる。  
v1 では平文 / 参照制御を基本とし、E2EE は extension 扱いとする。

---

## 15. Visibility and Access Policy

visibility は object に付与される論理ポリシーである。

### 15.1 Visibility Types
- `public`
- `community`
- `room`
- `private`

### 15.2 Semantics
- `public`: 発見・取得が広く許可される
- `community`: community membership 前提
- `room`: room participation 前提
- `private`: 明示的共有主体のみ

### 15.3 Enforcement
visibility 自体は envelope または object metadata に乗るが、実際の enforcement は以下の組み合わせで行う。

- locator 非公開
- blob access token
- encrypted payload
- authority-checked replication
- service / gateway policy

---

## 16. Transport Expectations

本仕様は特定 transport に固定されないが、v1 の想定実装は以下である。

- hint: `iroh-gossip`
- state sync: `iroh-docs`
- blob sync: `iroh-blobs`
- discovery: DHT / static-peer / relay assist
- connectivity assist: `iroh-relay` / community-node

Protocol v1 は transport abstraction を保つが、初期実装は上記を参照実装とする。

---

## 17. Minimal Interop Subset

外部相互運用のため、以下を optional subset として残す。

- public/private key identity
- signed envelope 外形
- selected tag mapping
- selected object export as foreign event
- bridge-based import

### 17.1 Important Rule
interop subset は **外部接続のための adapter** であり、core state model を規定しない。

---

## 18. Example End-to-End Flows

### 18.1 Text Post with Image

1. author が post object を作成
2. image blob を upload
3. media manifest を作成
4. post docs state に body + media_ref を保存
5. envelope を作成し署名
6. topic に hint 配信
7. recipient は hint を受け取り docs state を同期
8. 必要に応じて blob を fetch
9. UI 表示

---

### 18.2 Community Join

1. user が membership request object を作成
2. community authority が承認
3. membership docs state を更新
4. community topic に hint 配信
5. user node は membership state を同期
6. room/topic 参加可能範囲を更新

---

### 18.3 Live Session Start

1. owner が live-session object を `scheduled` から `live` に更新
2. session docs state 更新
3. stream manifest / asset pointers を反映
4. participant topic に hint 配信
5. participant が docs / blobs / transport を解決
6. live state を追従

---

### 18.4 Recovery After Offline

1. user node が再接続
2. known communities / rooms / sessions の namespace を resync
3. missing object index を取得
4. 必要 blob を lazy fetch
5. DHT / community node から peer 情報を更新
6. ローカル state 再構成

---

## 19. Implementation Requirements

### 19.1 Required
実装は最低限以下を満たさなければならない。

- signed envelope の生成・検証
- docs を正とする state sync
- blobs を用いた large object transport
- hint を起点とする再取得可能な同期
- hint 不達からの recovery
- object visibility の基本解釈
- conflict resolution の最低1戦略

### 19.2 Recommended
- local cache / index
- deduplication
- retry / backoff
- background resync
- manifest validation
- membership-aware fetch policy

### 19.3 Optional
- Nostr bridge
- advanced moderation
- AI assisted indexing
- encrypted payload extensions
- media transcoding pipeline
- search node / recommendation node

---

## 20. Versioning

### 20.1 Protocol Version
実装は protocol version を明示しなければならない。

例:
- `kukuri-protocol: v1`
- `supported-extensions: [...]`

### 20.2 Extension Mechanism
v1 の外で追加する機能は extension として表現する。

例:
- `ext.e2ee`
- `ext.nostr-bridge`
- `ext.realtime-presence`
- `ext.moderation-policy`
- `ext.game-realtime-sync`

extension は core semantics を壊してはならない。

---

## 21. Open Questions

以下は v1 正式化までに詰めるべき論点である。

1. canonical serialization をどこまで Nostr 互換に固定するか
2. `kind` を数値中心で保つか、文字列種別を許容するか
3. docs 上の versioning / merge 戦略を object 別にどう固定するか
4. private/community scope での暗号化方式をどう定義するか
5. community node の policy assist 範囲をどこまで protocol 化するか
6. presence / ephemeral event を core に含めるか extension にするか
7. live / game の低遅延同期を docs 外 extension としてどう切り出すか

---

## 22. Summary

`kukuri Protocol v1` は、署名付き envelope を最小互換層として維持しつつ、

- hint は通知
- docs は状態
- blobs は実体
- DHT / static-peer / relay assist は接続性

として責務分離するプロトコルである。

これにより kukuri は、relay-first なイベント配送プロトコルではなく、**community / topic / session を中心とする P2P state-sync / blob-sync プロトコル**として成立する。

---
