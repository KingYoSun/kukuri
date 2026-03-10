# Community Node label/attestation/trust E2E
日付: 2026年01月29日

## 概要
- 実ノードで label/attestation/trust を発行し、PostCard のラベル/信頼バッジ表示までを検証する E2E を追加した。

## 対応内容
- `cn-cli` の seed 結果に label/trust スコアを含め、E2E 用 JSON を出力するよう更新。
- Docker E2E スクリプトで seed JSON を取得し、E2E 環境変数へ渡す流れを追加。
- `community-node.labels-trust.spec.ts` を追加し、seed 投稿のラベル/信頼バッジ表示を検証。
- PostCard に label/trust バッジの `data-label` / `data-score` 属性を付与し、E2E での判定を安定化。

## 検証
- `./scripts/test-docker.ps1 e2e-community-node`
- `gh act --workflows .github/workflows/test.yml --job format-check`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`

## 補足
- Seed JSON は `tmp/logs/community-node-e2e/seed.json` に保存。
