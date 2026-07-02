# 2026-07-02 — #391 Project Arachnid Shield 統合

参照: [Issue #391](https://github.com/KingYoSun/kukuri/issues/391) /
ADR 0027 §7（追補: 写像表・非保持保証・operator-owned credentials）/
`docs/runbooks/project-arachnid-shield.md`（operator 手順）/
`.claude/plans/2026-07-02-issue-391-project-arachnid-shield.md`（実装プラン）

## 実装範囲

- **`crates/cn-safety-arachnid`（新規）**: `ShieldClient`（HTTP Basic auth、`POST /v1/media` /
  `POST /v1/pdq`）、`ProjectArachnidShieldProvider`（`SafetyProvider` 実装、名称
  `project-arachnid-shield`、capability `KnownCsamHashMatch` + `PerceptualHashMatch`）、
  `ShieldProviderConfig`（env: `PROJECT_ARACHNID_API_USERNAME` / `PROJECT_ARACHNID_API_PASSWORD`、
  base URL / timeout 上書き可）。
- **domain 拡張（`cn-safety`）**: `SafetyCategory::ProviderTest` / `ReasonCode::ProviderTestMatch`
  （Shield `test` classification の専用扱い。exclude するが `csam_confirmed` と区別）、router 規則3、
  `MediaFetcher` / `FetchedMedia`（media 一時 fetch の seam。本 issue では未接続）。
- **runtime 接続（`cn-core` / `cn-indexer`）**: `resolve_provider` に `project-arachnid-shield`
  （feature `safety-arachnid-provider`、known_csam slot 専用、underscore 表記は正規化）。
  credentials 欠落 = 構築 Err = ingest 起動せず（fail-closed）。
- **operator CLI / readiness（`cn-operator`）**: `safety test-provider project-arachnid-shield`
  （実 API への connectivity check。合成 PDQ hash 使用、credentials 値は非表示。feature
  `safety-arachnid`）、readiness check `known_csam_provider_resolvable` を追加（13 項目に）。
- **テスト**: wiremock contract テスト 15 件（exact / near / harmful-abusive / test /
  no-known-match の写像、401/5xx/timeout/未知 classification の fail-closed、credentials・
  Match Data の非露出、orchestrator 経由の moderation event 非汚染）。
- **docs**: runbook（operator の credentials / Submitted Data / No Known Match semantics /
  法的義務 disclaimer / 漏洩時手順 / Authorized Domains）、ADR 0027 §7 追補、
  `.env.community-node.example`。

## 実 API での確認（2026-07-02）

- 認証は HTTP Basic（`Www-Authenticate: Basic` を実測）。
- `POST /v1/pdq` の hash は **32 bytes の base64**（hex は 400 `invalid pdq hash`）。
- `safety test-provider project-arachnid-shield` が実 credentials で
  `connectivity: OK` / `probe_classification: NoKnownMatch` を返すことを確認。

## 維持した境界 / 意図的にやらないこと

- media scan pipeline（blob の一時 fetch → media_hint 付き scan 発火）は未接続。media 参照付き
  post は従来どおり `has_unscanned_media()` で fail-closed（index されない）。pipeline 実装時に
  `MediaFetcher` を `with_media_fetcher` で注入する。
- PDQ ローカル計算による hash submission・`POST /v1/url`（要 Authorized Domains）・
  `POST /v1/media/submit`（analyst 提出）は follow-up。
- Match Data（`near_match_details` / sha 群）は応答型に持たず、保存・P2P 配布・AI 入力の経路が
  構造的に存在しない。
- credentials は operator-owned。kukuri は同梱・共有・代理利用しない。値は log / error /
  Debug に出さない。
