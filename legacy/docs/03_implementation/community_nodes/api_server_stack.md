# API サーバ実装スタック（User API / Admin API）決定（v1）

**作成日**: 2026年01月23日  
**対象**: `./kukuri-community-node`（`cn-user-api` / `cn-admin-api`）

## 結論（v1）

v1 は「運用可能なコミュニティノード」を最短で成立させるため、User API / Admin API の実装スタックを次に確定する。

- **実装言語**: Rust（User API / Admin API ともに Rust で統一）
- **Web フレームワーク**: `axum`
- **OpenAPI**: `utoipa`（Rust型から spec を生成）
- **認証**
  - Admin API: password login + **`httpOnly` セッションcookie**（推奨。ブラウザ運用の XSS 耐性を優先）
  - User API: 署名チャレンジ（nostr鍵）→ access token（JWT（HS256））
- **middleware（基本セット）**: `tower-http` をベースに統一する（request id / timeout / body size limit / trace / CORS 等）
- **structured logging**: `tracing`（JSON 出力を基本）
- **metrics**: Prometheus `/metrics` を提供し、運用指標を収集できるようにする

補足:
- rate limit（DoS/濫用対策）は v1 で確定（Redis無し / in-mem / `tower` layer）。詳細: `docs/03_implementation/community_nodes/rate_limit_design.md`

## なぜ Rust 統一（axum）なのか（v1の判断）

- サービス群（relay/index/moderation/trust）を Rust で実装する前提と整合し、運用・ビルド・共通型・監査/ログの実装が二重化しない
- `tracing` 前提のロギング/メトリクス文化と相性がよい（既存コードでも利用実績がある）
- `tower` の middleware エコシステムで、HTTPの非機能（timeout/trace/limit等）を揃えやすい

## OpenAPI の扱い（v1）

- spec は `utoipa` で生成し、少なくとも以下を提供する
  - `GET /v1/openapi.json`（Admin API / User API それぞれ）
  - Swagger UI は任意（運用上必要なら追加）
- `admin-console` は OpenAPI から TS 型/クライアントを生成して利用する
  - 例: `openapi-typescript`（pnpm script で生成し、CI で差分検知）
- サービス境界（Admin API）は OpenAPI の **契約テスト**対象とする（`docs/03_implementation/community_nodes/repository_structure.md` の方針を踏襲）

## 認証（v1の前提）

### Admin API

- 推奨: `httpOnly` セッションcookie（`admin_sessions`）
- CSRF は運用形態により選択
  - reverse proxy で同一オリジン運用ができるなら、まずは same-site cookie 前提で最小化

### User API

- 署名チャレンジ（v1採用）で pubkey を確定し、短命 access token（JWT（HS256））を発行する
  - `POST /v1/auth/challenge` → `POST /v1/auth/verify`
  - `verify` の署名イベントは **NIP-42 と同形式の kind=22242** を推奨（tag: `relay`/`challenge`）
  - NIP-98（HTTP Auth）は v2 候補（v1 は採用しない）
  - 詳細: `docs/03_implementation/community_nodes/user_api.md`

## middleware（v1の基本セット）

v1 は「入口を揃える」ことを目的に、User/Admin で共通の middleware 構成を持つ。

- `request_id`（HTTP: `X-Request-Id` を尊重し、無ければ生成）
- `timeout`（route/サービスごとに上限を設定）
- `body_size_limit`（巨大リクエスト拒否）
- `trace`（開始/終了/latency/status を `tracing` に出す。本文は出さない）
- `CORS`（基本は reverse proxy で同一オリジン化。必要時のみ最小許可）
- `compression`（必要なら）

## structured logging（v1）

- `tracing` を利用し、JSON で `stdout` へ出力する（収集は `docs/03_implementation/community_nodes/ops_runbook.md` の方針に従う）
- 原則ログに出さない: 本文、JWT、IP、生 pubkey 等（必要ならハッシュ化）

## metrics（v1）

- `/metrics`（Prometheus形式）を提供する
- 最低限のメトリクスは `docs/03_implementation/community_nodes/ops_runbook.md` の必須メトリクスを満たす

## 非採用案（v1）

v1 は “最短で運用可能にする” を優先し、次は採用しない（v2以降で再検討）。

- TypeScript（Fastify 等）で API を実装: 開発速度は高いが、Rust サービス群と分離すると共通型/監査/認証/運用が二重化しやすい
- Go で API を実装: 同上（混在コストが増える）
