# Community Node 招待/鍵同期/暗号化投稿E2E
日付: 2026年01月28日

## 概要
- 実ノードで invite.capability を発行し、招待適用→鍵同期→invite scope 暗号化投稿までを通す E2E を追加・安定化した。

## 対応内容
- `cn-cli` に invite.capability 発行ヘルパーを追加し、招待 JSON を生成できるようにした。
- Docker の E2E スクリプトで招待発行を組み込み、E2E 環境変数へ渡すように更新した。
- `community-node.invite.spec.ts` を追加/更新し、招待適用→鍵同期→暗号化投稿の検証を行うようにした。
- PostCard に scope/暗号化バッジの判定用属性を追加し、E2E での検証を安定化した。

## 検証
- `./scripts/test-docker.ps1 e2e-community-node`

## 補足
- 本E2Eは実ノード/CLIヘルパーを使った legacy。v1 は P2P-only のため、`access_control_issue_invite`/`access_control_request_join` を使った E2E へ移行する。
