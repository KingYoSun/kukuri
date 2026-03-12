# kukuri 新 canonical flow 設計書
Version: draft-0.1  
Status: Proposal  
Scope: `next/` 再構築系 / MVP直後から導入可能な新同期基盤  
Target: topic-based SNS としての kukuri の長期拡張性確保（短文投稿 / 画像 / 動画 / 配信 / ゲーム）

---

## 1. 目的

kukuri の新 canonical flow は、`iroh-gossip` / `iroh-docs` / `iroh-blobs` の責務を明確に分離し、将来的な拡張にも耐える同期基盤を定義することを目的とする。

現行 MVP では `IrohGossipTransport` を最小導入し、topic ごとの投稿同期を成立させることを優先している。しかしこの構成のまま機能を積み増すと、以下の問題が生じやすい。

- gossip に履歴や本体配送の責務が混ざる
- Store と network state の境界が曖昧になる
- 画像 / 動画 / 配信 / ゲームなど、高容量・高頻度・長寿命データへの拡張時に設計負債が増大する
- SQLite が正本化し、共有状態の真実源が複数になる

この設計書では、kukuri の canonical flow を次の4層に再定義する。

- `iroh-gossip`: 新着通知・短命シグナル
- `iroh-docs`: 共有索引・共有メタデータ
- `iroh-blobs`: 本文・画像・動画などの本体
- `SQLite`: ローカル read model / UI state / 再構築可能なキャッシュ

---

## 2. 設計原則

### 2.1 Single Source of Truth
共有される durable state の正本は `iroh-docs` と `iroh-blobs` に限定する。  
SQLite は正本ではなく、ローカル派生データに限定する。

### 2.2 Gossip is not History
`iroh-gossip` は履歴同期・本体配送・正本保持に使わない。  
役割は「新しい情報が出たことを素早く知らせる trigger」に限定する。

### 2.3 Metadata / Payload Split
メタデータと本体を分離する。

- メタデータ: `iroh-docs`
- 本体データ: `iroh-blobs`

### 2.4 Rebuildability
SQLite の内容は、原則として docs/blobs から再構築可能でなければならない。

### 2.5 Future-proofing
短文投稿の都合に最適化しすぎず、以下への拡張が可能であることを前提にする。

- 画像投稿
- 動画投稿
- 配信セッション
- ゲームルーム / リプレイ / スナップショット
- 将来的な暗号化 payload
- 複数デバイス同期

---

## 3. 非目標

この設計書では以下は扱わない。

- DHT / community node / relay auth の最終設計
- E2EE DM の最終設計
- 最終的な ranking アルゴリズム
- 全文検索エンジンの最終仕様
- モデレーションポリシーの最終仕様
- 動画配信 / ゲーム通信の wire optimization 詳細

---

## 4. 新 canonical flow 概要

### 4.1 書き込みの canonical flow

```text
User Action
  -> Core command (`create_post`)
  -> Payload を iroh-blobs に保存
  -> Metadata / index を iroh-docs に保存
  -> Topic に GossipHint を publish
  -> Local projection(SQLite) を更新
  -> UI に反映
```

### 4.2 読み込みの canonical flow

```text
Peer Join / App Start
  -> 必要な replica(topic / author / device) を open / sync
  -> docs を読み、timeline / thread / profile を構築
  -> 必要な blob を lazy fetch
  -> SQLite projection を更新
  -> UI 表示
  -> 以後は gossip hint に応じて docs/blobs を追従
```

### 4.3 後追い参加者の canonical flow

```text
Late Joiner
  -> ticket/static peer で接続
  -> gossip は「今以降の通知」を受信
  -> docs から過去の索引を backfill
  -> blob を必要に応じて fetch
  -> timeline/thread を再構築
```

---

## 5. Data Source Policy

### 5.1 優先順位

各データ種別の canonical source は事前に固定する。

| データ種別                      | canonical source | 備考                                                  |
| -------------------------- | ---------------- | --------------------------------------------------- |
| 投稿ヘッダ                      | docs             | event_id, topic_id, author, root/reply, body_ref など |
| 投稿本文                       | blobs            | text / markdown / rich text の本体                     |
| 画像 / 動画 / 添付               | blobs            | 原本・variant・segment                                  |
| topic timeline index       | docs             | 並び順と存在の記録                                           |
| thread edge                | docs             | root / reply 関係                                     |
| profile metadata           | docs             | display_name, bio, avatar_ref など                    |
| topic metadata             | docs             | title, description, icon_ref など                     |
| moderation record          | docs             | tombstone / hide / lock / block                     |
| presence / typing          | gossip           | 非正本・短命                                              |
| live/game transient signal | gossip           | 非正本・短命                                              |
| home timeline merge結果      | SQLite           | ローカル派生                                              |
| unread count               | SQLite           | ローカル派生                                              |
| UI state                   | SQLite           | scroll/draft/expanded thread 等                      |

### 5.2 禁止事項

- SQLite にしか存在しない共有 durable state を作らない
- gossip payload を唯一の状態ソースとして扱わない
- docs と SQLite の両方を同時に「正本」として扱わない
- blobs 未保存の payload を shared durable post とみなさない
- public replica に private read cursor を書かない

### 5.3 再構築可能性

- SQLite を削除しても docs/blobs から共有状態を復元できる
- gossip を取り逃しても docs/blobs から durable post を復元できる
- 後追い参加者は docs/blobs のみで過去投稿を再現できる

---

## 6. データモデルの分類

### 6.1 Post / Thread / Topic 系

#### docs に置く

- `post/<event_id>/header`
- `topic/<topic_id>/timeline/<sort_key>/<event_id>`
- `thread/<root_event_id>/<sort_key>/<event_id>`
- `author/<pubkey>/posts/<sort_key>/<event_id>`

#### blobs に置く

- `body/<body_hash>`
- `asset/<asset_hash>`
- `attachment manifest`

#### gossip に流す

- `TopicIndexUpdated`
- `ThreadUpdated`

#### SQLite に置く

- timeline row cache
- thread row cache
- home timeline merged view
- unread counts

### 6.2 Profile / Topic Metadata

#### docs に置く

- `profile/<pubkey>`
- `topic_meta/<topic_id>`
- `moderation/<action_id>`

#### blobs に置く

- avatar image
- banner image
- topic icon / topic cover

#### gossip に流す

- `ProfileUpdated`
- `TopicMetaUpdated`

#### SQLite に置く

- profile summary cache
- topic summary cache

### 6.3 Cursor / Device State

#### docs に置く

- shared sync frontier
- replica head
- optional pinned shared marker

#### private docs or SQLite に置く

- read cursor
- per-device sync checkpoint

#### draft

- local preference

#### gossip に流す

- 原則なし

### 6.4 Live / Game / Rich Media

#### docs に置く

- live session manifest
- room metadata
- game room metadata
- replay manifest
- snapshot metadata

#### blobs に置く

- video poster
- video segment
- audio segment
- replay payload
- snapshot payload

#### gossip に流す

- live started / ended
- viewer hint
- game room ping
- typing / presence / room activity

#### SQLite に置く

- local playback cache index
- local session state
- local quality heuristic

---

## 7. Replica 設計

### 7.1 公開 replica

#### topic replica

- `topic::<topic_id>`

想定 key:

- `timeline/<sort_key>/<event_id>`
- `post/<event_id>/header`
- `thread/<root_id>/<sort_key>/<event_id>`
- `asset/<event_id>/<ordinal>`
- `live/<session_id>/manifest`
- `moderation/<action_id>`

#### author replica

- `author::<pubkey>`

想定 key:

- `profile`
- `posts/<sort_key>/<event_id>`

### 7.2 非公開 / デバイス replica

#### device replica

- `device::<pubkey>::<device_id>`

想定 key:

- `cursor/topic/<topic_id>`
- `cursor/thread/<root_id>`
- `pin/<blob_hash>`
- `draft/<topic_id>`
- `pref/<name>`

#### 原則

- topic / author replica は共有用
- device replica は private / per-device 用
- read cursor を public topic replica に置かない

---

## 8. Core 型の新設計

```rust
pub struct EventEnvelope {
    pub event_id: EventId,
    pub topic_id: TopicId,
    pub author: Pubkey,
    pub root: Option<EventId>,
    pub reply_to: Option<EventId>,
    pub created_at: i64,
    pub payload_ref: PayloadRef,
    pub attachments: Vec<AssetRef>,
    pub signature: Vec<u8>,
}

pub enum PayloadRef {
    InlineText { text: String }, // migration only
    BlobText {
        hash: BlobHash,
        mime: String,
        bytes: u64,
    },
}

pub struct AssetRef {
    pub hash: BlobHash,
    pub mime: String,
    pub bytes: u64,
    pub role: AssetRole,
}

pub enum AssetRole {
    ImageOriginal,
    ImagePreview,
    VideoPoster,
    VideoManifest,
    Attachment,
}

pub enum GossipHint {
    TopicIndexUpdated {
        topic_id: TopicId,
        event_ids: Vec<EventId>,
    },
    ThreadUpdated {
        root_id: EventId,
        event_ids: Vec<EventId>,
    },
    ProfileUpdated {
        author: Pubkey,
    },
    Presence {
        topic_id: TopicId,
        author: Pubkey,
        ttl_ms: u32,
    },
    Typing {
        topic_id: TopicId,
        root_id: Option<EventId>,
        author: Pubkey,
        ttl_ms: u32,
    },
    LiveSignal {
        topic_id: TopicId,
        session_id: String,
        kind: LiveSignalKind,
    },
}
```

### 注記

MVP 直後の移行期は InlineText を残してよい。
ただし canonical payload は最終的に BlobText に寄せる。

---

## 9. Module 責務

### 9.1 Core

- 型定義
- command orchestration
- id / sort_key / dedupe の規則
- canonical flow の制御

### 9.2 GossipTransport

- hint の subscribe / publish
- presence / typing / transient signal
- payload 本体を持たない

### 9.3 DocsSync

- replica open / import / subscribe
- index / metadata の同期
- docs event の購読

### 9.4 BlobService

- blob の保存
- blob の検証
- lazy fetch / prefetch / pin

### 9.5 ProjectionStore(SQLite)

- local cache
- merged timeline
- unread state
- UI list 高速化
- source provenance 付き再構築可能 projection

---

## 10. 書き込みフロー詳細

### 10.1 `create_post`

1. User が topic を選択し本文を入力する
2. Core が body を blob 化する
3. `body_hash` を取得する
4. `EventEnvelope` を組み立てる
5. topic replica に post header / timeline index / thread edge を書く
6. author replica に author log を書く
7. 必要なら asset manifest を docs に書く
8. `GossipHint::TopicIndexUpdated` を publish する
9. local projection を更新する
10. UI に optimistic ではなく canonical-ready 状態として表示する

### 10.2 `reply_post`

- `root` と `reply_to` を envelope に含める
- `thread/<root_id>/...` に index を追加する
- topic timeline にも通常投稿として並べるかは product policy に従う

### 10.3 update_profile

- profile metadata を docs に書く
- avatar があれば blob 保存する
- `ProfileUpdated` を gossip publish する
- local cache を更新する

---

## 11. 読み込みフロー詳細

### 11.1 起動時

1. device key を読み込む
2. known replica を列挙する
3. topic / author / device replica を open する
4. docs の最新状態を購読開始する
5. projection を必要範囲だけ更新する
6. missing blob を遅延取得キューに積む
7. UI を表示する
8. gossip subscribe を開始する

### 11.2 timeline 表示時

1. SQLite projection があれば即表示
2. なければ docs query から最小表示を生成
3. preview 用 blob が未取得なら placeholder 表示
4. body / image を lazy fetch する
5. 取得完了後 UI 差し替え

### 11.3 thread 表示時

1. `thread/<root_id>/...` を docs から解決
2. root / reply 群を projection 化
3. missing body blob を取得
4. thread pane に反映

---

## 12. 後追い参加 / 再同期設計

### 12.1 Late Joiner

1. peer ticket または static peer で参加
2. gossip で新着 hint を受け始める
3. topic replica を import / sync する
4. docs から timeline index を取得する
5. body / asset blob を必要に応じて取得する
6. projection を再構築する
7. UI に過去投稿を表示する

### 12.2 Gossip 取り逃し

gossip hint を取り逃しても durable post は失われない。
正本は docs/blobs にあるため、再同期により復元される。

### 12.3 SQLite 消失

SQLite 削除時は次の順で再構築する。

1. known replica を open
2. docs から index を再取得
3. blob availability を再確認
4. projection を再生成
5. UI state は必要に応じて初期化

---

## 13. Projection(SQLite) 方針

### 13.1 残してよいもの

- home timeline merged rows
- topic timeline cache
- thread cache
- unread count
- search token cache
- download queue
- local draft
- scroll position
- panel state

### 13.2 持ってはいけないもの

- 唯一の投稿本文
- 唯一の thread relation
- 唯一の topic index
- 唯一の profile state
- 唯一の moderation state

### 13.3 provenance 必須列

projection row は最低限次を持つ。

- `source_replica_id`
- `source_key`
- `source_event_id`
- `source_blob_hash`
- `derived_at`
- `projection_version`

---

## 14. Key / Sort 設計

timeline の並びと再構築性のため、docs key 設計を固定する。

#### 14.1 timeline key

`timeline/<logical_ts>-<event_id>`

### 14.2 thread key

`thread/<root_id>/<logical_ts>-<event_id>`

### 14.3 author key

`posts/<logical_ts>-<event_id>`

### 14.4 規則

- 同一 timestamp 衝突時も event_id で全順序を確定する
- sort_key は deterministic であること
- projection は docs の key 順を唯一の基準とする

---

## 15. 遷移期ポリシー

### 15.1 Dual Write

移行初期は `InlineText` と `BlobText` の併存を許可する。
ただし新規実装は `BlobText` 優先とする。

### 15.2 Read Priority

- `BlobText` があればそれを使う
- `BlobText` がなければ `InlineText` を使う
- projection は最終的に `BlobText` 前提へ移行する

### 15.3 Cutover 条件

以下が満たされたら `InlineText` を縮退候補とする。

- create_post が常に blob 保存を行う
- late joiner が docs/blobs だけで timeline を再構築できる
- SQLite 削除後も durable post が欠落しない
- thread 表示が lazy blob fetch に対応する

---

## 16. Contract / Scenario 追加案

### 16.1 Contract

- `gossip_hint_contains_no_payload_body`
- `post_body_blob_roundtrip`
- `topic_index_sort_key_stable`
- `thread_index_projection_stable`
- `sqlite_deletion_does_not_lose_shared_state`
- `private_cursor_not_written_to_public_replica`
- `projection_rebuild_from_docs_blobs_only`

### 16.2 Scenario

- `late_joiner_backfills_timeline_from_docs`
- `post_then_restart_then_restore_from_docs_blobs`
- `missing_gossip_but_docs_sync_recovers_post`
- `image_post_visible_before_full_blob_download`
- `two_device_private_cursor_isolated`
- `thread_open_triggers_lazy_blob_fetch`

---

## 17. Phase への組み込み方針

この設計は既存再構築プランを破壊せず、Phase 4 直後に追加する。

### 新設フェーズ案

#### Phase 4.5 Data Plane Split

#### 目的

- `gossip = hint`
- `docs = shared index`
- `blobs = canonical payload`
- `SQLite = derived projection`

#### Exit

- create_post が `blob -> docs -> gossip -> projection` の順で動作する
- list_timeline / list_thread が projection または docs 由来で成立する
- late joiner が docs/blobs のみで timeline を復元できる
- SQLite 削除後に durable state を失わない

---

## 18. 最小導入順

1. `PayloadRef`, `AssetRef`, `GossipHint` を導入
2. create_post を blob 保存前提にする
3. topic replica / author replica を導入
4. post header / timeline index を docs に書く
5. late joiner scenario を先に追加する
6. projection の provenance を導入する
7. list_timeline / list_thread を projection or docs 基準へ移行する
8. profile / image / thread をこの流れへ統合する

---

## 19. 最終判断

kukuri の canonical flow は次で統一する。

- **正本**
  - metadata: `iroh-docs`
  - payload: `*iroh-blobs`
- **非正本**
  - new post / presence / typing / transient signals: `iroh-gossip`
- **派生**
  - home timeline / unread / search / UI state: `SQLite`

この設計により、kukuri は短文投稿中心のメッセージング実装から脱却し、将来的な rich media / live / game 拡張にも耐える同期基盤へ移行できる。
