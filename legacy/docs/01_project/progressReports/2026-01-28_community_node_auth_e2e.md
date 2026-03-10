# Community Node 実ノード認証フローE2E
日付: 2026年01月28日

## 概要
- 実ノード challenge/verify を通す認証ヘルパーを追加し、community-node 設定/認証/同意取得の E2E を実ノードで再実行した。

## 対応内容
- E2E ブリッジに `communityNodeAuthFlow` を追加し、設定/認証/同意取得のフローを一括で呼び出せるようにした。
- E2E 側に `runCommunityNodeAuthFlow` ヘルパーを追加し、`community-node.settings.spec.ts` で利用するように変更した。

## 検証
- `./scripts/test-docker.ps1 e2e-community-node`
- `gh act --workflows .github/workflows/test.yml --job format-check`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`
