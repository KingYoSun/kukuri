# Dockerスモークテスト縮小タスクメモ

最終更新: 2025年10月17日

## 背景
- 現状の `test-runner` サービスは Rust P2P 統合テストと TypeScript 統合テスト（`pnpm test:integration`）を同一コンテナで実行しており、起動コストが高い。
- 契約テストのみを確認したいケースでも `docker compose up p2p-bootstrap test-runner` を実行する必要があり、待機時間とリソース消費が過大。
- `scripts/test-docker.ps1` に `contracts` オプションを追加したが、`rust-test` / `ts-test` サービスをそのまま呼び出しており、ベースイメージのキャッシュ前提でない環境では依然として重い。

## タスク候補
1. **軽量テストランナーの作成**  
   - `docker-compose.test.yml` とは別に契約テスト専用の Compose ファイル（例: `docker-compose.contracts.yml`）を用意し、Rust/TS いずれも最小限のソースと依存だけをマウントする。
   - CI では `contracts` ワークフローをこの軽量構成に切り替えて、スモーク実行時間の短縮を図る。
2. **ベースイメージの最適化**  
   - `Dockerfile.test` を共通ベースとしつつ、契約テスト用にはビルド済みの `cargo` / `pnpm` キャッシュを事前に取り込んだステージを作成する（Rust は `cargo test --test nip10_contract_tests`、TS は `pnpm vitest run ...` のみに絞る）。
   - ボリュームマウントを最小限にし、Docker Desktop 環境での I/O オーバーヘッドを削減。
3. **スクリプト連携の整理**  
   - `scripts/test-docker.ps1` / `scripts/docker/run-smoke-tests.sh` を更新し、契約テスト実行時は新構成を利用するように分岐。
   - `docs/03_implementation/p2p_dht_test_strategy.md` に軽量フローを追記し、開発者向けに運用手順を明文化。

## メモ
- 新構成移行後は `ENABLE_P2P_INTEGRATION` やブートストラップ依存を前提としないため、`p2p-bootstrap` サービスの起動を省略できる想定。
- Rust/TypeScript いずれも契約テストはローカルモジュールのみで完結するため、ネットワークアクセス不要となり CI/CD での安定性向上が期待できる。
