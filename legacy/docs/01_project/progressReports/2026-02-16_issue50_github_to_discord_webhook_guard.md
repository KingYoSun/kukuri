# Issue #50 GitHub -> Discord webhook guard

最終更新日: 2026年02月16日

## 概要

Issue #50 の要求に基づき、`.github/workflows/github-to-discord.yml` の通知処理に webhook 未設定時ガードを追加した。
`DISCORD_WEBHOOK_URL` が空の場合は Discord POST を実行せず明示ログを残し、Webhook が存在する場合は既存の送信挙動を維持する。

## 実施内容

1. `notify` job に `DISCORD_WEBHOOK_URL: ${{ secrets.DISCORD_WEBHOOK_URL }}` を追加。
2. `Send to Discord` に `if: ${{ env.DISCORD_WEBHOOK_URL != '' }}` を追加。
3. `Skip Discord POST (webhook not configured)` ステップを追加し、未設定時のスキップ理由を出力。
4. payload 生成（`jq -n --arg content "$CONTENT" '{content: $content, flags: 4}'`）と `curl` 実行ロジックは変更なし。

## 検証

- `gh act` で `github-to-discord.yml` を検証。
  - no-webhook: `Skip Discord POST` が実行され job 成功。
  - with-webhook: `Send to Discord` が実行され job 成功。
- AGENTS 必須の `test.yml` 3 ジョブを実行。
  - `format-check`: pass
  - `native-test-linux`: pass
  - `community-node-tests`: pass

ログ:
- `tmp/logs/gh-act-github-to-discord-no-webhook.log`
- `tmp/logs/gh-act-github-to-discord-with-webhook.log`
- `tmp/logs/gh-act-format-check-issue50.log`
- `tmp/logs/gh-act-native-test-linux-issue50.log`
- `tmp/logs/gh-act-community-node-tests-issue50.log`

## 影響範囲

- 変更対象は `.github/workflows/github-to-discord.yml` の通知 step 条件分岐のみ。
- Webhook 未設定時の誤失敗を解消し、Webhook 設定時の動作互換を維持。
