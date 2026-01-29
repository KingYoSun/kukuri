# Community Node search/index E2E
日付: 2026年01月29日

## 概要
- search/index の実データを投入し、検索UI（サジェスト/ページング/0件）と community node search API の連携を実ノードE2Eで確認した。

## 対応内容
- `cn-cli` の seed を拡充し、search/index の実データ（投稿/トピック）を追加。
- Community Node search API の型追加と検索結果正規化を反映し、ページング挙動を整理。
- `community-node.search.spec.ts` を更新し、サジェスト/ページング/0件の UI 挙動を実ノードで検証。
- 設定→検索遷移時のトピック再選択を追加し、E2E の安定性を改善。

## 検証
- `./scripts/test-docker.ps1 e2e-community-node`
- `gh act --workflows .github/workflows/test.yml --job format-check`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`

## 補足
- Desktop E2E artefacts は `tmp/logs/desktop-e2e/` と `test-results/desktop-e2e/` に出力。
