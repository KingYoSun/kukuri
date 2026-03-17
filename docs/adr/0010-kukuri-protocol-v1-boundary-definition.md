# ADR: kukuri Protocol v1 の境界定義（何を残し、何を切るか）

- Status: Proposed
- Date: 2026-03-17
- Authors: KingYoSun
- Decision Drivers:
  - codex/next の現在アーキテクチャとの整合
  - relay-first な Nostr 世界観との乖離解消
  - 今後の live / game / media / community-node 拡張のしやすさ
  - 技術的負債の抑制
  - 必要最小限の相互運用性維持

---

## 1. Context

`codex/next` ブランチでは、すでに kukuri の中核通信モデルは以下に移行している。

- static-peer での疎通
- DHT を用いた peer 探索と投稿伝播
- `iroh-gossip` をヒント配信・通知レイヤとして利用
- 実データは `iroh-docs` / `iroh-blobs` で同期
- `community-node`（`iroh-relay`）経由の peer 間疎通を拡張中

この構成により、テキスト投稿だけでなく、画像・動画・live・game を含む多様なデータ流通を、relay 上のイベント配送ではなく、P2P / doc-sync / blob-sync を前提とした形で扱えるようになっている。

一方、Nostr / NIP 群は本質的に以下の世界観を前提としている。

- relay と client の間で `REQ` / `EVENT` / `CLOSE` 等をやり取りする
- relay がイベント流通の中心である
- クライアント間の同期というより relay 経由の配信が基本である
- 大容量実体データは外部参照や別拡張に逃がすことが多い

このため、現在の kukuri 実装では、**署名付きイベント形式など一部は NIP と整合するが、流通・同期・接続・状態管理はすでに Nostr の中心設計から外れている**。

この乖離を抱えたまま「中核データ以外の拡張」を NIP に引きずられて実装し続けると、以下の問題が発生する。

- relay-first 意味論を擬似的に再実装する変換層が増える
- gossip / docs / blobs / DHT の役割分担が曖昧になる
- 実装・仕様・テストが二重化する
- kukuri 独自の強みが「Nostr 互換の都合」で制限される

---

## 2. Problem Statement

kukuri は今後、以下のどちらを選ぶべきか。

1. **NIP 準拠を広く維持し、relay-first の設計制約も一定受け入れる**
2. **内部プロトコルを kukuri 固有として定義し、NIP は限定的互換層に縮退させる**

本 ADR は、`kukuri Protocol v1` における **「何を残し、何を切るか」** の境界を定義する。

---

## 3. Decision

`kukuri Protocol v1` は、**内部プロトコルを kukuri 固有仕様として定義する**。

ただし、既存資産・署名検証・外部相互運用性を考慮し、**鍵・署名・イベント外形・一部タグ規約のみは継続利用する**。

すなわち、方針は次のとおりとする。

> **内部は kukuri Protocol、外部互換として Nostr subset を維持する。**

また、設計上は **「脱NIP」ではなく「脱 relay-first」** を明確に意思決定する。

---

## 4. Scope Boundary

---

### 4.1 残すもの（Keep）

#### 4.1.1 鍵ペアと署名モデル
以下は継続する。

- 公開鍵 / 秘密鍵ベースのアイデンティティ
- イベント署名による作成者証明
- 署名検証可能なイベント envelope

理由:

- 既存データ資産を活かせる
- クライアント・ノード・外部連携での検証コストが低い
- `kukuri identity` を独自設計し直すコストに対して利益が薄い

---

#### 4.1.2 イベントの基本外形
以下のような概念は維持する。

- `id`
- `pubkey`
- `created_at`
- `kind` 相当の型識別子
- `tags`
- `content`
- `sig`

ただしこれは **Nostr relay に投げることを前提としたイベント** ではなく、**kukuri における署名付きメタデータ envelope** として扱う。

理由:

- データモデルとして十分に枯れている
- ログ、監査、差分検出、検証に使いやすい
- docs / blobs の参照メタとして扱いやすい

---

#### 4.1.3 一部タグ規約
以下のような、意味が汎用的なタグ規約は継続候補とする。

- 参照先を表すタグ
- 関連オブジェクトを表すタグ
- 著者・対象・親子関係を示すタグ
- media / mime / pointer / hash のような汎用メタ情報

ただし、採用するタグは **kukuri の実装に本当に必要なもののみ** に限定する。

理由:

- 完全独自タグ体系に振り切るより移行負荷が低い
- import/export 時の対応が取りやすい

---

#### 4.1.4 Import / Export 互換
以下は将来の互換層として維持する。

- Nostr 風イベントの import
- kukuri イベントからの export
- 一部 kind / tag の変換
- relay bridge / gateway の実装余地

理由:

- エコシステム接続の保険になる
- 開発中のデータ可視化・検証にも使える
- 「内部は独自、外部へは必要に応じて橋渡し」の方針と整合する

---

### 4.2 切るもの（Drop）

#### 4.2.1 relay-first を中核前提とする設計
以下は kukuri Protocol v1 の中核仕様から外す。

- relay が真実の流通ハブである前提
- `REQ` / `EVENT` / `CLOSE` を内部同期の基準にすること
- relay subscription を購読モデルの主軸にすること
- relay availability をシステム健全性の中心に置くこと

理由:

- kukuri の現実装は docs / blobs / gossip / DHT / relay-node の分離に寄っている
- relay 基準を保つほど設計が二重化する
- live / game / media の実体流通に向いていない

---

#### 4.2.2 NIP の意味論を内部仕様の正とすること
以下はやめる。

- 「NIP にあるから内部仕様でもそうする」
- 「Nostr クライアントがそうだから kukuri もそうする」
- 「kind や relay の都合で内部データ構造を決める」

理由:

- 仕様判断の主語を NIP から kukuri に戻す必要がある
- 今後の強みは P2P / topic / state sync にある

---

#### 4.2.3 実体データをイベント配送中心で考えること
以下はやめる。

- 投稿本体やメディア実体をイベント配送の延長として扱う
- 大容量データを relay 互換都合で表現する
- 「イベント1個 = 流通単位」という前提

理由:

- すでに `iroh-docs` / `iroh-blobs` が実体同期の中心である
- live / game / media は pointer + state + blob の分離の方が自然

---

#### 4.2.4 Community Node を「Nostr Relay の別名」として扱うこと
以下は明示的に否定する。

- community-node = relay
- community-node = ただのイベント中継器
- relay プロトコル互換性が第一要件

代わりに、community-node は以下として再定義する。

- bootstrap node
- connectivity assist node
- discovery assist node
- policy / trust / moderation assist node
- optional gateway / bridge node

理由:

- 現在の役割に即している
- 将来的な moderation / federation / trust 拡張にも自然

---

## 5. kukuri Protocol v1 のレイヤ定義

`kukuri Protocol v1` では、責務を以下のように分離する。

### Layer 1. Identity / Signed Envelope
責務:

- 鍵
- 署名
- 作成者証明
- 最小メタ情報

代表要素:

- `pubkey`
- `sig`
- `id`
- `created_at`
- `kind`
- `tags`
- `content`

備考:

- NIP 由来の外形を保持してよい
- ただし意味論の最終決定権は kukuri にある

---

### Layer 2. Hint / Notification
責務:

- 新着通知
- topic 告知
- participation hint
- doc/blob locator の伝搬
- peer 接続補助

実装候補:

- `iroh-gossip`

備考:

- 真実の保存先ではない
- loss-tolerant / eventually-notified でよい
- 「届かなかったら終わり」ではなく「拾えたら同期へ進む」役割

---

### Layer 3. State / Structured Sync
責務:

- 投稿一覧
- スレッド状態
- membership
- room/community state
- live/game session state
- pointer index
- versioned state

実装候補:

- `iroh-docs`

備考:

- ここが kukuri の中心
- 「今の状態」を扱う責務を置く

---

### Layer 4. Blob / Large Object Transport
責務:

- 画像
- 動画
- 音声
- 添付ファイル
- live / game 関連アセット
- 大容量 payload

実装候補:

- `iroh-blobs`

備考:

- Envelope には参照情報だけを載せる
- 実体は blob 側に置く

---

### Layer 5. Discovery / Routing / Connectivity
責務:

- peer discovery
- DHT
- static-peer
- relay assist
- NAT 越え補助
- bootstrap

実装候補:

- DHT
- static-peer
- `iroh-relay`
- community-node

備考:

- relay はこの層の一部であり、中核ではない

---

## 6. Design Principles

### 6.1 Source of Truth を明確にする
- 通知の真実は gossip ではない
- 一覧や状態の真実は docs
- 実体の真実は blobs
- 接続性の補助は DHT / relay / static-peer

---

### 6.2 Envelope と Payload を分離する
- 署名対象は最小メタ情報
- 実体データは別レイヤで扱う
- media / live / game ほどこの原則を強める

---

### 6.3 Nostr compatibility is optional, not normative
- 互換は「あれば便利」
- 内部仕様の決定根拠にはしない
- bridge/gateway で吸収する

---

### 6.4 Community / Topic / Session を第一級概念にする
Nostr は event-first だが、kukuri は以下を第一級にする。

- community
- topic
- room
- live session
- game session
- stateful membership

---

### 6.5 メディアとリアルタイム性を正面から扱う
kukuri はテキスト SNS のみを対象にしない。

- image
- video
- audio
- live
- game

を最初から扱えるよう、イベント一本化ではなく、state + blob + hint の分離モデルを採用する。

---

## 7. Consequences

### 7.1 Positive
- アーキテクチャの整合性が上がる
- relay-first の擬似互換負債が減る
- gossip / docs / blobs / DHT の責務分離が明確になる
- live / game / media など kukuri 独自領域を伸ばしやすい
- community-node の役割を自然に拡張できる
- テスト観点が整理しやすい

---

### 7.2 Negative
- 「全面 Nostr 互換」を期待する実装は難しくなる
- 一部の NIP 実装資産はそのままでは使えなくなる
- 独自仕様の設計・命名・移行作業が必要になる
- 将来 bridge 層の保守コストが発生する

---

### 7.3 Neutral / Trade-off
- 鍵・署名・イベント外形は残すため、完全独自化より移行は楽
- ただし「中途半端な互換」は避け、境界を明示する必要がある

---

## 8. Rejected Alternatives

### Alternative A: 全面 NIP 維持
却下理由:

- 現在の codex/next と設計思想が一致しない
- relay-first の制約が内部実装に負債として残る
- kukuri の独自性が伸ばしにくい

---

### Alternative B: 鍵・署名・イベント外形も含めて完全脱NIP
却下理由:

- 移行コストが高い
- 既存資産を捨てるコストに対して得るものが限定的
- import/export 互換の足場まで失う

---

### Alternative C: 表面上だけ NIP 準拠を名乗り続ける
却下理由:

- 実態と仕様が乖離する
- 開発者認知を誤らせる
- 負債を先送りするだけになる

---

## 9. Protocol Boundary Summary

### kukuri Protocol v1 に含める
- 鍵と署名
- 署名付き envelope
- 最小イベントメタ
- docs/blobs/gossip/DHT/relay の責務分離
- community-node の bootstrap / assist 役割
- topic/community/session を中心とした状態同期

### kukuri Protocol v1 に含めない
- relay-first 意味論
- REQ/EVENT/CLOSE を中核とする内部設計
- relay subscription ベースの状態管理
- NIP を内部仕様の規範とすること
- イベント単体で実体流通を完結させる考え方

---

## 10. Migration Guidance

### Phase 1: 用語の再定義
- relay → connectivity assist / gateway の一部
- event → signed envelope
- post delivery → pointer + state sync + blob fetch

### Phase 2: Data Source Policy の固定
- hint の正: gossip ではない
- state の正: docs
- blob の正: blobs
- discovery の正: DHT / static-peer / relay assist

### Phase 3: NIP 依存点の棚卸し
分類:

- keep
- wrap
- replace
- delete

例:
- keep: 鍵、署名、基本 envelope
- wrap: 一部 tag / import-export
- replace: relay subscription 前提処理
- delete: relay-first を前提とした内部状態管理

### Phase 4: Bridge 層の後置
- Nostr 互換は adapter として外出し
- core に NIP 依存を残さない

---

## 11. Decision Statement

kukuri は `Protocol v1` において、**内部仕様を relay-first な Nostr から独立させる**。

ただし、既存資産と相互運用性の観点から、**鍵・署名・イベント外形・一部タグ規約は継承する**。

この決定により、kukuri は **Nostr 準拠プロダクト** ではなく、**Nostr 由来の署名付き envelope を採用した P2P / state-sync / blob-sync 指向プロトコル** として進化する。

---
