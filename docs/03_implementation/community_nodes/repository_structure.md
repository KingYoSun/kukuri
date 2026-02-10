# kukuri-community-node リポジトリ構成案

**作成日**: 2026年01月22日

本プロジェクトは `./kukuri-community-node` 配下に実装する（本体リポジトリ `kukuri/` のサブディレクトリとして追加）。

## 目標

- サービスごとに分離・単独起動できる（Docker Compose profiles で切替）
- 共通の Postgres を使い、設定・監査・ジョブ状態を一元管理できる
- 旧CLIを統合し、`bootstrap` / `relay` をサービスとして起動可能にする
- 管理画面（Web）と管理 API（control plane）を持つ
- 外部公開する HTTP API は User API に集約し、認証/課金/購読/レート制限を統一できる

## ディレクトリ構造（提案）

```
kukuri-community-node/
  README.md
  docker-compose.yml
  .env.example
  .sqlx/                           # sqlx offline schema（`query!` を使う場合。コミットする）
  apps/
    admin-console/                 # React + TS + Vite（shadcn/ui）
  crates/
    cn-admin-api/                  # 管理API（control plane）
    cn-user-api/                   # ユーザーAPI（外部I/F統合）
    cn-core/                       # DB/設定/監査/共通型/署名鍵I/F
    cn-kip-types/                  # KIP-0001の型/validate/serialize（必要最小から）
    cn-bootstrap/                  # bootstrap（daemon）
    cn-relay/                      # relay（daemon）
    cn-index/                      # index（daemon: Meilisearch同期）
    cn-moderation/                 # moderation（daemon）
    cn-trust/                      # trust（daemon: AGE計算）
    cn-cli/                        # 管理CLI（旧CLIの統合先）
  migrations/                      # Postgres用（sqlx）
  docker/
    postgres-age/                  # Apache AGE入りPostgresイメージ（Dockerfile）
```

## Rust の構成方針

- **単一バイナリ方式**（推奨）
  - `cn-cli`（もしくは `cn`）を 1 つのバイナリとして実装し、サブコマンドでサービスを切替
  - Compose では `command: ["cn", "relay"]` / `["cn", "user-api"]` のように起動
  - メリット: ビルド成果物/依存関係の集約、設定読み込み共通化、共通ロジックの重複回避
- **サービス別バイナリ方式**
  - サービスごとに独立したバイナリを用意
  - メリット: イメージ最適化/責務が明確
  - デメリット: ビルド/デプロイが増える

いずれの場合も、共通部は `cn-core` に寄せる。

## 管理画面（admin-console）

- `kukuri-tauri` と同様に `pnpm` を想定（依存管理/作法の統一）
- `admin-console` は `Admin API` の OpenAPI（`utoipa` で生成）に沿って API クライアントを生成/実装する
  - 実装スタックの決定: `docs/03_implementation/community_nodes/api_server_stack.md`

## `cn-cli` 統合（要件）

詳細は `docs/03_implementation/community_nodes/kukuri_cli_migration.md` を参照。

## テスト方針（補完）

- Rust: unit + integration（DBは compose の Postgres を使うか、Testcontainers を検討）
- Admin UI: Vitest + Testing Library（既存の kukuri-tauri と同系統）
- サービス境界: `Admin API` の契約テスト（OpenAPI/スキーマ）
