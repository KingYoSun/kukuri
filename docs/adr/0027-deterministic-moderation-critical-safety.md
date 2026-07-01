# ADR 0027: Deterministic Moderation (CSAM / Known-Hash Critical Safety)

## Status
Draft

## Date
2026-06-30

## Base Branch
`main`

## Related
- `docs/safety/community-node-critical-safety.md`（§3 goals / §4 non-goals / §5 component boundary / §6 data flow / §7 verdict model / §8 fail-closed / §9 signed events / §10 reporting・appeal / §11 readiness / §12 prerequisites）
- `docs/architecture/moderation-event-trust-semantics.md`（advisory ≠ command / authority scope / visibility。本 ADR に折り込む moderation 原則の出所）
- `docs/architecture/p2p-first-community-node-responsibility-boundary.md`（service provider / authority scope）
- `docs/adr/0025-community-node-indexing-foundation.md`（fail-closed indexing gate = `allow` のみ surfacing）
- `docs/adr/0026-community-node-trust-relation-foundation.md`（絶対指標 = 決定論 + 厳格非決定論。本 ADR は絶対指標の決定論側）
- `crates/cn-safety/src/{verdict,policy,provider,capability,event,signal}.rs`（実装済み domain）
- `crates/cn-safety-runtime/src/`（`SafetyOrchestrator` / `SystemScanClock` / `UuidEventIdGenerator` / `verify_signed_event`）
- `crates/cn-core/src/safety_events.rs`（署名 event / risk signal 永続化・配布, #405）
- Issue: #353（safety foundation, closed）, #391（Project Arachnid Shield）, #404（fail-closed indexing 本体）, #405（永続化・実鍵署名）, #406（runtime 結線・risk signal 反映）, #411（非決定論的 moderation）, #410（本 ADR）

## 位置づけ

community node の moderation のうち **決定論的 moderation（既知 hash / provider-verdict による CSAM critical safety）** の設計を、番号付き ADR として整理・集約する。これまで normative な内容は `docs/safety/community-node-critical-safety.md` と `docs/architecture/moderation-event-trust-semantics.md` に散在し、ADR ではないため「設計ドキュメントを ADR と誤認する」混乱があった。本 ADR がその decision record を担い、既存 doc は elaboration / 出所として参照する（§6）。

本 ADR のスコープは **決定論的検知（既知 hash 一致 = `Basis::KnownHashMatch` による confirmed）**と、その上に載る **共通の verdict / fail-closed / signed event / visibility / provider abstraction フレームワーク**である。classifier ベースの **非決定論的検知（`Basis::ClassifierScore` = suspected / VLM）**の詳細は #411 に委ね、本 ADR は両者が差さる共通枠組みと決定論側を固定する。

実装は既に `cn-safety`（domain / policy）・`cn-safety-runtime`（orchestration）・`cn-core`（永続化, #405）に存在する。本 ADR はそれらを normative な決定として記録する。

## Feature Data Classification
- Feature 名: deterministic moderation verdict（known-hash CSAM confirmed）+ signed moderation event
- Durable / Transient: verdict は transient な純関数結果。signed moderation event / risk signal は durable な server state（#405, `cn_safety.signed_moderation_events` / `cn_safety.risk_signals`）
- Canonical Source: signed event = issuer node が署名した record（`cn-core` Postgres `cn_safety` schema）。verdict = provider scan 結果を policy router（純関数 `route()`）に通した派生
- Replicated?: signed event は visibility に従い配布（`subscribed_nodes` / `public`）。client への canonical replication はしない
- Rebuildable From: provider scan + `route()` の再実行（verdict）。signed event は発行後 id 冪等で権威（`persist_signed_moderation_event` は最初の writer 優先）
- Public Replica / Private Replica / Local Only: node-local server state（`cn-core` / Postgres）。配布は visibility 規則
- Gossip Hint 必要有無: No
- Blob 必要有無: No（**no permanent blob storage**。moderation server は scan のため一時 fetch のみ）
- SQLite projection 必要有無: No（server は Postgres）
- 必須 contract（大半は #353 / #405 で実装済み。本 ADR で normative に固定）:
  - `deterministic_confirmed_requires_known_hash_match`
  - `no_known_match_is_not_clean`
  - `fail_closed_never_allows`（unscanned / scan_failed / provider_unavailable）
  - `only_allow_verdict_is_indexable`
  - `known_csam_provider_required_else_fail_closed`
  - `critical_route_separated_from_general`
  - `critical_detection_not_downgraded_below_threshold`
  - `signed_event_verified_before_persist`
  - `suspected_defaults_local_visibility`
- 必須 scenario（実装済みテスト群を decision として維持）:
  - known hash match → `Exclude` / `CsamConfirmed` / `critical=true` / `is_indexable()=false`
  - scan failure / provider unavailable / unscanned → `Allow` にならない（fail-closed）
  - `require_known_csam=true` で known CSAM capability の結果が無ければ `ProviderUnavailable` に倒れる
  - 改竄 / 別鍵 / issuer 詐称の signed event は永続化・配布されない（`verify_signed_event`）

## 1. 背景

- `community_index` / `moderation` / `community_local_trust` は `cn-operator` で `Availability::Planned`。public indexing 解禁前に fail-closed な critical safety を **architecture constraint** として満たす必要がある（critical-safety.md §1）。
- kukuri は P2P 基盤であり、network 全体を統治する中央 moderation authority は構造的に存在し得ない。moderation は各 node の authority scope 内の判断に限定される。
- 決定論的 = 既知 hash 一致 / provider confirmed（evidence ベース、通報数に依らず、community / culture に依らず絶対）。これは ADR 0026 の **絶対指標**の決定論側に対応する。

### 1.1 goals（critical-safety.md §3）
- CSAM / CSE 等の critical safety risk を、その node の index / discovery / recommendation / relation 出力から能動的に排除する。
- 排除を signed moderation event / risk signal で説明・監査可能にする。
- **個人 / 小規模 operator が有害メディアを手動で目視検査することに依存しない**（既知 hash / provider による自動判定でこれを避ける）。
- community node が blob 本体を恒久保存しない前提を維持する。

### 1.2 non-goals（critical-safety.md §4）
- CSAM hash データベースを自前でホスト / 配布しない。
- 自前の CSAM 検知モデルを学習しない。
- operator に CSAM / CSE メディアの手動レビューを要求しない。
- kukuri project / default node を network-wide moderation authority として位置づけない（P2P 基盤上に成立し得ない）。
- general NSFW moderation を CSAM / CSE critical route と同じ route にまとめない。

## 2. Decision

### 2.1 advisory ≠ command / authority scope（moderation 原則）
- moderation event / risk signal は **issuer node が署名**し、効果は issuer node の **authority scope** に限定される。network-wide command ではない（`moderation-event-trust-semantics.md` を本 ADR に折り込む）。
- user identity / profile / social graph は node-independent。moderation はこれらの canonical を所有・凍結・削除しない。
- default node も global moderation authority ではない。provenance 不明時に default node へ帰属させない。
- **受け手の採用は opt-in**（trust-semantics §3）: 他 node の moderation event / risk signal は、受け手が購読していない issuer のものを自動適用しない。採用は受け手の判断であり、効果は受け手自身の出力に閉じる（他 node / user の canonical state を変更しない）。これが「optional trust input」の実効的意味である。

### 2.2 verdict model（label と action の分離、confirmed と suspected の分離）
- action は `SafetyAction`（`allow` / `hold` / `quarantine` / `exclude`）。**`allow` のみが index / discovery / recommendation へ surfacing を許す**（`SafetyAction::allows_indexing` / `SafetyVerdict::is_indexable`）。
- 検知ラベル `SafetyLabel`（category / confidence / provider_capability）は action と独立。
- `ReasonCode` で `CsamConfirmed`（**決定論的**: known hash match / provider confirmed）と `CsamSuspected` / `CseSuspected`（classifier = 非決定論、#411）を型で分離する。
- **router が confirmed CSAM に付ける basis は `Basis::KnownHashMatch`**（`basis_for_reason(CsamConfirmed) → KnownHashMatch`）。`Basis::ProviderVerdict` は「provider が confirmed と返した」意味の basis だが、現状の `route()` は `ProviderVerdict` を `GeneralModeration` に割り当てており、provider-confirmed CSAM を confirmed basis として emit する経路は未整備（既知ギャップ。#391 provider 統合時に `basis_for_reason` と `default_visibility_for` の整合を取る）。`Basis::ClassifierScore` は suspected どまり（confirmed に昇格させない、#411）。

### 2.3 policy routing（critical と general の分離、純関数）
- `route(&[ProviderScanResult], &SafetyPolicy, scanned_at) -> SafetyVerdict` は純関数。`SafetyPolicy::public_node_default()` を public-node 既定とする。
- critical route（`SafetyCategory` の `Csam` / `Cse` / `Grooming` = `is_critical_safety`）と general route（`Nsfw` / `Spam` / `Malware` / `Phishing`）を分離する。同一 queue / reason code にまとめない。注: 「violence / hate / harassment」は provider が返し得る example label であり `SafetyCategory` の variant ではない（general moderation の入力例）。
- 本 ADR は critical / general の **route 分離**と決定論的 confirmed（known-hash）を owns する。grooming / CSE / 未知 CSAM の **suspected 検知は classifier ベース（非決定論）であり #411 が owns する**（本 ADR は route と fail-closed 枠組みのみを固定し、classifier 検知の詳細は持たない）。
- **`require_known_csam=true`**: `KnownCsamHashMatch` capability の scan 結果が無い場合、general が clean でも `ProviderUnavailable` として fail-closed する（mandatory known-CSAM provider 欠落防止）。
- **critical 取りこぼし防止**: critical capability / critical label を持つ `Completed` 検知は、閾値未満や `score=None` でも `Allow` / `Clean` に落とさない（`is_critical_detection` + 最終ガード）。suspected の実効スコアは `result.score` または critical label の最大 confidence（`effective_critical_score`）。
- **`NoKnownMatch` を `Clean` / safe と同一視しない**（known-hash provider のみでは「未知 CSAM は未検査」）。

### 2.4 fail-closed invariants（DB 制約 + テストで保証）
- unscanned media は index しない（`ReasonCode::Unscanned`）。
- scan failure / provider unavailable は `Allow` に倒さない（`ScanOutcome::is_fail_closed`、`ReasonCode::ScanFailed` / `ProviderUnavailable`）。
- `hold` / `quarantine` / `exclude` は search で返さない。critical verdict は discovery / recommendation に入れない。
- public-node profile は mandatory known-CSAM provider 欠落で readiness 失敗。
- **no permanent blob storage** を維持。
- これらは ADR 0025（indexing）の surfacing gate と同一の不変条件であり、`SafetyVerdict::is_indexable()` が単一判定点。

### 2.5 signed moderation events（issuer 署名・永続化・検証）
- `ModerationEventBody`（unsigned canonical）を issuer node が署名し `SignedModerationEvent` にする。実鍵署名は secp256k1 schnorr（#405）。`MockSigner`（非暗号 FNV-1a）は test 専用で production API に出さない。
- 永続化（`cn-core` `persist_signed_moderation_event`）は **保存前に `verify_signed_event` で署名検証**し、改竄 / 別鍵 / issuer 詐称を拒否する（配布クエリが常に検証済みを返す保証）。id 冪等（最初の writer が権威）。
- event の `ModerationAction` は `Exclude` / `Hold` / `Quarantine` / `RiskLabel`。`RiskLabel` は「強制排除等ではなく根拠つき risk label を付す」action であり、その内容は §2.6 の `SafetyRiskSignal`（target / category / basis / severity / confidence / visibility）として表現・配布される（event action ↔ signal payload の対応）。
- 事象記録・監査は event id / reference id / basis category を使い、有害コンテンツ本体を証拠として再配布しない。

### 2.6 risk signals + visibility
- `SafetyRiskSignal`（target / category / severity / basis / confidence / visibility / expiry / appeal）は断定ラベルではなく根拠つき advisory。
- visibility 3 段階 `Local` / `SubscribedNodes` / `Public`。**suspected unknown CSAM / CSE は既定 `Local`**、confirmed（known hash / provider verdict）のみ `SubscribedNodes` 以上を検討（誤検知を public advisory に拡散しない）。`default_visibility_for` に従う。
- confirmed（決定論）CSAM の risk signal は ADR 0026 の **trust 絶対成分**の入力の**一つ**になる（絶対成分は決定論 #410 + 厳格非決定論 #411 の両方が入る。決定論はその片方）。relation 非依存、report-bomb 不動。#406 が runtime で反映を結線する。

### 2.7 provider abstraction + capability（決定論の実体 / #391）
- **component boundary（critical-safety.md §5）**: community node は生コンテンツではなく最小参照（`media_hint` = hash / CID、`text`）を moderation server に渡す。moderation server が provider credentials を保持し、必要時に blob を一時 fetch（**恒久保存しない**）して provider / router を実行し verdict を返す。node は `allow` verdict のみ index に反映する。
- provider は boolean ではなく `SafetyProviderCapability` で表す。決定論的 confirmed を生み得るのは `KnownCsamHashMatch`（`can_confirm_known_csam`）。`PerceptualHashMatch` は critical だが near-match（confirmed ではない）。
- 通報ワークフローは `SafetyProviderCapability::ReportingWorkflow` として表す。known CSAM confirmed の route は exclude + critical event + risk signal に加え、適用可能な場合 **reporting workflow hook** を含む（critical-safety.md §7）。
- `SafetyProvider`（async trait: name / capabilities / scan）。`ProviderScanRequest` は生コンテンツではなく最小参照（`media_hint` = hash / CID、`text`）を渡す（no permanent blob storage 前提）。`ScanError`（Unavailable / Timeout / Protocol）は `ScanOutcome::Failed` / `Unavailable` に写像して fail-closed。
- **#391 Project Arachnid Shield**（operator-owned credentials, known-match provider）を決定論的 provider 実装として差す。制約: Match Data を長期保存 / 公開 event / P2P / AI pipeline に流さない。`No Known Match` を安全確認済みとして扱わない。
- mock provider（`mock` feature）で決定論的テストを担保、本番 provider 未設定でも abstraction / readiness を検証できる。

### 2.8 client explainability / reporting / appeal / audit
- **client explainability（trust-semantics §4）**: client が moderation label / risk signal を表示する際、**issuer node / target / category / severity / basis / confidence / expiresAt / visibility / subscription state** を説明できる状態にする（断定ラベルを根拠なく出さない）。「network 全体がこの user を危険と認定した」という誤認を与えない。
- 通報は中央集約しない。実際に surfacing / moderation / cache / report に関与した node の authority scope へ route する。report routing は content provenance の `observedVia`（capability = `moderation` / `trust_signal`）で issuer node を responsible node とし、manifest authority scope と突き合わせて通報先候補にする。issuer node はその signal に関する appeal / 異議の窓口候補になる（trust-semantics §5）。
- appeal は `AppealStatus`（None / Disputed / Cleared）で表現。operator は event id / reference id / basis category を使い、有害コンテンツ本体を再配布せずに exclude を説明・監査できる。

### 2.9 readiness before public indexing
public-node profile が public indexing を有効化する前に満たすべき条件（critical-safety.md §11、`cn-operator safety readiness`）:
- known-CSAM provider が設定されている
- provider credential が readiness check で検証される
- `index_before_scan = false`
- scan error が hold / fail-closed に route される
- signed moderation event が有効
- permanent blob storage が無効
- scan coverage metrics が利用可能
- `Moderation` / `CommunityIndex` / `CommunityLocalTrust` の `Availability::Planned` 昇格は readiness と fail-closed indexing（#404）が揃った段階で別途判断。

### 2.10 スコープ分担
- **本 ADR（決定論, #410）**: known-hash / provider-verdict confirmed CSAM ＋ 共通枠組み（verdict / fail-closed / signed event / visibility / provider abstraction / readiness）。
- **#411（非決定論, VLM）**: `ClassifierScore` ベースの suspected CSAM / CSE、general media VLM、`Local` 既定 visibility、human-in-the-loop。本 ADR の枠組みに差し込む。
- **ADR 0025**: verdict を index surfacing gate に接続。
- **ADR 0026**: confirmed（決定論）は trust 絶対成分の入力の一つ（絶対成分 = 決定論 #410 + 厳格非決定論 #411）。

## 3. Consequences
- 決定論 moderation の decision record が番号付き ADR として確定し、「architecture doc を ADR と誤認」する混乱が解消される。
- 既存実装（`cn-safety` / `cn-safety-runtime` / `cn-core` safety_events）が本 ADR の normative 対象として位置づけられる。新規実装は #404（indexing 本体）・#406（runtime 結線）・#411（非決定論）が担う。
- confirmed CSAM の risk signal が ADR 0026 の trust 絶対成分へ流れる経路が明確化される（#406）。

## 4. Out of scope（後続 / 別 ADR・Issue）
- classifier ベース非決定論的検知の詳細（#411）。
- 本番 provider 統合の実装詳細（#391）。
- fail-closed indexing 本体の DB 実装（#404）。
- runtime 結線・trust/relation 反映（#406）。

## 5. 維持する境界
- moderation は node の authority scope 内の advisory。network-wide command でも canonical でもない。
- user identity / profile / social graph の canonical を所有・改変しない。
- `allow` 以外・unscanned・scan_failed・provider_unavailable を surfacing に出さない（fail-closed、単一判定点 `is_indexable()`）。
- `NoKnownMatch` を safe の証明にしない。router の confirmed CSAM basis は known-hash（`KnownHashMatch`）。classifier suspected を confirmed に昇格させない。
- no permanent blob storage。有害コンテンツを証拠として再配布しない。
- suspected / classifier は confirmed に昇格させず、visibility は `Local` 既定。

## 6. 資料の整理（fold / reference）
- 本 ADR を決定論 moderation の **decision record（authoritative）** とする。
- `docs/safety/community-node-critical-safety.md` は provider outreach 向けの詳細 architecture narrative として **supporting reference** に残す（削除しない。code / operator docs から参照されている）。
- `docs/architecture/moderation-event-trust-semantics.md` の moderation 原則（advisory ≠ command / authority scope / visibility / default node）は本 ADR に折り込んだ。trust/relation reflection 部分は ADR 0026 が担う。今後、原則の decision は本 ADR、詳細解説は architecture doc、という役割分担にする。
