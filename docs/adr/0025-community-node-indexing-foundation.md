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
- `docs/adr/0027-deterministic-moderation-critical-safety.md`（§2 component boundary / data flow / fail-closed invariants / §2.9 readiness。旧 `community-node-critical-safety.md` を集約）
- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`（authority scope）
- `docs/adr/0027-deterministic-moderation-critical-safety.md`（§2.1 advisory ≠ command。旧 `moderation-event-trust-semantics.md` を集約）
- `crates/cn-operator/src/capability.rs`（`CommunityIndex` = `Availability::Planned`）
- 実装側 Issue: #404（fail-closed community indexing 本体）
- 後続 ADR: trust / relation foundation（#409）, 決定論的 moderation（#410）, 非決定論的 moderation（#411）

## 位置づけ

community node の主要未検討機能のひとつ「indexing（index / search / discovery / recommendation）」の責務境界を固定する foundation ADR である。`cn-core` に index の実体はまだ無く（実装は #404）、現状 fail-closed 不変条件は決定論 moderation ADR（`docs/adr/0027-deterministic-moderation-critical-safety.md` §2.4）に集約されている。本 ADR は、その土台として **何を index し、何を index しないか**を先に確定し、その上に trust/relation（#409）・moderation（#410/#411）を載せられるようにする。

本 ADR は indexing の **scope（範囲）と content kind（対象種別）と fail-closed（安全）** の境界を定義する。trust / relation への risk signal 反映、provider 接続、ranking/recommendation アルゴリズムの詳細は本 ADR のスコープ外（後続 ADR / Issue）。

## Feature Data Classification
- Feature 名: community node content index（topic-scoped post 本文テキスト + media 派生タグ + room メタデータ）
- Durable / Transient: Durable な node-local server index state（再構築可能な derived projection）
- Canonical Source: index は canonical ではない。canonical source は author-owned のまま（post 本文は topic replica、room は `live/<id>/state` / `game/<id>/state` の docs pointer + manifest blob）。CN は ingestion = Model C（§6）で sync 元の topic / channel docs replica（public は導出 namespace、private は登録 capability）から取り込む。index は派生した検索用テキスト + media 派生タグ + メタデータ + safety verdict state のみを持つ
- Replicated?: No（index 自体は node-local。client へ canonical として replicate しない）。CN は supported topic / 許可 channel の **docs sync peer として参加**する（§6 Model C）。検索/発見結果は node の authority scope 内で API として提供する
- Rebuildable From: sync した topic / channel replica の post 本文・room state pointer + safety scan（VLM タグ含む）の verdict。再 ingest + 再 scan で再構築でき、docs+blob で backfill / restart 復元する
- Public Replica / Private Replica / Local Only: node-local な private server state（`cn-core` / Postgres）。public manifest には capability の有無のみを載せ、index 中身は載せない
- Gossip Hint 必要有無: peer discovery 補助として使用（Model C, §6）。基本データ経路は docs replica sync
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
  - `index_only_indexes_shared_replica_entries`
  - `index_private_channel_requires_submitted_channel_secret`
  - `index_private_channel_request_authz_reuses_channel_permission`
  - `indexing_startup_requires_validated_relay`
  - `indexing_startup_fails_without_own_or_external_relay`
- 必須 scenario:
  - operator が topic を supported set に追加 → その topic の post 本文が検索に出る / supported 外 topic の post は検索に出ない
  - 画像のみの投稿は、本文が無くても VLM 派生タグで検索でき、raw blob 自体は index されない
  - exclude された media のタグは index されない（`allow` media のタグのみ検索に出る）
  - streaming / metaverse room は title / description / タグで検索/発見に出るが、その room 内のコメント・action は出ない
  - トピック毎の検索窓で topic 内検索ができ、CN 横断検索は別画面で supported topic 全体を横断する
  - unscanned / scan_failed / `allow` 以外の verdict の content は search / discovery / recommendation に出ない
  - private channel の secret を提示した indexing リクエストで `channel::` replica が sync され検索に出る / secret 無しでは index されない
  - 自前 relay も外部 relay も未設定の CN は indexing 起動に失敗する（fail-closed）
  - 共有 replica に実在しない content（CN へ直接渡されただけ）は index されない

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
- **index 対象は public topic に限らない。** 招待制（身内向け）CN（ADR 0024 admission）では **private channel に対する indexing request も想定**する。private channel の index は §6.3 の条件（indexing リクエスト＝secret 送信 / リクエスト権限は channel 権限モデルの応用 / scope を channel メンバー + その CN の authority に閉じる / visibility は `local` 寄り）を満たすことを必須とする。public topic（namespace 秘密が導出可能）と private channel（`channel::`、capability 必要）は取得経路が異なる（§6）。

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
- ingestion = Model C を基本とするため、CN は supported topic / 許可 channel の **docs sync participant node を新たに常駐**させる（純 relay + KV rendezvous の現行構成を一段拡張）。indexing 起動時に **relay validation** を gate として通し、自前 relay が無ければ外部 relay 設定を必須化する。Model B / A は Appendix（optional）。
- private channel の indexing は「indexing リクエスト＝secret 送信」で C に capability を注入して解決し、別 ingestion 経路を新設しない。リクエスト権限は channel の権限モデルを応用する。
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

## 6. 投稿・room 情報の取得経路（ingestion path）= Model C を基本とする（Decision）

index する content（post 本文・media タグ・room メタデータ）を community node がどこから / 誰から / どの経路で受け取るかを、**Model C（topic / channel の docs replica を sync する canonical pull）を基本（required）**として確定する。Model B / Model A は **Appendix A（実装してもよいが必須ではない optional）**とする。

前提（AGENTS.md 通信経路）: 基本優先度は `Direct P2P -> Relay Supported P2P -> Relay Fallback`。`cn-user-api` が topic rendezvous state の owner。community node は P2P network の **service provider** であって home server ではない。

### 6.1 なぜ C を基本にするか（cost / attack surface）

- **コスト**: 総コストは safety scan（特に media VLM）が支配し、これは ingestion 経路に依らず共通。ingestion の差は相対的に小さい。replica sync は iroh-docs の **delta 同期（range 突合）**で per-post を償却し、late-join backfill / restart 復元が docs+blob だけで成立する（ADR 0005/0006）。「常駐プロセスが無い」ことを A の優位とみなすのは誤り（`cn-user-api` は既に常駐し、A は backfill 不能）。
- **attack surface（ghost 注入）**: post envelope の署名検証は self-contained（`KukuriEnvelope::verify`、pubkey 埋め込み schnorr）。しかし有効署名は「実際に共有 replica / gossip に存在した投稿」であることを証明しない。Model A は **CN に直接 POST された、ネットワークに流れていないゴースト投稿**を index し得る（CN 限定の私的注入面を新設し、index がネットワーク実体から乖離する）。Model C は **全参加者と同じ共有 replica の entry のみ**を index するため、この私的注入面を作らない。
- 結論: C は index を共有実体に一致させ、A 固有の注入面を持たず、per-post コストも償却できる。

### 6.2 Model C の決定内容

- **public topic**: `topic_replica_id(topic)`（= `topic::<topic_id>`）から namespace を導出し、`public_replica_secret` で replica を open、peer を discovery して iroh-docs sync する（`crates/docs-sync/src/{access,iroh_sync}.rs`）。
- **ghost 注入を作らない**: index 対象は **sync された共有 replica に実在する entry のみ**。CN に直接渡されただけで replica に存在しない content は index しない（`index_only_indexes_shared_replica_entries`）。
- **blob**: scan のための一時 fetch のみ。恒久保存しない（no permanent blob storage）。

### 6.3 課題 1 の解決: private channel indexing = secret 送信

- 招待制（身内向け）CN（ADR 0024 admission）では private channel への indexing request も想定する。private channel replica（`channel::`）は namespace 秘密が導出できず capability（登録済み secret）が必要（`access.rs`）。
- **解決**: **indexing リクエスト＝secret 送信**とする。リクエスト時に channel secret（capability）を CN に渡し、CN はそれを登録して **Model C と同じ仕組みで `channel::` replica を sync** する。private channel 専用の別 ingestion 経路は新設せず、C に capability を注入するだけで解決する。
- **リクエスト権限**: indexing をリクエストできる権限は **channel の既存権限モデルをそのまま応用**する（channel の secret にアクセスできる権限者が、その secret を提示して indexing をリクエストできる）。CN は新しい権限体系を作らない。
- scope / consent: private channel index は channel メンバー + その CN の authority に閉じ、risk signal / visibility は `local` 寄り（trust-semantics）。
- contract: `index_private_channel_requires_submitted_channel_secret` / `index_private_channel_request_authz_reuses_channel_permission`。

### 6.4 課題 2 の解決: relay validation（relay 抜き CN）

- CN は relay 抜き構成を許容するため、Model C の peer discovery を CN-local relay 前提にできない。
- **解決**: **indexing 起動時に relay を validate する**。自前 relay（`cn-iroh-relay`）が無い構成では、**外部 relay の設定を必須化**する。relay（自前 or 外部）が未設定なら indexing を起動しない（fail-closed の起動 gate）。
- これにより relay 有無に依らず C の discovery（seed peer / imported ticket / 外部 relay / DHT。`iroh_sync.rs` の `seed_peers` / `learned_peers` / `imported_peers`）が成立する。
- contract: `indexing_startup_requires_validated_relay` / `indexing_startup_fails_without_own_or_external_relay`。

### 6.5 Feature Data Classification への反映（C 確定）

- Canonical Source: index は派生。canonical は sync 元の topic / channel docs replica（public は導出 namespace、private は登録 capability）。
- Replicated?: index 自体は node-local。CN は supported topic / 許可 channel の **docs sync peer として参加**する。
- Rebuildable From: sync した replica（topic / channel）+ safety scan。docs+blob で backfill / restart 復元。
- Gossip Hint: peer discovery 補助として使用。基本データ経路は docs replica sync。

## Appendix A: 代替・補助 ingestion モデル（B / A, optional）

以下は Model C を補完する optional モデル。**実装してもよいが必須ではない。** 採用する場合も §6.1–§6.4 の不変条件（ghost 注入を作らない / relay validation / private は capability）を満たすこと。

### Model B: CN が gossip 参加者として受信（liveness 補助, optional）

- CN が supported topic の gossip を subscribe し、新着の liveness hint を得て次 peer へ forward する（sink ではなく Relay Supported P2P の良き参加者）。C の canonical sync を低遅延化する補助。
- index は依然 §6.2 の「共有 replica の entry のみ」に従う（gossip だけで来た未 replica content を直接 index しない）。

### Model A: client が cn-user-api へ明示 submit（opt-in 例外, optional）

- client が index 許可付きで post を CN の API に submit する opt-in 経路。
- 必須ではない。採用する場合、ghost 注入を防ぐため **submit された content も共有 replica 上の実在を確認してから index する**（単独 POST だけで index しない）。確認できないなら index しない。
