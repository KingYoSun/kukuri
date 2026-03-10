# Community Nodes cn-user-api 契約テスト追加
日付: 2026年02月04日

## 概要
- cn-user-api の /v1/labels /v1/trust/* /v1/trending 成功系契約テストを追加し、seed/fixture を整備。

## 対応内容
- `kukuri-community-node/crates/cn-user-api/src/subscriptions.rs` に成功系契約テストを追加。
- labels/trust/trending 用の seed（labels、trust score/attestation、relay events、topic subscription、consents）をテストセットアップで用意。
- テスト実行時にマイグレーションとデフォルトプランを初期化。

## 検証
- `docker compose -f docker-compose.test.yml up -d community-node-postgres`
- `docker run --rm --network kukuri_community-node-network -e DATABASE_URL=postgres://cn:cn_password@community-node-postgres:5432/cn -v C:\Users\kgm11\kukuri:/app -w /app/kukuri-community-node rust:1.88-bookworm bash -c "cargo test -p cn-user-api --tests -- --nocapture"`
- `gh act --workflows .github/workflows/test.yml --job format-check`（git clone の some refs were not updated / pnpm approve-builds 警告）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（git clone の some refs were not updated / pnpm approve-builds 警告 / React act・useRouter 警告 / ENABLE_P2P_INTEGRATION 未設定による skip / performance tests ignored）
