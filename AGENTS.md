# Repository Guidelines

## プロジェクト構成
- `kukuri-tauri/`: デスクトップアプリ（React + TypeScript + Tauri）
  - `src/` UI・hooks・stores・`__tests__/`
  - `src-tauri/` Rust・`migrations/`・`tauri.conf.json`・`.sqlx/`
- `kukuri-cli/`: DHTブートストラップ/リレー用Rust CLI
- `pkarr/`: Pkarrリレー（サブモジュール）。設定は`config.toml`
- `docs/`: 設計・実装ガイド、API JSONは`docs/apis/`
- `scripts/`: セットアップ・テスト・起動用スクリプト

## ビルド・テスト・開発
- アプリ開発: `cd kukuri-tauri && pnpm tauri dev`
- アプリビルド: `pnpm tauri build`（Windowsクロス: `--runner cargo-xwin`）
- TypeScript: `pnpm test` / `pnpm test:coverage` / `pnpm lint` / `pnpm format`
- Rust(kukuri-tauri): `cd kukuri-tauri/src-tauri && cargo test && cargo clippy -D warnings`
- Rust(kukuri-cli): `cd kukuri-cli && cargo test && cargo build --release`
- Dockerテスト: `docker compose -f docker-compose.test.yml up --build test-runner`
- Windows推奨: `./scripts/test-docker.ps1 all|rust|ts|lint`
- Pkarrローカル: `docker compose up -d`（`8080`/`6881`）

## コーディング規約
- EditorConfig: LF、TS/JS 2スペース、Rust/TOML 4スペース
- Prettier: シングルクォート・セミコロン・幅100（`pnpm format`）
- ESLint: TSルール厳格（未使用は`_`接頭で許容）
- Rust: `rustfmt`/`clippy`必須、`anyhow`/`thiserror`で一貫したエラー処理
- React: コンポーネントはPascalCase、storesはcamelCase（例: `topicStore.ts`）
- フロントの`console.error`禁止。`errorHandler`を使用（docs参照）

## テスト指針
- 単体・結合: Vitest + Testing Library（`*.test.ts(x)` または `__tests__/`）
- 統合: `kukuri-tauri/src/test/integration` は `pnpm test:integration`
- Rust: `cargo test`（`kukuri-tauri/src-tauri` と `kukuri-cli`）
- 変更時は必ずテスト追加/更新とカバレッジ確認

## コミット/PR
- Conventional Commits推奨: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`
- PRに含める: 要約、根拠/設計意図、再現/確認手順、関連Issue、UI変更はスクリーンショット
- 無関係な整形・リファクタは分離。サブモジュールは`git submodule update --init --recursive`

## エージェント運用ルール（Codex CLI）
- 作業開始前チェック: `docs/01_project/activeContext/tasks/` の `tasks/priority/critical.md` と `tasks/status/in_progress.md`、および `tasks/README.md` を確認。併せて関連ドキュメントの最終更新日を確認。
- 言語: 回答・記述は日本語で統一。
- ツール: GitHub操作は `gh`、JSONの調査/整形は `jq`。
- API参照: iroh系はローカルの `docs/apis/*.json` を優先して参照し、必要に応じて `jq` で検索。
- コミット: ユーザーから明示的に要求されない限り、絶対にコミットしない。
- DRY原則: 新規クラス/メソッド/型の実装前に既存の重複機能がないか調査。
- 依存追加: 追加時は最新安定版を確認して採用。
- フロントのエラー処理: `console.error` は禁止。`docs/03_implementation/error_handling_guidelines.md` の `errorHandler` を使用。
- Windowsテスト: DLL等の理由で不安定な場合は `./scripts/test-docker.ps1` を用いて Docker 経由で実行。
- 検証必須: テスト・型・リント修正タスクは、実際にコマンドを実行しエラーが出ないことを確認してから完了とする。
- Rustテスト実行方針: Rust（`kukuri-tauri/src-tauri` と `kukuri-cli`）の `cargo test` は所要時間が長くても必ず実行し、完了まで待つ。必要に応じてタイムアウトを延長してでもテスト結果を確認する。
- 日付記法: ドキュメント内の日付は `date "+%Y年%m月%d日"` の出力を使用。

## タスク管理ルール
- 開始時: `tasks/priority/critical.md` から対象を選び、`tasks/status/in_progress.md` に移動して着手を明示。
- 作業中: 原則 `tasks/status/in_progress.md` のみを更新（進捗/メモ）。他ファイルは必要時のみ編集。
- 完了時: `tasks/completed/YYYY-MM-DD.md` に完了内容を追記し、`in_progress.md` から削除。重要な変更は進捗レポートを作成。
- ブロッカー: 発生時は `tasks/context/blockers.md` に追記し、解決後は削除。

### 作業完了チェックリスト
- [ ] `tasks/completed/YYYY-MM-DD.md` に完了タスクを追記
- [ ] `tasks/status/in_progress.md` から当該タスクを削除
- [ ] 重要な変更について進捗レポートを作成

## Nostr互換性の確認
- NIP準拠: 実装時は `docs/nips/` の該当NIPを参照し、仕様順守を確認。
- 標準仕様: 新機能の前に関連NIPの有無を確認。
- 拡張時: 独自拡張は標準との差分を明記して文書化。

## SQLx開発環境（オフライン準備）
SQLx の `query!` を用いる場合はオフラインスキーマを準備し、`.sqlx/` ディレクトリを必ずコミットする。

```bash
cd kukuri-tauri/src-tauri
DATABASE_URL="sqlite:data/kukuri.db" sqlx database create
DATABASE_URL="sqlite:data/kukuri.db" sqlx migrate run
DATABASE_URL="sqlite:data/kukuri.db" cargo sqlx prepare
```

PowerShell では `$env:DATABASE_URL="sqlite:data/kukuri.db"` を用いる。

## ドキュメント構成/配置
- 優先参照順: `docs/SUMMARY.md` → `docs/01_project/activeContext/` → 各ディレクトリの `summary.md` → 詳細ドキュメント。
- すべてのドキュメントは `./docs/` 以下に配置（`kukuri-tauri/docs/` などのサブディレクトリは作成しない）。
- 進捗レポート: `docs/01_project/progressReports/`
- 実装ガイド: `docs/03_implementation/`
- アーキテクチャ: `docs/02_architecture/`

## 参照先（ローカルAPI JSON）
- iroh: `docs/apis/iroh_0.91.1_x86_64-unknown-linux-gnu_latest.json`
- iroh-blobs: `docs/apis/iroh-blobs_0.93.0_x86_64-unknown-linux-gnu_latest.json`
- iroh-docs: `docs/apis/iroh-docs_0.91.0_x86_64-unknown-linux-gnu_latest.json`
- iroh-gossip: `docs/apis/iroh-gossip_0.91.0_x86_64-unknown-linux-gnu_latest.json`

## プロジェクト概要/技術スタック
- フロント: React 18 + TypeScript / Vite / shadcn/ui / Zustand / Tanstack Query / Tanstack Router
- デスクトップ: Tauri v2 (Rust)
- P2P: iroh（QUIC）, 配信: iroh-gossip, プロトコル: Nostr (nostr-sdk)
- DB: SQLite (sqlx)、暗号: secp256k1, AES-256-GCM

## アーキテクチャ（レイヤー）
1. Client: Tauriアプリ（UI + ビジネスロジック）
2. Discovery: ピア発見サービス（Workers/Container）
3. P2P Network: iroh による直接通信
4. Marketplace: 検索/推薦等の専門ノード

## 追加コマンド備考
- Androidビルド: `pnpm tauri android build`
- フォーマット（Rust）: `cargo fmt`
- フォーマットチェック: `pnpm format:check`
