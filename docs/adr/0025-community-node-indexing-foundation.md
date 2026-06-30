# ADR 0025: Community Node Indexing Foundation

## Status
Draft

## Date
2026-06-30

## Base Branch
`main`

## Related
- `docs/adr/0012-topic-first_progressive_community_filtering_draft.md`（topic-first スコープ）
- `docs/adr/0013-social-graph-foundation-draft.md`（author-owned canonical source）
- `docs/adr/0005-live-session-data-classification.md`（live session = streaming room）
- `docs/adr/0006-game-room-data-classification.md`（game / metaverse room）
- `docs/safety/community-node-critical-safety.md`（§5 component boundary, §6 data flow, §8 fail-closed invariants, §11 readiness）
- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`（authority scope）
- `docs/architecture/moderation-event-trust-semantics.md`（advisory ≠ command）
- `crates/cn-operator/src/capability.rs`（`CommunityIndex` = `Availability::Planned`）
- 実装側 Issue: #404（fail-closed community indexing 本体）
- 後続 ADR: trust / relation foundation（#409）, 決定論的 moderation（#410）, 非決定論的 moderation（#411）

## 位置づけ

community node の主要未検討機能のひとつ「indexing（index / search / discovery / recommendation）」の責務境界を固定する foundation ADR である。`cn-core` に index の実体はまだ無く（実装は #404）、現状 fail-closed 不変条件は `docs/safety/community-node-critical-safety.md` に moderation 視点で散在しているだけだった。本 ADR は、その土台として **何を index し、何を index しないか**を先に確定し、その上に trust/relation（#409）・moderation（#410/#411）を載せられるようにする。

本 ADR は indexing の **scope（範囲）と content kind（対象種別）と fail-closed（安全）** の境界を定義する。trust / relation への risk signal 反映、provider 接続、ranking/recommendation アルゴリズムの詳細は本 ADR のスコープ外（後続 ADR / Issue）。

## Feature Data Classification
- Feature 名: community node content index（topic-scoped post 本文テキスト + media 派生タグ + room メタデータ）
- Durable / Transient: Durable な node-local server index state（再構築可能な derived projection）
- Canonical Source: index は canonical ではない。canonical source は author-owned のまま（post 本文は topic replica、room は `live/<id>/state` / `game/<id>/state` の docs pointer + manifest blob）。index は派生した検索用テキスト + media 派生タグ + メタデータ + safety verdict state のみを持つ
- Replicated?: No（index は node-local。client へ canonical として replicate しない。検索/発見結果は node の authority scope 内で API として提供する）
- Rebuildable From: topic replica の post 本文 + room state pointer + safety scan（VLM タグ含む）の verdict。再 ingest + 再 scan で再構築できる
- Public Replica / Private Replica / Local Only: node-local な private server state（`cn-core` / Postgres）。public manifest には capability の有無のみを載せ、index 中身は載せない
- Gossip Hint 必要有無: ingestion path（§6, 未決）に依存。Model B/C を採る場合は supported topic の gossip hint / docs replica を ingest 入力として使う
- Blob 必要有無: No（**no permanent blob storage** を維持。raw media blob は index しない。index は VLM 由来の派生タグのみ。moderation server の一時 fetch のみ）
- SQLite projection 必要有無: No（server は SQLite を使わず Postgres projection）
- 必須 contract:
  - `index_scope_limited_to_operator_supported_topics`
  - `index_rejects_topic_outside_supported_set`
  - `index_admits_approved_user_indexing_request`
  - `index_text_post_body_only`
  - `index_media_searchable_via_derived_tags`
  - `index_excludes_raw_media_blob`
  - `index_streaming_room_metadata_only_excludes_in_room_comments_actions`
  - `index_metaverse_room_metadata_only_excludes_in_room_activity`
  - `index_only_allow_verdict_content`
  - `index_excludes_unscanned_and_scan_failed`
  - `search_discovery_recommendation_excludes_non_allow`
  - （ingestion path 確定後に追加: 受信経路ごとの consent / scope 境界 contract）
- 必須 scenario:
  - operator が topic を supported set に追加 → その topic の post 本文が検索に出る / supported 外 topic の post は検索に出ない
  - 画像のみの投稿は、本文が無くても VLM 派生タグで検索でき、raw blob 自体は index されない
  - exclude された media のタグは index されない（`allow` media のタグのみ検索に出る）
  - streaming / metaverse room は title / description / タグで検索/発見に出るが、その room 内のコメント・action は出ない
  - トピック毎の検索窓で topic 内検索ができ、CN 横断検索は別画面で supported topic 全体を横断する
  - unscanned / scan_failed / `allow` 以外の verdict の content は search / discovery / recommendation に出ない

## 1. 背景

- 現状 `community_index` / `moderation` / `community_local_trust` は `Availability::Planned`（spec のみ）で、index / search / discovery / recommendation の実体が無い。
- index は content-surfacing path（content を公開的に浮上させる経路）であり、安全設計が無いまま解禁してはならない（critical-safety.md §1）。
- index を「無制限に何でも拾う」設計にすると、(a) operator が責任を負えない範囲まで authority scope が膨張し、(b) media を含む有害 content の surfacing 経路が広がる。本 ADR はこれを scope と content kind の両面で**絞る**ことを既定とする。

## 2. Decision

### 2.1 index は node-local な derived projection（canonical ではない）

- index は **issuer node の authority scope 内**に閉じた node-local projection である。canonical source（author-owned の post / room）を所有・改変しない。
- index を消すことは canonical content を消すことを意味しない。index への登録/除外は「この node が自分の検索・発見・推薦にこの content を出すか」という node-local 判断に限定される（p2p-first responsibility boundary に従う）。
- user identity / profile / social graph は node-independent であり、index 化の対象としても canonical を所有しない。

### 2.2 無制限な indexing の防止 — scope は operator-supported topics に限定し、user indexing request も受ける

- index 対象は **operator が指定した supported topic の集合**に限定する。supported set 外の topic の content は index しない（`index_scope_limited_to_operator_supported_topics` / `index_rejects_topic_outside_supported_set`）。
- supported topic set は node-local な運用 state として持ち、運営 UI / CLI から変更できる（admin 画面の「サポートする topic を選択する機能」= #382 と接続）。
- **user からの indexing request を受け付ける**。ユーザーは「この topic / この content を index してほしい」と要求できる。ただし request は **index を保証しない**:
  - request された topic を supported set に入れるかは operator policy の判断。
  - supported になっても、個々の content は §2.5 の安全ゲートを通過した `allow` のみが index される。
  - つまり `request → operator 承認（supported 化）→ 安全 verdict 通過 → index` の多段ゲートとする（`index_admits_approved_user_indexing_request`）。
- この設計により、index の authority scope は「operator が明示的に引き受けた topic」に常に限定され、無制限に膨張しない。
- **index 対象は public topic に限らない。** 招待制（身内向け）CN（ADR 0024 admission）では **private channel に対する indexing request も想定**する。private channel の index は §6 考慮 1 の条件（channel capability の正当な保持 / scope を channel メンバー + その CN の authority に閉じる / visibility は `local` 寄り）を満たすことを必須とする。public topic（namespace 秘密が導出可能）と private channel（`channel::`、capability 必要）は取得経路が異なる（§6）。

### 2.3 index 対象の content kind — テキスト本文 + media の派生メタデータ（タグ）。raw blob は index しない

- index に格納・検索対象とするのは次に限る:
  - post 本文テキスト（`index_text_post_body_only`）
  - media の **派生メタデータ（タグ）**。media は moderation 過程で VLM（非決定論的 moderation, #411）を必ず通すため、そこで生成される descriptive tag / metadata を index し、media を **タグ経由で検索可能**にする（`index_media_searchable_via_derived_tags`）。
  - 最小限の共通メタデータ（post id / author pubkey / topic / timestamp / safety verdict state）
- **raw blob（画像 / 動画 / その他ファイルのバイト列）・知覚ハッシュ・サムネイルは index しない**（`index_excludes_raw_media_blob`）。index するのは VLM 由来の派生タグのみ。これは **no permanent blob storage**（critical-safety.md §3/§8）と整合する。
- 派生タグは `allow` verdict の media に対してのみ生成・index する。critical safety で exclude された media は index しない。CSAM 等の Match Data / 生検知結果はタグや index に流さない（#391 / #411 の非ゴールと整合）。
- **タグのサムネイル代替表示**: client は読み込み中、または安全用の代替表示（特にアダルト / 暴力的コンテンツ）として、サムネイルの代わりにこのタグを表示してよい。具体的な表示挙動は client UI 側の設計（本 ADR スコープ外）だが、index がタグを保持することで成立する。

### 2.4 streaming / metaverse — room メタデータのみ index する。room 内の activity は index しない

- streaming（live session, ADR 0005）/ metaverse・game room（ADR 0006）は、**room メタデータ**のみを index・検索対象にする。具体的には room id / topic / title / description テキスト / タグ。
  - room タグは現状未実装の可能性がある。実装され次第 index 対象に含める（未実装の間は title / description テキストを対象にする）。
- room メタデータのテキスト・タグは検索可能にする（room を見つけるための検索）。
- **room 内のコメント・chat・action・score・presence などの in-room activity は index しない**（`index_streaming_room_metadata_only_excludes_in_room_comments_actions` / `index_metaverse_room_metadata_only_excludes_in_room_activity`）。in-room の逐次 activity は discovery / search / recommendation の対象にしない。

### 2.5 安全設計 — fail-closed。`allow` verdict の content のみ index する

index への登録は、**scope ゲート（§2.2/§2.3/§2.4）と safety ゲート**の両方を通過した content のみとする。safety ゲートは critical-safety.md §8 の fail-closed 不変条件を index 本体の制約として固定する:

- scan 前（unscanned）の content は index しない（`index_excludes_unscanned_and_scan_failed`）。
- scan failure / provider unavailable は `allow` に倒さない。fail-closed（index しない）。
- `hold` / `quarantine` / `exclude` verdict の content は search で返さない。
- critical verdict は discovery / recommendation に入れない。
- `allow` verdict の content のみが search / discovery / recommendation に出る（`index_only_allow_verdict_content` / `search_discovery_recommendation_excludes_non_allow`）。

safety verdict は `cn-safety-runtime` の `SafetyVerdict` を index 反映の前段に組み込むことで供給する。index entry は対応する safety verdict state を必ず伴う（verdict 無しの index entry を作らない）。

### 2.6 二重ゲートの順序

content が index に入る条件は次の AND とする:

1. **scope ゲート**: topic が supported set 内（operator 指定 or 承認済み user request）であり、content kind が index 可能（post 本文テキスト / media 派生タグ / room メタデータ）であること。
2. **safety ゲート**: safety verdict が `allow` であること（unscanned / scan_failed / provider_unavailable / 非 `allow` は不可）。

どちらか一方でも満たさない content は index されず、search / discovery / recommendation に出ない。

### 2.7 検索 UX — トピック毎の検索窓を基本とし、CN 横断検索は別画面

- 基本 UX は **トピック毎の検索窓**（topic-scoped search）。あるトピックの中で検索するのが既定の体験。
- **CN ベースのトピック横断検索（cross-topic search）は別画面**として用意する。supported topic set 全体を対象に、node の authority scope 内で横断検索する。
- どちらの検索面も §2.2 の supported-topic scope と §2.5 の safety ゲートに従う（横断検索でも supported 外 topic / 非 `allow` content は出ない）。

## 3. Consequences

- index は「拾えるものを全部拾う」のではなく、operator が引き受けた supported topic × index 可能な content kind × `allow` verdict の交差に限定される。authority scope が常に説明可能になる。
- media 検索は VLM 由来の派生タグ経由で提供する（raw blob の内容検索・画像類似検索は提供しない）。media タグ生成は非決定論的 moderation（#411）に依存する。
- streaming / metaverse の検索体験は「room を見つける」までで（title / description / タグ）、room 内発言の全文検索は提供しない。
- index 本体（schema / storage / query boundary）と fail-closed 制約の実装は #404 が担う。本 ADR は #404 が満たすべき contract / scenario の必要集合を先に固定する。
- `CommunityIndex` capability の `Availability::Planned` → 昇格は、§2 の scope/content/safety ゲートと critical-safety.md §11 readiness（known-CSAM provider 設定・signed event 有効・blob 恒久保存無効 等）が実装・テストで満たされた段階で別途判断する（本 ADR では昇格しない）。

## 4. Out of scope（後続へ申し送り）

- trust / relation reads への risk signal 反映経路（#406 / trust-relation ADR #409）。
- 決定論的 / 非決定論的 moderation の verdict 生成詳細（#410 / #411）。
- ranking / recommendation アルゴリズム、関連度スコアリングの具体。
- 画像類似検索 / raw blob 内容検索（タグ経由の検索は本 ADR で扱う）。
- tag 語彙（tag vocabulary）の標準化と VLM タグ生成の詳細（#411）。
- public manifest での index capability の表現詳細（capability メタデータ更新）。

## 5. 維持する境界

- index は node-local projection であり network-wide な canonical store ではない。
- index への登録/除外は node-local 判断であり、他 node や user の canonical state を変更しない。
- **no permanent blob storage** を維持する（raw media blob を index に取り込まない。index するのは VLM 由来の派生タグのみ）。
- `allow` 以外、unscanned、scan_failed、provider_unavailable を surfacing 経路に出さない（fail-closed）。

## 6. 未決事項: 投稿・room 情報の取得経路（ingestion path）— 要決定

index する content（post 本文・media タグ・room メタデータ）を、**community node がどこから / 誰から / どの経路で受け取るか**を本 ADR で確定する必要がある。これは authority scope・consent・P2P-first 境界・no permanent blob storage に直結する設計情報であり、未確定のままでは #404 の実装境界（schema / ingest / scan 順序）が定まらない。

前提（AGENTS.md 通信経路）: 基本優先度は `Direct P2P -> Relay Supported P2P -> Relay Fallback`。`cn-user-api` が topic rendezvous state の owner、`cn-iroh-relay` は純 iroh relay。community node は P2P network の **service provider** であって home server ではない。

候補モデル:

### Model A: client が cn-user-api へ明示 submit（dual-publish）

- client が gossip publish と並行して、index 許可付きで post を CN の API に submit する。
- 長所: opt-in / 明示 consent が明確。CN は送られたものだけを index。supported 外 topic への要求も submit ベースで扱える。
- 短所: publish 経路が二重化。opt-in した client の投稿しか集まらず「topic を広く index する」用途には届かない。CN が特別な publish 先になり P2P-first から外れ気味。

### Model B: CN が gossip 参加者として受信（good-citizen relay）

- CN が supported topic の gossip を subscribe し、通常の P2P 経路で post を受信。受信した post は次の peer へ forward する（sink ではなく Relay Supported P2P の良き参加者）。
- 長所: 既存 P2P 経路に乗る。supported topic の public 投稿を広く拾える。「P2P network の service provider」という位置づけと整合。
- 短所: CN は subscribe した topic の public gossip を広く見る（scope / privacy 配慮が要る。supported topic に限定して境界化する）。

### Model C: CN が topic の public docs replica を複製（canonical pull）

- CN が supported topic の author / topic docs replica（iroh-docs）を canonical として sync し、gossip は hint としてのみ使う（ADR 0005/0006 の `docs state + manifest blob` モデルと整合）。
- 長所: canonical な post 本文・room manifest を確実に取得。late-join backfill / restart 復元が docs だけで成立。
- 短所: docs 複製の対象範囲（どの author / topic replica を引くか）の管理が要る。

### 組み合わせと決定の反映

組み合わせ（例: supported topic は Model C で canonical 取得 + Model B で liveness、opt-in / 例外は Model A）も選択肢。どのモデル（または組み合わせ）を既定とするかは設計判断であり、**決定後に本 ADR の §2 Decision と Feature Data Classification（Canonical Source / Replicated / Rebuildable From / Gossip Hint）へ反映し、ingestion ごとの consent / scope 境界 contract を追加する**。

### 取得経路の追加考慮事項（決定前に織り込む）

#### 考慮 1: 招待制（身内向け）CN と private channel indexing

- 現状 CN は admission（`open` / `invite` / `whitelist`、ADR 0024）を実装済みで、**「招待したユーザーのみ許可」**する運用を認めている。これは public CN を立てることの**法的ハードルの高さ**を踏まえ、よりハードルの低い**身内向け CN** を想定したため。
- したがって index は public topic だけでなく、**private channel に対する indexing request も当然想定する必要がある**。「supported topic に限定」という §2.2 の scope 前提を、private channel index 対象を含む形に拡張する。
- 技術的含意（`crates/docs-sync/src/access.rs`）: private channel replica（`channel::`）は namespace 秘密が topic id から**導出できず**、capability（登録済み secret）が必要。public topic replica（導出可能）とは取得経路が根本的に異なる。
  - したがって private channel の index は (a) その channel の capability を CN が正当に保持していること、(b) scope / consent が **channel メンバー + その CN の authority** に閉じること、(c) risk signal / 露出 visibility が `local` 寄りであること、を満たす必要がある。
  - Model 適合: capability を伴う Model A（メンバーが capability 付きで submit / CN に capability 登録）と相性が良い。Model B/C で private channel を扱う場合も、public derive では取得できないため capability 注入が前提になる。
- 身内向け CN の index は「その CN の admission を通ったメンバー向けの node-local 検索」であり、network-wide 公開ではない。p2p-first responsibility boundary / trust-semantics と整合させる。

#### 考慮 2: relay 抜き CN を許容する

- 現状の CN 要件は **relay 抜き CN**（`cn-iroh-relay` を持たない構成）を許容している。よって ingestion 設計を「CN 自身が relay を持つこと」に依存させてはならない。
- 技術的含意: Model B/C の peer discovery を CN-local relay 前提にできない。relay 無し CN では、docs/gossip 参加ノードの peer discovery を別経路で成立させる必要がある（imported ticket / seed peer / 外部 relay / DHT discovery 等。`iroh_sync.rs` の `seed_peers` / `learned_peers` / `imported_peers` と整合）。
- つまり「CN を docs 参加ノードにする（Model B/C）」コストには、**relay 有無による discovery 経路差**が加わる。relay 無し構成でも canonical sync が成立する discovery を設計に含めること。
- Model A（client が cn-user-api へ submit）は HTTP 面のみで relay/参加ノードに依存しないため、relay 抜き CN との相性は最も良い。

これら 2 点は、§2.2 の scope（public topic 限定 → private channel を含む）と、Model 選択（参加ノード化のコスト / relay 依存の排除）に直接効く。決定時に上記を満たすことを必須とする。
