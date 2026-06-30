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
- Feature 名: community node content index（topic-scoped post 本文テキスト + room メタデータ）
- Durable / Transient: Durable な node-local server index state（再構築可能な derived projection）
- Canonical Source: index は canonical ではない。canonical source は author-owned のまま（post 本文は topic replica、room は `live/<id>/state` / `game/<id>/state` の docs pointer + manifest blob）。index は派生した検索用テキスト + メタデータ + safety verdict state のみを持つ
- Replicated?: No（index は node-local。client へ canonical として replicate しない。検索/発見結果は node の authority scope 内で API として提供する）
- Rebuildable From: topic replica の post 本文 + room state pointer + safety verdict。再 ingest + 再 scan で再構築できる
- Public Replica / Private Replica / Local Only: node-local な private server state（`cn-core` / Postgres）。public manifest には capability の有無のみを載せ、index 中身は載せない
- Gossip Hint 必要有無: No
- Blob 必要有無: No（**no permanent blob storage** を維持。media blob は index しない。moderation server の一時 fetch のみ）
- SQLite projection 必要有無: No（server は SQLite を使わず Postgres projection）
- 必須 contract:
  - `index_scope_limited_to_operator_supported_topics`
  - `index_rejects_topic_outside_supported_set`
  - `index_admits_approved_user_indexing_request`
  - `index_text_post_body_only`
  - `index_excludes_image_video_file_blobs`
  - `index_streaming_room_metadata_only_excludes_in_room_comments_actions`
  - `index_metaverse_room_metadata_only_excludes_in_room_activity`
  - `index_only_allow_verdict_content`
  - `index_excludes_unscanned_and_scan_failed`
  - `search_discovery_recommendation_excludes_non_allow`
- 必須 scenario:
  - operator が topic を supported set に追加 → その topic の post 本文が検索に出る / supported 外 topic の post は検索に出ない
  - 画像のみ / 動画のみの投稿は検索に出ない（本文テキストがある投稿のみ本文が index される）
  - streaming / metaverse room は検索/発見に出るが、その room 内のコメント・action は出ない
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
  - supported になっても、個々の content は §2.4 の安全ゲートを通過した `allow` のみが index される。
  - つまり `request → operator 承認（supported 化）→ 安全 verdict 通過 → index` の多段ゲートとする（`index_admits_approved_user_indexing_request`）。
- この設計により、index の authority scope は「operator が明示的に引き受けた topic」に常に限定され、無制限に膨張しない。

### 2.3 index 対象の content kind — テキスト（投稿本文）のみ。media は index しない

- **blob はテキスト（投稿本文）のみ index する。** 画像 / 動画 / その他ファイルの blob は index しない（`index_text_post_body_only` / `index_excludes_image_video_file_blobs`）。
- index に格納・検索対象とするのは post 本文テキストと最小限のメタデータ（post id / author pubkey / topic / timestamp / safety verdict state）に限る。
- media blob のバイト列・知覚ハッシュ・サムネイル等を検索インデックスに入れない。これは **no permanent blob storage**（critical-safety.md §3/§8）と整合する。
- 注: media を含む投稿でも、index されるのは本文テキストのみ。media 自体は safety scan の対象ではあるが（surfacing 前に scan される）、検索可能な index entry にはしない。本文テキストが無い（media のみ）投稿は index entry を持たない。

### 2.4 streaming / metaverse — room のみ index する。room 内の activity は index しない

- streaming（live session, ADR 0005）/ metaverse・game room（ADR 0006）は、**room そのもの**のみを index する。具体的には room の存在 + room メタデータ（room id / topic / title / description テキスト）に限る。
- **room 内のコメント・chat・action・score・presence などの in-room activity は index しない**（`index_streaming_room_metadata_only_excludes_in_room_comments_actions` / `index_metaverse_room_metadata_only_excludes_in_room_activity`）。
- room メタデータのテキスト（title / description）は §2.3 のテキスト規則に従い index 可能だが、in-room の逐次 activity は discovery/search/recommendation の対象にしない。

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

1. **scope ゲート**: topic が supported set 内（operator 指定 or 承認済み user request）であり、content kind が index 可能（post 本文テキスト / room メタデータ）であること。
2. **safety ゲート**: safety verdict が `allow` であること（unscanned / scan_failed / provider_unavailable / 非 `allow` は不可）。

どちらか一方でも満たさない content は index されず、search / discovery / recommendation に出ない。

## 3. Consequences

- index は「拾えるものを全部拾う」のではなく、operator が引き受けた supported topic × index 可能な content kind × `allow` verdict の交差に限定される。authority scope が常に説明可能になる。
- media 検索（画像/動画そのものの検索）は v1 indexing では提供しない。media の安全な surfacing は別途設計が必要（本 ADR スコープ外）。
- streaming / metaverse の検索体験は「room を見つける」までで、room 内発言の全文検索は提供しない。
- index 本体（schema / storage / query boundary）と fail-closed 制約の実装は #404 が担う。本 ADR は #404 が満たすべき contract / scenario の必要集合を先に固定する。
- `CommunityIndex` capability の `Availability::Planned` → 昇格は、§2 の scope/content/safety ゲートと critical-safety.md §11 readiness（known-CSAM provider 設定・signed event 有効・blob 恒久保存無効 等）が実装・テストで満たされた段階で別途判断する（本 ADR では昇格しない）。

## 4. Out of scope（後続へ申し送り）

- trust / relation reads への risk signal 反映経路（#406 / trust-relation ADR #409）。
- 決定論的 / 非決定論的 moderation の verdict 生成詳細（#410 / #411）。
- ranking / recommendation アルゴリズム、関連度スコアリングの具体。
- media 検索（画像/動画の検索）と、その安全な surfacing 設計。
- public manifest での index capability の表現詳細（capability メタデータ更新）。

## 5. 維持する境界

- index は node-local projection であり network-wide な canonical store ではない。
- index への登録/除外は node-local 判断であり、他 node や user の canonical state を変更しない。
- **no permanent blob storage** を維持する（media blob を index に取り込まない）。
- `allow` 以外、unscanned、scan_failed、provider_unavailable を surfacing 経路に出さない（fail-closed）。
