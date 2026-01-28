
# kukuri Community Node 実装 設計書兼計画書（Codex向け）

> 対象: kukuri OSS（kukuriリポジトリ内）に「コミュニティノード（収益化Super Node）」を追加し、KIP-0001のDraft仕様を実装する。
> 目的: P2Pネットワークの持続性・UX・当局折衝を担う“代表ノード”市場を作りつつ、中央集権化を避ける。

## 0. 背景と設計原則

### 0.1 背景
- 端末スペック不足（index/trust計算/大量ストレージ/監視）をクライアント側だけで解決しにくい。
- それ以上に、**コミュニティの代表（運用者）**が存在することが「継続・法対応・通報窓口」に寄与する。

### 0.2 原則（破らない）
1) **ノードは権威ではなく“提案者”**  
   - moderation = label（署名付き提案）  
   - trust = attestation（署名付き主張）  
   - クライアントは採用ノードを選べる。

2) **役割分割 + 併用可能**  
   - bootstrap/index/moderation/trust/relay を分離可能にし、全部入りノードへのロックインを避ける。

3) **公開度の段階設計**  
   - public → friend+ → friend → invite の導線を壊さない  
   - “追放”は epochローテで未来閲覧を止める（過去は取り戻せない前提）。

## 1. スコープ

### 1.1 実装対象（KIP-0001）
- Event kinds: 39000/39001/39005/39006/39010/39011/39020/39021
- 暗号: NIP-44方式（ライブラリ/既存実装に合わせる）
- データ検証: 署名検証、tag正当性、期限(exp)処理

### 1.2 “ノードAPI”の扱い
- KIPはイベント表現中心。join.request は **イベント化（39022）** で確定し、課金・購読は User API で扱う。
- 初期は **HTTP API** を併設し、後でKIP化（イベント化）を検討する。

## 2. アーキテクチャ

### 2.1 コンポーネント
- **Client (Tauri)**:
  - KIPイベント生成/検証
  - ノード一覧・採用ポリシー設定
  - 暗号鍵の保管（Keychain/OSストア推奨）
  - label/attestationの適用（UI/フィルタ/警告）

- **Community Node (Server/CLI/Daemon)**:
  - 役割別モジュール
    - Bootstrap: discovery補助（ヒント配布）
    - Relay (optional): 互換送受信
    - Index: topic別集約・検索・ランキング
    - Moderation: report受理、label発行、監査ログ
    - Trust: attestation発行、集約（有料化ポイント）
  - 署名鍵（Node Key）管理
  - 監査/ポリシー/当局窓口メタ情報配布

### 2.2 “ノード市場”を成立させる要件
- クライアントが **複数ノードを並行採用**できる
- ノードを **簡単に追加/削除/切替**できる
- ノードが出すデータは **署名と期限で検証可能**

## 3. データモデル（実装用）

### 3.1 共通
- Event（Nostr互換フィールド）:
  - id, pubkey, created_at, kind, tags, content, sig
- Tag:
  - `["k","kukuri"]` `["ver","1"]` 推奨
- 時刻:
  - `exp`（unix seconds）を扱う: 期限切れは無効扱い or 重み低下

### 3.2 kind別スキーマ
#### 39000 node.descriptor (replaceable)
- content(JSON):
  - schema, name, roles[], endpoints{http,ws}, pricing{}, policy_url, jurisdiction, contact
- tags:
  - role/jurisdiction/policy/ver

#### 39001 node.topic_service (replaceable)
- tags:
  - t(topic_id), role, scope

#### 39005 report
- tags:
  - target(event/pubkey/node), reason
- content:
  - 任意（説明）

#### 39006 label
- tags:
  - target, label, confidence, policy, exp
- content:
  - 任意（説明/根拠）

#### 39010 attestation
- content(JSON):
  - schema, subject, claim, value, evidence[], context, expires
- tags:
  - sub(type,id), claim, t(topic_id)?, exp

#### 39011 trust.anchor (replaceable)
- tags:
  - attester, claim?, t(topic_id)?, weight

#### 39020 key.envelope
- tags:
  - p(recipient), t(topic_id), scope, epoch
- content:
  - NIP-44暗号文（中身は {topic,scope,epoch,key_b64,...}）

#### 39021 invite.capability
- content:
  - NIP-44暗号文（中身は {topic,scope,expires,max_uses,nonce,issuer,...}）

#### 39022 join.request
- tags:
  - t(topic_id), scope, d(join:...)
- content:
  - {topic, scope, invite_event_json?, requester, requested_at}

## 4. 主要フロー（実装レベル）

### 4.1 ノード発見・採用
1) Clientは gossip/DHT/既知URL から node.descriptor(39000) を収集
2) topicごとに node.topic_service(39001) を検索/購読
3) ユーザーが採用:
   - indexノードA、moderationノードB、trustノードC…のように併用可能

実装要点:
- ノード候補の“優先度”は UI設定 + trust.anchor で決める
- 署名検証 + ver + exp を必ずチェック

### 4.2 モデレーション（report → label）
- report(39005) は入力（ゲームされやすい）
- nodeは内部ポリシーに沿って label(39006) を発行（署名付き提案）
- clientは採用ノードのlabelのみ適用（もしくは重み付け）

実装要点:
- labelは必ず exp を付ける（暫定判断）
- labelは「消す」ではなく「隠す/警告/隔離」など段階にする

### 4.3 Trust（attestation + anchor）
- trust計算ノードは、以下を出せる:
  - (A) **根拠つきattestation（39010）**
  - (B) 集約スコア（39010 claim=reputation など）※ただし“提案”
- clientは trust.anchor(39011) で採用attesterを決める

実装要点:
- 最初は claim を絞る（例: `reputation`, `moderation.risk`, `capability`）
- evidence は “見せられるもの” だけにする（プライバシー注意）
- 集約ロジックは差別化ポイントなのでノードごとに違ってよい（ただしスキーマは共通）

### 4.4 公開度（鍵管理）
- friend/friend+:
  - 投稿contentを共有鍵で暗号化
  - 鍵は受信者ごとに key.envelope(39020) を送付
- invite:
  - invite.capability(39021) を送る
  - join.request(39022) で参加希望を通知し、承認後に key.envelope を配る（P2P-only）

追放:
- epoch++（鍵ローテ）
- 残留者へ新 key.envelope を再配布
- 過去暗号文は回収しない（現実的限界）

実装要点:
- 鍵の保存は OS keychain推奨（Tauri連携）
- epoch管理は topic+scope単位
- “friend+（FoF）”対象集合は trustノードが提供してもよい（低スペック対策）

## 5. Node HTTP API（初期実装案）

> KIP外だが実装を進めるために最小限入れる。Access Control（invite/keys）は **P2P-only** を正とする。

- `GET /v1/bootstrap/nodes` : 39000配布（node.descriptor）
- `GET /v1/bootstrap/topics/:topic_id/services` : 39001配布（topic_service）
- `POST /v1/reports` : report受理（ただし最終は39005で発行しても良い）
- `GET /v1/search?q=...` : index（課金は将来）

## 6. セキュリティ & プライバシー（最低限の脅威モデル）

### 6.1 脅威
- 悪意ノードが label/attestation を乱発して世論操作
- Sybil（大量鍵）で friend+ / trust を汚染
- invite漏洩、鍵漏洩
- reportスパム（DoS）
- indexが個人関係を推定する（プライバシー侵害）

### 6.2 対策（v1で必須）
- クライアント側で「採用ノード」を明示設定（デフォルトは保守的）
- label/attestationは必ず署名検証 + 期限(exp)
- capabilityは短命/回数制限/nonce（リプレイ耐性）
- join.request は受信側で rate limit/手動承認（濫用耐性）
- friend+/FoFは **“関係そのもの”を外に出さない**（可能ならローカル計算 or 暗号化配送）

## 7. リポジトリ構成（提案）

- `docs/kips/` : KIP仕様（KIP-0001.md）
- `/crates/kip_types/` : kind/tag/contentの型、検証、(de)serialize
- `/crates/crypto/` : NIP-44ラッパ（既存採用に合わせる）
- `/apps/client-tauri/` : クライアント
- `/apps/community-node/` : ノード（daemon）
- `/apps/kukuri-cli/` : 管理用CLI（鍵ローテ/招待発行/監査）

## 8. 実装マイルストーン（最短で価値が出る順）

### M0: 仕様固定（1〜2日）
- KIP-0001 v0.1 を `docs/kips/` に追加
- kind・tag命名・schema名を確定
- 互換性方針（ver/schema/v2方針）をREADMEに明記

### M1: kip_types 基盤（2〜4日）
- Event/Tag型
- kindごとの `validate()`（必須tag、json schema、exp）
- 署名検証の統一API

**Acceptance**
- 39000/39001/39010/39020 を生成・検証できる

### M2: Community Node “広告”だけ実装（2〜4日）
- 39000 node.descriptor を定期発行
- 39001 topic_service を発行
- クライアントがノード一覧表示できる

**Acceptance**
- クライアントで「このtopicのindex候補ノード」が見える

### M3: Moderation v1（3〜6日）
- report(39005) 発行UI（クライアント）
- nodeがlabel(39006)を発行（手動でもOK）
- クライアントが採用ノードのlabelでフィルタ/警告表示

**Acceptance**
- 採用ノードAのlabelだけが反映され、ノードBは無視できる

### M4: Access Control v1（5〜10日）
- key.envelope(39020) 送受信
- friend/friend+ scopeの暗号投稿（復号できる）
- invite.capability(39021) + join.request(39022)

**Acceptance**
- friend投稿が非メンバーには読めず、メンバーは復号できる
- epochローテで追放後の新投稿は読めなくなる

### M5: Trust v1（5〜10日）
- attestation(39010) 発行（reputation/moderation.risk/capabilityから開始）
- trust.anchor(39011) UI
- クライアントの“採用attester”切替で表示が変わる

**Acceptance**
- 同じ対象に対する評価が、採用attesterによって変わる（＝押し付けない）

### M6: Index v1（任意/収益化の核）
- topic別の簡易インデックス（最新N件、人気、検索）
- 課金は別KIP化（購読トークン等）

## 9. コーディング指示（Codexに渡すプロンプト用要約）

- KIP-0001 kindsを `kip_types` に型定義し、必須tag/exp/署名検証を実装せよ。
- community-node は 39000/39001 を発行し、Access Control は **P2P-only** を正とする。
- client-tauri は
  - ノード採用設定（role別に複数）
  - label適用（採用ノードのみ）
  - key.envelope受理と鍵保管
  - scope別投稿（暗号化/復号）
  を実装せよ。
- trust はまず attestationの発行/表示/anchor採用の切替まで。集約アルゴリズムは簡易（重み平均など）でよいが、必ず“提案”として扱え。

## 10. 未決定事項（実装前に決めるチェックリスト）
- Topic ID形式（固定長/ハッシュ/人間可読）
- “friend / friend+” の関係定義（ローカル? ノード提案?）
- NIP-44実装ライブラリの選定/互換性
- gossip層とイベント保存層の境界（永続ノードの扱い）
- 課金方式（購読トークン / APIキー / 署名付きライセンス）

---

# KIP-0001 (Draft): Community Node / Attestation Trust / Access Control

> Status: Draft (v0.1)  
> Scope: kukuriネットワークにおける「コミュニティノード（収益化Super Node）」の責務分割、Trust提案（署名付きアテステーション）、公開度（public/friend+/friend/invite）の鍵配布フローを定義する。

## 0. Goals / Non-Goals

### Goals
- コミュニティノードが **役割（bootstrap/relay/index/moderation/trust）** を部分的に提供できる。
- クライアントが **複数ノードを選択・併用・乗り換え**できる（中央集権化を避ける）。
- Trustは「スコア押し付け」ではなく **署名付き提案（attestation）** として配布する。
- 公開度を段階化し、**public→friend+→friend→invite** の「サードプレイス導線」を実現する。

### Non-Goals（このKIPでは扱わない）
- 課金・決済の具体仕様（別KIP）
- 完全なスパム耐性の保証（段階的に導入）
- 過去暗号文の完全剥奪（追放は“未来の閲覧”を止めることが主）

## 1. Terms

- **User Key**: ユーザーの署名鍵（Nostr互換）
- **Node Key**: コミュニティノードの署名鍵（ノード人格）
- **Topic ID**: kukuriトピック識別子（正規形文字列。例: `kukuri:<64hex>` / `kukuri:global`）
- **Role**: `bootstrap | relay | index | moderation | trust`
- **Scope**（公開度）: `public | friend_plus | friend | invite`
- **Epoch**: 鍵世代番号（追放/漏洩時に増える）

## 2. Event Kind Allocation (reserved for kukuri)

本KIPは以下のkindを使用する（将来衝突を避けるため、kukuriで予約帯域として管理する）。

- `39000` **kukuri.node.descriptor** (replaceable推奨)
- `39001` **kukuri.node.topic_service** (replaceable推奨)
- `39005` **kukuri.report** (通報; append-only)
- `39006` **kukuri.label** (モデレーション提案; append-only)
- `39010` **kukuri.attestation** (署名付き主張; append-only)
- `39011` **kukuri.trust.anchor** (信頼アンカー; replaceable推奨)
- `39020` **kukuri.key.envelope** (鍵封筒; append-only)
- `39021` **kukuri.invite.capability** (招待capability; append-only)

共通tags（推奨）
- `["k","kukuri"]` : kukuriイベント識別
- `["ver","1"]` : 本KIPのバージョン（互換性管理）

## 3. Community Node: Capability Advertisement

### 3.1 kukuri.node.descriptor (kind=39000)
ノードが「自分の能力・規約・連絡先・エンドポイント・料金」を宣言する。

推奨tags:
- `["role","bootstrap"]` 等（複数可）
- `["jurisdiction","JP"]` 等
- `["policy","<url>"]`

content（JSON推奨例）:
```json
{
  "schema":"kukuri-node-desc-v1",
  "name":"Example Node",
  "roles":["bootstrap","index","moderation","trust"],
  "endpoints":{"http":"https://node.example","ws":"wss://node.example/ws"},
  "pricing":{"index":"subscription","trust":"per-request"},
  "policy_url":"https://node.example/policy",
  "jurisdiction":"JP",
  "contact":"ops@example"
}
```

### 3.2 kukuri.node.topic_service (kind=39001)

ノードが「特定topicに対して提供する役割」を宣言する（topic非依存運用でも、実務的にtopicが自然発生する前提）。

必須tags:

- `["t","<topic_id>"]`（NIP-01 の `#t` フィルタに寄せる）
- `["role","index|moderation|trust|relay|bootstrap"]`
- `["scope","public|friend_plus|friend|invite"]`（少なくともindex/moderationに推奨）

## 4. Moderation = Reports + Labels (提案モデル)
### 4.1 kukuri.report (kind=39005)

誰でも通報できる入力。自動適用しない（ゲームされやすい）。

tags（例）:

- `["target","event:<id>"]` or `["target","pubkey:<hex>"]`
- `["reason","spam|illegal|harassment|impersonation|nsfw"]`

### 4.2 kukuri.label (kind=39006)

コミュニティノード（または権限者）が署名して出す「判断/提案」。

tags（例）:

- `["target","event:<id>"]` / `["target","pubkey:<hex>"]`
- `["label","spam|illegal|harassment|impersonation|nsfw|safe"]`
- `["confidence","0.0-1.0"]`
- `["policy","<policy_url>#<section>"]`
- `["exp","<unix_ts>"]`（推奨: 期限）

クライアントは 採用するlabel発行者（ノード） を選択可能。

## 5. Trust = Signed Attestations (押し付けない)
### 5.1 What is Attestation?

Attestation =「私はこう主張する」という内容を署名で証明したもの。
Trust値は “結果” ではなく 根拠つき主張の束として配る。

### 5.2 kukuri.attestation (kind=39010)

署名主体（attester）が、subjectについて claim を発行する。

tags（例）:

- `["sub","pubkey","<hex>"]` / `["sub","node","<hex>"]` / `["sub","event","<id>"]`
- `["claim","reputation|identity.link|moderation.risk|capability|social.distance"]`
- `["t","<topic_id>"]`（任意）
- `["exp","<unix_ts>"]`（推奨）

content（JSON推奨例）:

```json
{
  "schema":"kukuri-attest-v1",
  "subject":"pubkey:<hex>",
  "claim":"reputation",
  "value":{"score":0.82,"level":"good"},
  "evidence":["event:<id>","url:https://..."],
  "context":{"topic":"<topic_id>"},
  "expires":1767225600
}
```

### 5.3 kukuri.trust.anchor (kind=39011)

ユーザーが「どのattesterをどの範囲で信頼するか」を宣言する（置き換え可能）。

tags（例）:

- `["attester","<pubkey_hex>"]`
- `["claim","reputation"]`（任意）
- `["t","<topic_id>"]`（任意）
- `["weight","0.0-1.0"]`

## 6. Access Control (Public → Friend+ → Friend → Invite)
### 6.1 Keys and Epoch

scopeごとに共有鍵を持つ: `K_fp`, `K_f`, `K_inv`

追放/漏洩時に `epoch++` して鍵ローテ（未来閲覧を止める）

### 6.2 kukuri.key.envelope (kind=39020)

受信者ごとに暗号化して「鍵」を渡す（NIP-44推奨）。

tags（必須）:

- `["p","<recipient_pubkey_hex>"]`
- `["t","<topic_id>"]`
- `["scope","friend_plus|friend|invite"]`
- `["epoch","<int>"]`

content（暗号化前のJSON例）:

```json
{
  "schema":"kukuri-keyenv-v1",
  "topic":"<topic_id>",
  "scope":"friend_plus",
  "epoch":7,
  "key_b64":"....",
  "issued_at":...,
  "expires":...
}
```

### 6.3 kukuri.invite.capability (kind=39021)

招待トークン（capability）を配布し、join後に key.envelope を渡す。

content（暗号化前のJSON例）:

```json
{
  "schema":"kukuri-invite-v1",
  "topic":"<topic_id>",
  "scope":"invite",
  "expires":...,
  "max_uses":1,
  "nonce":"...",
  "issuer":"pubkey:<hex>"
}
```

### 6.4 Recommended Flows

- public: 平文 or 最小限のフィルタのみ
- friend / friend+: 投稿contentを共有鍵で暗号化、鍵は key.envelope で配布
- invite: capability → join.request（39022）→ key.envelope 配布
- ban/追放: epoch++ → 残留者へ新 key.envelope 配布（過去暗号文は回収しない）

## 7. Client Rules (Minimal)

- ノードは 提案者。label/attestationの採用はユーザー/クライアント設定に依存。
- 重要データ（鍵・capability・個人関係）はデフォルトで暗号化・最小開示。
- 期限（exp）を積極的に使い、固定スコア化・永続BANを避ける設計を推奨。

## 8. Versioning

- `["ver","1"]` を共通tagとして使用
- schema名に `-v1` を付け、破壊的変更時はv2へ


---
