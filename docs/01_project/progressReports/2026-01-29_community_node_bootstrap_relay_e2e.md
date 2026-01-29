# Community Node bootstrap/relay E2E
日付: 2026年01月29日

## 概要
- bootstrap/relay 実ノードの list_bootstrap_nodes/services を E2E で検証するテストを追加。
- 実ノード seed と Docker テストシナリオに bootstrap サービスを追加し、検証経路を確保。

## 実施内容
- E2E ブリッジに communityNodeListBootstrapNodes/services を追加して list_bootstrap_* を呼び出し。
- `community-node.bootstrap.spec.ts` を追加し、39000/39001 イベントの content を検証。
- community-node CLI seed に bootstrap/relay を topic_services へ upsert、cleanup を追加。
- docker compose とテストスクリプトに community-node-bootstrap サービス起動を追加。

## 確認
- `./scripts/test-docker.ps1 e2e-community-node`
- `gh act --workflows .github/workflows/test.yml --job format-check`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`

## 補足
- E2E のログ/レポートは `tmp/logs/desktop-e2e/` と `test-results/desktop-e2e/` に出力。